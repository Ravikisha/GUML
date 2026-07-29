//! GUML formatter and canonicaliser.
//!
//! # Why this works on the line stream, not the AST
//!
//! The obvious formatter is `parse → print`. It is wrong here for two reasons:
//!
//! 1. **The lexer discards comments** (`guml-syntax`, blank lines and `//` never affect
//!    layout), so an AST round-trip would silently delete every note the author wrote.
//! 2. **The main caller formats invalid input.** The repair loop's whole job is broken
//!    documents; a formatter that requires a clean parse is useless exactly when it is
//!    needed. A tab-indented or three-space-indented generation is a whitespace problem
//!    that should never cost an LLM round.
//!
//! So this crate consumes [`guml_syntax::lex`] output plus the raw source, and rewrites
//! line by line. It sits *below* the parser in the dependency graph, which also keeps it
//! out of the `guml-codegen`/`guml-parser` cycle (invariant 7).
//!
//! # Two modes, one engine
//!
//! - Default: opinionated but faithful. Fixes indentation, spacing and alignment; keeps
//!   comments and single blank lines.
//! - [`Options::canonical`]: strips every discretionary byte, so two documents with the
//!   same meaning have the same bytes. That is what makes dedup, caching and inter-run
//!   consistency measurement possible — a benchmark cannot compare generations that differ
//!   only in blank lines.
//!
//! # The invariant
//!
//! Formatting never changes meaning. `ast(fmt(x)) == ast(x)` for every input that parses,
//! and that is a test (`tests/preserves.rs` in `guml-compiler`, which is where the parser
//! is reachable from).

//! # Why the highlighter lives here too
//!
//! [`highlight`] needs exactly what the formatter needs and nothing more: the lexer, the
//! registry, and the nesting analysis in `layout` that mirrors the parser's indent rule.
//! Splitting it into its own crate would mean a second copy of that rule, and two copies of
//! a rule that must match the parser is the drift this crate exists to remove.

pub mod highlight;
mod layout;
mod print;
mod trivia;

use guml_diagnostics::Diagnostics;
use guml_registry::Registry;
use guml_syntax::lex;

pub const INDENT: usize = 2;

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Strip comments and blank lines, hoist and sort directives, prefer the shortest
    /// spelling of every value. Semantically identical documents become byte-identical.
    pub canonical: bool,
}

impl Options {
    pub fn canonical() -> Self {
        Self { canonical: true }
    }
}

#[derive(Debug)]
pub struct Formatted {
    pub text: String,
    /// Whether formatting changed anything — drives `guml fmt --check` and the editor's
    /// "no edit needed" path.
    pub changed: bool,
    /// What the lexer saw. Lines that produced a lexical error are passed through
    /// unchanged rather than guessed at, so this is also the list of places the formatter
    /// deliberately did not touch.
    pub diagnostics: Diagnostics,
}

pub fn format(src: &str, opts: Options) -> Formatted {
    let registry = Registry::builtin();
    let lexed = lex(src);

    // A line whose tokens are untrustworthy is reprinted from its raw text: re-emitting
    // tokens from a failed lex would turn a small syntax error into a mangled line, and
    // the author would lose work.
    let broken: Vec<u32> = lexed.diagnostics.items.iter().map(|d| d.span.line).collect();

    let doc = trivia::split(src, &lexed);
    let infos = layout::analyse(&lexed.lines, &registry);
    let text = print::render(&doc, &lexed, &infos, &broken, &registry, opts);

    Formatted { changed: text != src, text, diagnostics: lexed.diagnostics }
}

