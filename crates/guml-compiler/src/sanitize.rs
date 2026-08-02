//! Stripping what a model wrapped around the document, before anything tries to parse it.
//!
//! # Why this belongs in the compiler
//!
//! It existed only in `bench/gen/lib/pipeline.mjs` — a benchmark script. So the measured repair
//! pipeline had a layer the *product* did not: anyone using `guml check` on real model output hit a
//! parse error on a ``` fence that the paper's own numbers had already discounted. A repair layer that
//! only runs in the harness makes the harness optimistic about the tool, which is the one direction a
//! benchmark must not be wrong in.
//!
//! Moving it here means the CLI, the LSP, the wasm build and the npm package all get it, and it is
//! covered by the same test suite as the parser.
//!
//! # What is packaging and what is document
//!
//! Three things get removed, in increasing order of how careful the rule has to be.
//!
//! 1. **A code fence.** ```` ```guml ```` … ```` ``` ````. Unambiguous: no GUML line begins with a
//!    backtick.
//! 2. **A markdown horizontal rule.** `---`, `***`, `___` on their own line. Also unambiguous, since a
//!    tag name cannot start with `-`, `*` or `_`.
//! 3. **Trailing commentary.** This is the one that needs the compiler. A sentence like
//!    "This page uses the tabs control to filter." is indistinguishable from GUML *by pattern* — `This`
//!    is a plausible tag name — so the rule is not lexical. It is: while the last content line is the
//!    subject of an error *and* [`is_commentary`] agrees it is prose rather than broken GUML, drop it.
//!    Only from the end, and bounded, so a mistake in the middle of a document is never deleted.
//!
//!    The `is_commentary` half is not a refinement, it is the whole rule. "Drop the last erroring line"
//!    alone eats a document one line per round — see that function for the four-line case it destroyed.
//!
//! # What is deliberately not removed
//!
//! Leading commentary. Dropping from the front would need the same "is this line an error" test, and at
//! the front an error is far more likely to be a *real* mistake in the first tag than a preamble —
//! deleting it would throw away the document's `page` directive and turn one fixable error into a
//! cascade. A model that puts prose first is better served by the diagnostic.

use guml_diagnostics::Severity;

/// What [`sanitize`] removed. Reported rather than returned silently: "the repair loop works" is not a
/// claim worth making, and "layer 0 handled 2 of 6 generations, and here is what it did" is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stripped {
    /// A fenced code block was unwrapped.
    pub fence: bool,
    /// Markdown horizontal rules removed.
    pub rules: usize,
    /// Trailing commentary lines dropped from the end.
    pub trailing: usize,
}

impl Stripped {
    pub fn is_empty(&self) -> bool {
        !self.fence && self.rules == 0 && self.trailing == 0
    }

    /// One line describing what happened, for a CLI or a telemetry record.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.fence {
            parts.push("unwrapped a code fence".to_string());
        }
        if self.rules > 0 {
            parts.push(format!("removed {} markdown rule(s)", self.rules));
        }
        if self.trailing > 0 {
            parts.push(format!("dropped {} trailing commentary line(s)", self.trailing));
        }
        if parts.is_empty() { "nothing to strip".to_string() } else { parts.join(", ") }
    }
}

/// Maximum trailing lines to drop.
///
/// A bound rather than "until it parses": an unbounded loop on a document that is *entirely* prose would
/// delete all of it and report success on an empty file. Twelve is comfortably more than the two or three
/// sentences a model appends, and far less than a document.
const MAX_TRAILING: usize = 12;

