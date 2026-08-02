//! Go to definition, find references, rename, and range formatting.
//!
//! Separated from `features` because these four share one idea the others do not: they are the
//! operations that *edit* or *move around* a document rather than describe it, so they are the ones
//! where being wrong costs the author work rather than a moment's confusion.
//!
//! That shapes the design in two places:
//!
//! * **Rename is verified, not trusted.** Finding occurrences is lexical — a word-boundary scan — which
//!   is the only approach that cannot miss one. An over-match is then caught by re-checking the
//!   renamed document: if it has errors the original did not, the rename is refused with the reason.
//!   An editor cannot undo an edit it has already applied across files, so declining is the only safe
//!   failure mode.
//! * **Range formatting is exact or absent.** GUML's formatter is line-local — no rule reflows across a
//!   line boundary — so formatting the document and keeping the selected lines is not an approximation,
//!   it is the same answer. When that assumption does not hold (the line count changed), it returns
//!   nothing rather than a plausible range.

use crate::features::{Position, word_at};
use guml_diagnostics::Severity;
use guml_registry::Registry;

/// A range in the document, zero-based, as the protocol uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Where the declaration under the cursor is written.
///
/// Only `state`, `store`, `data`, `type` and `def` have definitions — those are the things a document
/// names. A tag resolves against the *registry*, so "go to definition" on `card` has no answer inside
/// the file and returns nothing rather than jumping somewhere plausible.
pub fn definition(src: &str, at: Position) -> Option<Range> {
    let name = word_at(src.lines().nth(at.line as usize)?, at.character as usize)?;
    let (program, _) = guml_compiler::check(src);
    let line = declaration_line(&program, &name)?;
    let text = src.lines().nth(line as usize - 1)?;
    // The whole declaration line, so the editor highlights it rather than one character of it.
    Some(Range {
        start: Position { line: line - 1, character: 0 },
        end: Position { line: line - 1, character: text.chars().count() as u32 },
    })
}

/// The 1-based source line a name is declared on.
fn declaration_line(program: &guml_ast::Program, name: &str) -> Option<u32> {
    if let Some(s) = program.states.iter().find(|s| s.name == name) {
        return Some(s.span.line);
    }
    if let Some(r) = program.resources.iter().find(|r| r.name == name) {
        return Some(r.span.line);
    }
    if let Some(d) = program.defs.iter().find(|d| d.name == name) {
        return Some(d.span.line);
    }
    if let Some(t) = program.types.iter().find(|t| t.name == name) {
        return Some(t.span.line);
    }
    None
}

/// Every occurrence of a name, on word boundaries.
///
/// Lexical on purpose. Resolving each site would miss the places a name appears that the resolver does
/// not model — inside a `js` body, inside an action, in prose interpolation — and a rename that misses
/// one silently breaks the document. Over-matching is the safer error, and `rename` catches it.
pub fn references(src: &str, name: &str) -> Vec<Range> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let needle: Vec<char> = name.chars().collect();
    if needle.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        // A comment line names nothing.
        if line.trim_start().starts_with("//") {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut j = 0;
        while j + needle.len() <= chars.len() {
            let hit = chars[j..j + needle.len()] == needle[..];
            let before_ok = j == 0 || !is_word(chars[j - 1]);
            let after_ok = chars.get(j + needle.len()).copied().is_none_or(|c| !is_word(c));
            if hit && before_ok && after_ok {
                out.push(Range {
                    start: Position { line: i as u32, character: j as u32 },
                    end: Position { line: i as u32, character: (j + needle.len()) as u32 },
                });
                j += needle.len();
            } else {
                j += 1;
            }
        }
    }
    out
}

/// Why a rename was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum RenameError {
    /// The cursor is not on a name this document declares.
    NotADeclaration,
    /// The new name could not be written as a bare word, so no document could reference it.
    BadName(String),
    /// Already taken by another declaration, or by a registry tag.
    Taken(String),
    /// The rename produced errors the original document did not have.
    WouldBreak(String),
}

