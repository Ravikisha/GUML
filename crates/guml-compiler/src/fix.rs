//! Applying diagnostics without asking a model.
//!
//! Every diagnostic that carries a `suggestion` is a repair the compiler already knows how
//! to make. Spending an LLM round to rename `crad` to `card` is the most expensive possible
//! way to fix a typo: a full generation, billed at output rates, for an edit the compiler
//! described precisely.
//!
//! So this is the free layer of the repair loop. It runs before any model call and reports
//! what it changed, which is what makes the saving measurable rather than assumed.
//!
//! # What counts as applicable
//!
//! A suggestion is a *replacement* for its span, unless it contains `…` — accessibility
//! diagnostics suggest shapes like `toggle aria="…"` where the ellipsis is a placeholder for
//! a human to fill in. Splicing that in literally would put an ellipsis in the accessible
//! name, which is worse than the diagnostic it silenced.

use guml_diagnostics::Diagnostic;

/// Placeholder marker inside a suggested shape.
const PLACEHOLDER: char = '…';

pub fn is_applicable(d: &Diagnostic) -> bool {
    d.suggestion.as_deref().is_some_and(|s| !s.contains(PLACEHOLDER) && !s.contains('\n'))
}

/// Whether the edit is a token rename rather than a line rewrite.
///
/// A suggestion replaces its span, so a bare word attached to a whole-line span replaces the
/// line with that word. The validator no longer does this, but diagnostics come from four
/// crates and the failure mode is silent data loss, so the applier checks rather than trusts:
/// if the span covers whitespace, the replacement has to as well.
fn spans_one_token(span_text: &str, replacement: &str) -> bool {
    !span_text.contains(char::is_whitespace) || replacement.contains(char::is_whitespace)
}

#[derive(Debug, Default)]
pub struct Applied {
    pub text: String,
    /// Codes that were applied, in source order. One entry per edit.
    pub codes: Vec<String>,
    /// Rounds it took to settle.
    pub rounds: usize,
}

/// Apply every applicable suggestion, re-checking until nothing changes.
///
/// Iterating matters: fixing an unknown tag lets the resolver see the element's attributes
/// for the first time, which can surface a second suggestion that was previously masked. The
/// bound stops a pathological document from looping.
pub fn fix(src: &str, max_rounds: usize) -> Applied {
    let mut text = src.to_string();
    let mut codes = Vec::new();
    let mut rounds = 0;

    for _ in 0..max_rounds {
        let (_, diags) = crate::check(&text);
        let mut edits: Vec<(usize, usize, String, String)> = diags
            .items
            .iter()
            .filter(|d| is_applicable(d))
            .map(|d| {
                (d.span.start, d.span.end, d.suggestion.clone().unwrap_or_default(), d.id.clone())
            })
            .collect();

        if edits.is_empty() {
            break;
        }

        // Right to left, so earlier offsets stay valid as later text is replaced.
        edits.sort_by_key(|(start, ..)| std::cmp::Reverse(*start));
        // Overlapping spans would corrupt each other; keep the first (rightmost) of any pair.
        let mut last_start = usize::MAX;
        let mut applied_this_round = 0;
        for (start, end, replacement, code) in edits {
            if end > last_start {
                continue;
            }
            if start > text.len() || end > text.len() || start > end {
                continue;
            }
            if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
                continue;
            }
            if !spans_one_token(&text[start..end], &replacement) {
                continue;
            }
            text.replace_range(start..end, &replacement);
            codes.push(code);
            last_start = start;
            applied_this_round += 1;
        }

        rounds += 1;
        if applied_this_round == 0 {
            break;
        }
    }

    Applied { text, codes, rounds }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typo_is_fixed_without_a_model() {
        let out = fix("page P\n\ncrad Hello\n  p x\n", 3);
        assert!(out.text.contains("card Hello"), "{}", out.text);
        assert_eq!(out.codes, vec!["GUML0030"]);
        assert!(!crate::check(&out.text).1.has_errors());
    }

    #[test]
    fn a_template_suggestion_is_left_alone() {
        // `toggle aria="…"` is a shape for a human. Applying it puts an ellipsis in the
        // accessible name, which silences the diagnostic and keeps the defect.
        let src = "page P\nstate on=false\n\ntoggle {on}\n";
        let out = fix(src, 3);
        assert_eq!(out.text, src, "nothing applicable");
        assert!(!out.text.contains('…'));
    }

    #[test]
    fn several_typos_are_fixed_in_one_pass() {
        let out = fix("page P\n\ncrad One\n  p a\ncrad Two\n  p b\n", 3);
        assert_eq!(out.text.matches("card").count(), 2, "{}", out.text);
        assert_eq!(out.codes.len(), 2);
    }

    #[test]
    fn a_line_span_is_never_replaced_by_a_bare_word() {
        // `GUML0061` knows the mutation is called `save`, but its span is the whole element:
        // the AST does not carry token spans inside an action. Splicing a bare word there
        // would replace the entire line with `save`, so the name lives in `help` instead and
        // the applier refuses the edit even if a diagnostic offers it.
        let src = "page P\n\
                   type T {id, done:bool}\n\
                   data rows:T[] GET /api/rows\n\
                   \x20 save PATCH /api/rows/{id} {done} optimistic\n\
                   list rows\n\
                   \x20 text {id}\n\
                   \x20 check {done} aria=\"done\" >rows.sve\n";
        let out = fix(src, 3);
        assert_eq!(out.text, src, "left for a human rather than mangled");

        let diags = crate::check(src).1;
        let d = diags.items.iter().find(|d| d.id == "GUML0061").expect("reported");
        assert!(d.help.as_deref().unwrap_or_default().contains("save"), "the name is in help");
        assert!(d.suggestion.is_none(), "not machine-applicable against a line span");
    }

    #[test]
    fn a_bare_word_against_a_line_span_is_refused_even_if_offered() {
        use guml_diagnostics::{Code, Span};
        // Defence in depth: diagnostics come from four crates, and the failure mode here is
        // silent data loss.
        let d = Diagnostic::error(Code::UnknownTag, "x", Span::new(0, 20, 1, 1))
            .with_suggestion("card");
        assert!(is_applicable(&d), "the suggestion itself is well formed");
        assert!(
            !spans_one_token("card Hello there x", "card"),
            "but it must not replace a span containing whitespace"
        );
    }

    #[test]
    fn a_clean_document_is_returned_untouched_in_one_round() {
        let src = "page P\n\ncard Hi\n  p x\n";
        let out = fix(src, 3);
        assert_eq!(out.text, src);
        assert!(out.codes.is_empty());
        assert_eq!(out.rounds, 0, "no edits means no rounds");
    }

    #[test]
    fn fixing_terminates_even_when_a_suggestion_does_not_resolve() {
        // A suggestion that reproduces the same diagnostic would loop forever without the
        // bound. The document is unchanged and the function returns.
        let out = fix("page P\n\ncrad One\n", 2);
        assert!(out.rounds <= 2);
    }
}