/// Convenience for the common case.
pub fn format_str(src: &str) -> String {
    format(src, Options::default()).text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(src: &str) -> String {
        format_str(src)
    }
    fn c(src: &str) -> String {
        format(src, Options::canonical()).text
    }

    #[test]
    fn fixes_indentation_to_two_spaces_per_level() {
        let src = "page X\ncard Hi\n    p One\n        p Two\n";
        assert_eq!(f(src), "page X\n\ncard Hi\n  p One\n    p Two\n");
    }

    #[test]
    fn tabs_become_spaces() {
        // A tab is `GUML0001`. Fixing it here is the point: it is a whitespace mistake
        // that should never cost the repair loop a model call.
        let src = "page X\ncard Hi\n\tp One\n";
        assert_eq!(f(src), "page X\n\ncard Hi\n  p One\n");
    }

    #[test]
    fn prose_is_never_reflowed_or_respaced() {
        // Text tags take the line remainder verbatim. Collapsing runs of spaces inside
        // prose would silently edit content.
        let src = "page X\np Two  spaces   inside\n";
        assert!(f(src).contains("p Two  spaces   inside"));
    }

    #[test]
    fn structured_lines_collapse_to_single_spaces() {
        let src = "page X\nbtn    Add     primary\n";
        assert_eq!(f(src), "page X\n\nbtn Add primary\n");
    }

    #[test]
    fn attributes_lose_their_spacing() {
        let src = "page X\ninput draft placeholder = \"Add a task…\"\n";
        assert!(f(src).contains("input draft placeholder=\"Add a task…\""));
    }

    #[test]
    fn an_action_stays_last_and_is_trimmed() {
        let src = "page X\nbtn Add primary >  count++  \n";
        assert!(f(src).contains("btn Add primary >count++\n"));
    }

    #[test]
    fn enum_domains_keep_tight_pipes_but_content_pipes_breathe() {
        let src = "page X\nstate filter = all | open | done\ncard \"A\"|Some prose\n";
        let out = f(src);
        assert!(out.contains("state filter=all|open|done"), "{out}");
        assert!(out.contains("card \"A\" | Some prose"), "{out}");
    }

    #[test]
    fn type_bodies_are_normalised() {
        let src = "page X\ntype Task {id,title , done:bool}\n";
        assert!(f(src).contains("type Task {id, title, done:bool}"));
    }

    #[test]
    fn comments_survive_and_follow_their_block() {
        let src = "page X\ncard Hi\n// about the paragraph\n      p One\n";
        let out = f(src);
        assert!(out.contains("  // about the paragraph\n  p One"), "{out}");
    }

    #[test]
    fn canonical_drops_comments_and_blank_lines() {
        let src = "page X\n\n// a note\n\n\ncard Hi\n\n  p One\n";
        assert_eq!(c(src), "page X\ncard Hi\n  p One\n");
    }

    #[test]
    fn canonical_hoists_and_sorts_directives() {
        let src = "page X\ncard Hi\nstate draft=\"\"\ntype Task {id}\n";
        assert_eq!(c(src), "page X\ntype Task {id}\nstate draft=\"\"\ncard Hi\n");
    }

    #[test]
    fn a_data_block_moves_with_its_mutations() {
        let src =
            "page X\ncard Hi\ndata tasks:Task[] GET /api/tasks\n  add POST /api/tasks {title}\n";
        let out = c(src);
        let data_at = out.find("data tasks").unwrap();
        let add_at = out.find("add POST").unwrap();
        let card_at = out.find("card Hi").unwrap();
        assert!(data_at < add_at && add_at < card_at, "mutations follow their resource\n{out}");
    }

    #[test]
    fn canonical_unquotes_a_label_that_needs_no_quotes() {
        let src = "page X\nbtn \"Add\" primary\n";
        assert!(c(src).contains("btn Add primary"));
        // …but not one that would become a modifier or lose a space.
        assert!(c("page X\nbtn \"primary\"\n").contains("btn \"primary\""));
        assert!(c("page X\nbtn \"Sign in\"\n").contains("btn \"Sign in\""));
    }

    #[test]
    fn default_mode_leaves_quotes_alone() {
        assert!(f("page X\nbtn \"Add\" primary\n").contains("btn \"Add\" primary"));
    }

    #[test]
    fn mutation_columns_are_aligned_in_default_mode_only() {
        let src = "page X\ndata tasks:Task[] GET /api/tasks\n  add POST /api/tasks {title} optimistic:prepend\n  drop DELETE /api/tasks/{id} optimistic\n";
        let human = f(src);
        assert!(human.contains("  add  POST   /api/tasks"), "columns line up\n{human}");
        assert!(human.contains("  drop DELETE /api/tasks/{id}"), "{human}");
        assert!(c(src).contains("  add POST /api/tasks {title} optimistic:prepend"));
    }

    #[test]
    fn blank_line_runs_collapse_to_one_and_the_file_ends_in_a_newline() {
        let src = "page X\n\n\n\ncard Hi\n\n\n";
        assert_eq!(f(src), "page X\n\ncard Hi\n");
    }

    #[test]
    fn a_blank_line_is_inserted_after_the_directive_block() {
        // Directives and the tree are different kinds of statement; the eye needs the seam.
        let src = "page X\nstate count=0\ncard Hi\n";
        assert_eq!(f(src), "page X\nstate count=0\n\ncard Hi\n");
    }

    #[test]
    fn a_line_the_lexer_could_not_read_is_passed_through() {
        // Unterminated string: reprinting from tokens would drop the rest of the line.
        let src = "page X\nbtn \"unterminated\n";
        let out = format(src, Options::default());
        assert!(out.text.contains("btn \"unterminated"), "{}", out.text);
        assert!(!out.diagnostics.is_empty());
    }

    #[test]
    fn formatting_is_idempotent() {
        for src in [
            "page X\ncard Hi\n    p One\n",
            "page X\n\n// note\nstate a=1\nstate b=2\n\ncard Hi\n  p Two  spaces\n",
            "page X\ndata t:T[] GET /u\n  add POST /u {x} optimistic:prepend\n",
        ] {
            let once = f(src);
            assert_eq!(f(&once), once, "not idempotent for {src:?}");
            let cc = c(src);
            assert_eq!(c(&cc), cc, "canonical not idempotent for {src:?}");
        }
    }

    #[test]
    fn empty_and_whitespace_only_input_do_not_panic() {
        assert_eq!(f(""), "");
        assert_eq!(f("\n\n  \n"), "");
        assert_eq!(f("// only a comment\n"), "// only a comment\n");
        assert_eq!(c("// only a comment\n"), "");
    }

    #[test]
    fn crlf_is_normalised() {
        assert_eq!(f("page X\r\ncard Hi\r\n"), "page X\n\ncard Hi\n");
    }

    #[test]
    fn unchanged_input_reports_unchanged() {
        let src = "page X\n\ncard Hi\n  p One\n";
        assert!(!format(src, Options::default()).changed);
    }
}