/// Rename a declaration and every reference to it.
///
/// Returns the ranges to replace, so the caller builds the protocol edit. The rename is applied
/// internally first and the result re-checked — see the module note on why refusing beats returning a
/// broken edit.
pub fn rename(src: &str, at: Position, new_name: &str) -> Result<Vec<Range>, RenameError> {
    let old = src
        .lines()
        .nth(at.line as usize)
        .and_then(|l| word_at(l, at.character as usize))
        .ok_or(RenameError::NotADeclaration)?;

    let (program, before) = guml_compiler::check(src);
    if declaration_line(&program, &old).is_none() {
        return Err(RenameError::NotADeclaration);
    }
    if old == new_name {
        return Ok(Vec::new());
    }

    let legal = new_name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && new_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !legal {
        return Err(RenameError::BadName(new_name.to_string()));
    }
    if Registry::builtin().get(new_name).is_some() {
        return Err(RenameError::Taken(format!("`{new_name}` is a component in the registry")));
    }
    if declaration_line(&program, new_name).is_some() {
        return Err(RenameError::Taken(format!("`{new_name}` is already declared")));
    }

    let ranges = references(src, &old);
    let renamed = apply(src, &ranges, new_name);
    let (_, after) = guml_compiler::check(&renamed);

    // Error *counts*, not sets: a rename legitimately moves spans and rewrites messages, so set
    // equality would refuse every rename. A new error is the signal that the lexical scan overreached.
    let errors = |d: &guml_diagnostics::Diagnostics| {
        d.items.iter().filter(|i| i.severity == Severity::Error).count()
    };
    if errors(&after) > errors(&before) {
        let first = after
            .items
            .iter()
            .find(|d| d.severity == Severity::Error)
            .map(|d| format!("{}: {}", d.id, d.message))
            .unwrap_or_default();
        return Err(RenameError::WouldBreak(first));
    }

    Ok(ranges)
}