pub fn sanitize(src: &str) -> (String, Stripped) {
    let mut notes = Stripped::default();

    let mut text = match unfence(src) {
        Some(inner) => {
            notes.fence = true;
            inner
        }
        None => src.to_string(),
    };

    let kept: Vec<&str> = text
        .lines()
        .filter(|line| {
            if is_horizontal_rule(line) {
                notes.rules += 1;
                false
            } else {
                true
            }
        })
        .collect();
    text = kept.join("\n").trim().to_string();

    // Trailing commentary, decided by the compiler rather than by a regex for prose.
    for _ in 0..MAX_TRAILING {
        let lines: Vec<&str> = text.lines().collect();
        let Some(last) = lines.iter().rposition(|l| !l.trim().is_empty()) else { break };
        // Never delete the only content line: a one-line document that fails to parse is a document to
        // report on, not one to empty out.
        if last == 0 {
            break;
        }

        let (_, diags) = crate::check(&text);
        let errors: Vec<_> = diags.items.iter().filter(|d| d.severity == Severity::Error).collect();
        if errors.is_empty() {
            break;
        }
        // `span.line` is 1-based.
        let on_last: Vec<_> = errors.iter().filter(|d| d.span.line as usize == last + 1).collect();
        if on_last.is_empty() {
            break;
        }
        if !is_commentary(lines[last], &on_last) {
            break;
        }

        let mut remaining: Vec<&str> = lines.clone();
        remaining.remove(last);
        text = remaining.join("\n").trim().to_string();
        notes.trailing += 1;
    }

    if text.is_empty() { (String::new(), notes) } else { (format!("{text}\n"), notes) }
}

/// Whether a trailing line is *commentary* rather than GUML that merely fails to compile.
///
/// # The bug this rule exists for
///
/// The first version dropped any last line that was the subject of an error. Given
/// `page P / div / span Hello / button Save / hr` it deleted `hr`, then `button Save`, then
/// `span Hello`, then `div` — every line, one per round, each being "the last erroring line" in turn.
/// A document with four fixable HTML habits was reduced to `page P`, and reported as sanitised.
/// The 12-line bound did not help; the bound limits the damage, it does not detect it.
///
/// So being an error is not enough. Two further conditions, both of which prose satisfies and broken
/// GUML does not:
///
/// 1. **The compiler has no suggestion for it.** `div` carries `did you mean col?`, so it is a rename
///    away from being correct — that is a repair, not a deletion. `This page greets the reader.` gets no
///    suggestion, because `This` is not close to any tag.
/// 2. **Its first word is not a tag.** `\tp Tabs for indentation.` has a tab error and no suggestion, but
///    its first word is `p` — real content whose *indentation* is wrong, and the formatter's job. Without
///    this condition the sanitiser deleted the line the formatter was about to fix.
fn is_commentary(line: &str, errors: &[&&guml_diagnostics::Diagnostic]) -> bool {
    // A repairable error is a repair, not commentary.
    if errors.iter().any(|d| d.suggestion.is_some()) {
        return false;
    }
    let Some(first) = line.split_whitespace().next() else { return false };
    let reg = guml_registry::Registry::builtin();
    // A real tag, or something one edit from one, is a document line.
    if reg.get(first).is_some() || reg.suggest(first).is_some() {
        return false;
    }
    // Directives are not in the tag registry, and a mis-indented `state` is content too.
    const DIRECTIVES: &[&str] =
        &["page", "type", "data", "state", "store", "on", "route", "auth", "def", "js", "raw"];
    !DIRECTIVES.contains(&first)
}

/// The contents of the first fenced block, if the text contains one.
///
/// Hand-written rather than a regex so the crate keeps its dependency list, and because the rule is
/// simple: find a line that is a fence, take everything up to the next fence line or the end. An
/// unterminated fence is treated as running to the end, which is what a truncated generation looks like.
fn unfence(src: &str) -> Option<String> {
    let lines: Vec<&str> = src.lines().collect();
    let open = lines.iter().position(|l| l.trim_start().starts_with("```"))?;
    let rest = &lines[open + 1..];
    let close = rest.iter().position(|l| l.trim_start().starts_with("```")).unwrap_or(rest.len());
    Some(rest[..close].join("\n").trim().to_string())
}