/// Apply edits back to front, so an earlier replacement does not shift a later range.
pub fn apply(src: &str, ranges: &[Range], text: &str) -> String {
    let mut lines: Vec<String> = src.lines().map(str::to_string).collect();
    for r in ranges.iter().rev() {
        if let Some(line) = lines.get_mut(r.start.line as usize) {
            let chars: Vec<char> = line.chars().collect();
            let (s, e) = (r.start.character as usize, r.end.character as usize);
            if e <= chars.len() && s <= e {
                let mut next: String = chars[..s].iter().collect();
                next.push_str(text);
                next.extend(chars[e..].iter());
                *line = next;
            }
        }
    }
    let mut out = lines.join("\n");
    if src.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Format only the lines a selection touches.
///
/// `None` when nothing in the selection would change — so the editor records no undo step — and also
/// `None` when formatting changed the document's line count, because a whole-document result cannot
/// then be sliced line-for-line and a wrong range is worse than declining.
pub fn format_range(src: &str, range: Range) -> Option<(Range, String)> {
    let formatted = guml_fmt::format(src, guml_fmt::Options::default());
    if !formatted.changed {
        return None;
    }
    let before: Vec<&str> = src.lines().collect();
    let after: Vec<&str> = formatted.text.lines().collect();
    if before.len() != after.len() || before.is_empty() {
        return None;
    }

    let first = (range.start.line as usize).min(before.len() - 1);
    // A selection ending at character 0 has not entered that line.
    let last = if range.end.character == 0 && range.end.line > range.start.line {
        (range.end.line as usize).saturating_sub(1)
    } else {
        range.end.line as usize
    };
    let last = last.min(before.len() - 1);
    if first > last || before[first..=last] == after[first..=last] {
        return None;
    }

    Some((
        Range {
            start: Position { line: first as u32, character: 0 },
            end: Position { line: last as u32, character: before[last].chars().count() as u32 },
        },
        after[first..=last].join("\n"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A document that compiles cleanly, so a rename test can assert the *result* compiles rather than
    /// inheriting a pre-existing error and proving nothing.
    const DOC: &str = "page Tasks\nstate draft=\"\"\ntype Task {id, title}\ndata tasks:Task[] GET /api/tasks\n  add POST /api/tasks {title}\nform >tasks.add{title:draft}; draft=\"\"\n  input draft aria=\"New\"\nlist tasks\n  text {title}\n";

    fn at(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn definition_finds_each_kind_of_declaration() {
        // `draft` is used on the `input` line (6, zero-based) and declared on line 1.
        let d = definition(DOC, at(6, 8)).expect("state definition");
        assert_eq!(d.start.line, 1, "should point at `state draft`");

        // `tasks` used by `list` on line 7, declared on line 3.
        let d = definition(DOC, at(7, 5)).expect("resource definition");
        assert_eq!(d.start.line, 3);
    }

    #[test]
    fn a_registry_tag_has_no_definition_in_the_document() {
        // `card` resolves against the registry, not the file. Jumping somewhere plausible would be
        // worse than doing nothing.
        let src = "page P\ncard Hi\n";
        assert!(definition(src, at(1, 1)).is_none());
    }

    #[test]
    fn definition_of_a_def_points_at_the_def_line() {
        let src = "page P\ndef panel title\n  card {title}\npanel \"Hi\"\n";
        let d = definition(src, at(3, 2)).expect("def definition");
        assert_eq!(d.start.line, 1);
    }

    #[test]
    fn references_respect_word_boundaries() {
        // The bug a naive substring search has: `draft` must not match inside `drafts`.
        let src = "page P\nstate draft=\"\"\nstate drafts=0\nmetric {draft}\nmetric {drafts}\n";
        let hits = references(src, "draft");
        assert_eq!(hits.len(), 2, "expected the declaration and one use, got {hits:?}");
        assert_eq!(hits[0].start.line, 1);
        assert_eq!(hits[1].start.line, 3);
    }

    #[test]
    fn references_skip_comment_lines() {
        let src = "page P\nstate draft=\"\"\n// draft is a draft\nmetric {draft}\n";
        let hits = references(src, "draft");
        assert!(hits.iter().all(|r| r.start.line != 2), "a comment was renamed: {hits:?}");
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn rename_rewrites_the_declaration_and_every_use() {
        // `memo` rather than `note`: `note` became a registry tag in 0.2, and the rename guard now
        // refuses it with `Taken`. That refusal is the feature — see
        // `rename_into_a_registry_tag_is_refused` — so this case needs a name that is not a component.
        let ranges = rename(DOC, at(1, 7), "memo").expect("rename should be allowed");
        let out = apply(DOC, &ranges, "memo");
        assert!(out.contains("state memo=\"\""), "{out}");
        assert!(out.contains("input memo aria=\"New\""), "{out}");
        // Inside an action body too, which a resolver-driven rename would have missed.
        assert!(out.contains("{title:memo}; memo=\"\""), "{out}");
        assert!(!out.contains("draft"), "an occurrence was left behind:\n{out}");

        // And the result still compiles.
        let (_, diags) = guml_compiler::check(&out);
        assert!(!diags.has_errors(), "{:?}", diags.items);
    }

    #[test]
    fn rename_reaches_into_a_js_block() {
        // The strongest argument for a lexical scan: nothing resolves names inside an escape hatch, so
        // a resolver-driven rename would leave the block referring to a variable that no longer exists
        // and the emitted output would not compile.
        let src = "page P\nstate month=all|q1\njs\n  const isQ1 = month === \"q1\";\n";
        let ranges = rename(src, at(1, 7), "period").expect("rename");
        let out = apply(src, &ranges, "period");
        assert!(out.contains("const isQ1 = period === \"q1\";"), "{out}");
    }

    #[test]
    fn rename_refuses_a_name_that_is_not_writable() {
        assert!(matches!(rename(DOC, at(1, 7), "2bad"), Err(RenameError::BadName(_))));
        assert!(matches!(rename(DOC, at(1, 7), "with space"), Err(RenameError::BadName(_))));
    }

    #[test]
    fn rename_refuses_a_name_that_is_taken() {
        // By a registry tag…
        assert!(matches!(rename(DOC, at(1, 7), "card"), Err(RenameError::Taken(_))));
        // …or by another declaration in the same document.
        assert!(matches!(rename(DOC, at(1, 7), "tasks"), Err(RenameError::Taken(_))));
    }

    #[test]
    fn rename_refuses_when_the_cursor_is_not_on_a_declaration() {
        // `card` is a tag, not something this document declares.
        let src = "page P\ncard Hi\n";
        assert_eq!(rename(src, at(1, 1), "panel"), Err(RenameError::NotADeclaration));
    }

    #[test]
    fn renaming_to_the_same_name_is_a_no_op() {
        assert_eq!(rename(DOC, at(1, 7), "draft"), Ok(Vec::new()));
    }

    #[test]
    fn format_range_touches_only_the_selected_lines() {
        // Two badly indented lines, in a document the formatter neither grows nor shrinks — the blank
        // line after the declarations is already there, so the whole-document result can be sliced
        // line-for-line.
        let src = "page P\n\ncard A\n      p One\n      p Two\n";
        let (range, text) = format_range(src, Range { start: at(3, 0), end: at(3, 11) })
            .expect("the selected line should change");
        assert_eq!(range.start.line, 3);
        assert_eq!(range.end.line, 3, "the untouched line must not be in the edit");
        assert_eq!(text, "  p One");
    }

    #[test]
    fn format_range_declines_when_the_selection_is_already_formatted() {
        // No edit means no undo step in the editor.
        let src = "page P\n\ncard A\n  p One\n      p Two\n";
        assert!(format_range(src, Range { start: at(3, 0), end: at(3, 7) }).is_none());
    }

    #[test]
    fn format_range_declines_when_the_line_count_would_change() {
        // The formatter inserts a blank line after the declarations here, so a whole-document result
        // cannot be sliced line-for-line. Returning nothing beats returning a wrong range.
        let src = "page P\ncard A\n      p One\n";
        let out = guml_fmt::format(src, guml_fmt::Options::default());
        if out.text.lines().count() != src.lines().count() {
            assert!(format_range(src, Range { start: at(2, 0), end: at(2, 11) }).is_none());
        }
    }
}