/// `---`, `***`, `___` alone on a line. A GUML tag cannot start with any of these characters, so there
/// is nothing to disambiguate.
fn is_horizontal_rule(line: &str) -> bool {
    let t = line.trim();
    if t.len() < 3 {
        return false;
    }
    let first = t.chars().next().unwrap();
    matches!(first, '-' | '*' | '_') && t.chars().all(|c| c == first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(src: &str) -> String {
        sanitize(src).0
    }

    #[test]
    fn a_fenced_block_is_unwrapped() {
        let (text, notes) = sanitize("```guml\npage P\np Hello.\n```\n");
        assert_eq!(text, "page P\np Hello.\n");
        assert!(notes.fence);
    }

    #[test]
    fn a_fence_with_prose_around_it_keeps_only_the_block() {
        // The shape a model actually produces when it ignores "no prose before or after".
        let src = "Here is the page you asked for:\n\n```guml\npage P\np Hello.\n```\n\nLet me know if you want changes.\n";
        let (text, notes) = sanitize(src);
        assert_eq!(text, "page P\np Hello.\n");
        assert!(notes.fence);
    }

    #[test]
    fn an_unterminated_fence_runs_to_the_end() {
        // What a truncated generation looks like. Treating it as an error would throw away a document
        // that is otherwise complete.
        let (text, _) = sanitize("```\npage P\np Hello.\n");
        assert_eq!(text, "page P\np Hello.\n");
    }

    #[test]
    fn markdown_rules_are_removed() {
        let (text, notes) = sanitize("page P\n---\np Hello.\n***\n");
        assert_eq!(text, "page P\np Hello.\n");
        assert_eq!(notes.rules, 2);
    }

    #[test]
    fn trailing_commentary_is_dropped() {
        // Decided by the compiler: `This` is a plausible tag name, so no pattern can tell this line
        // from GUML. The compiler knows it is not.
        let (text, notes) = sanitize("page P\np Hello.\nThis page greets the reader.\n");
        assert_eq!(text, "page P\np Hello.\n");
        assert_eq!(notes.trailing, 1);
    }

    #[test]
    fn several_trailing_sentences_are_dropped() {
        let src = "page P\np Hello.\n\nThis page greets the reader.\nIt uses one paragraph.\n";
        assert_eq!(clean(src), "page P\np Hello.\n");
    }

    #[test]
    fn a_valid_document_is_left_alone() {
        // The property that matters most: this layer runs on every document, so it must be a no-op on a
        // correct one. A sanitiser that edits valid input is worse than no sanitiser.
        let src = "page P\nstate count=0\n\ncard sm center\n  h Clicks\n  metric {count}\n";
        let (text, notes) = sanitize(src);
        assert_eq!(text, src);
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn an_error_in_the_middle_is_never_deleted() {
        // Only the *end* is trimmed, and only while the last line is the subject of an error. A mistake
        // in the middle is a diagnostic to report, not content to silently remove.
        let src = "page P\nnosuchtag Hello.\np Real content.\n";
        let (text, notes) = sanitize(src);
        assert!(text.contains("nosuchtag"), "the error line was deleted: {text:?}");
        assert_eq!(notes.trailing, 0);
    }

    #[test]
    fn a_repairable_line_at_the_end_is_not_mistaken_for_commentary() {
        // The bug that made `is_commentary` necessary. Dropping "the last line that has an error" ate this
        // document one line per round — `hr`, then `button Save`, then `span Hello`, then `div` — leaving
        // `page P` and calling it sanitised. Four fixable HTML habits deleted instead of renamed.
        let src = "page P\ndiv\n  span Hello\n  button Save\n  hr\n";
        let (text, notes) = sanitize(src);
        assert_eq!(notes.trailing, 0, "repairable GUML was dropped as commentary");
        for want in ["div", "span Hello", "button Save", "hr"] {
            assert!(text.contains(want), "`{want}` was deleted:\n{text}");
        }
    }

    #[test]
    fn a_mis_indented_content_line_is_not_dropped() {
        // A tab is an error with no suggestion, so the "no suggestion" rule alone would delete this line —
        // and it is real content whose whitespace is the formatter's job, not the sanitiser's.
        let src = "page P\ncard A\n\tp Tabs for indentation.\n";
        let (text, notes) = sanitize(src);
        assert_eq!(notes.trailing, 0);
        assert!(text.contains("Tabs for indentation."), "{text}");
    }

    #[test]
    fn a_document_is_never_emptied() {
        // An unbounded "drop until it parses" would delete everything here and report success on an
        // empty file, which is the worst possible outcome: a silent pass on nothing.
        let (text, _) = sanitize("This is not GUML at all.\n");
        assert!(!text.trim().is_empty(), "the whole document was deleted");
    }

    #[test]
    fn prose_that_happens_to_end_the_document_survives_if_it_parses() {
        // `p …` lines at the end are content. Nothing is dropped unless the compiler objects to it.
        let src = "page P\np This page greets the reader.\n";
        assert_eq!(clean(src), src);
    }
}
