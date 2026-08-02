//! The repair pipeline, as a product feature rather than a benchmark script.
//!
//! # What this is
//!
//! Three deterministic layers, in increasing order of what they touch, run in order and reported
//! individually:
//!
//! | layer | what it fixes | cost |
//! |---|---|---|
//! | `sanitize` | code fences, markdown rules, trailing commentary | free |
//! | `format` | indentation, tabs, spacing | free |
//! | `fix` | every unambiguous diagnostic suggestion | free |
//!
//! **No model call happens here, ever.** That is the entire point. A fourth layer — one generation with
//! the diagnostics attached — exists in `bench/gen`, and the measurement that matters is how rarely it is
//! needed: every document this file repairs is a round trip the project never pays for.
//!
//! # Why it was worth moving out of the harness
//!
//! Layers 0–2 lived in `bench/gen/lib/pipeline.mjs`. The consequence was not just duplication, it was
//! that the *measured* pipeline had capabilities the shipped tool did not: the benchmark discounted a
//! ``` fence that `guml check` would still choke on. A harness that is more capable than the product
//! flatters the product, which is the one direction a benchmark must not be wrong in.
//!
//! # Order, and why it is this order
//!
//! Sanitising first, because the other two need something parseable-ish to work on and a fence makes
//! line 1 an error. Formatting second, because `fix` works from spans and the formatter is the only
//! layer that moves them wholesale. `fix` last, because it is the only layer that re-checks in a loop,
//! so it should see input the cheaper layers have already settled.

use crate::sanitize::Stripped;
use guml_diagnostics::{Diagnostics, Severity};

/// What each layer did, and whether the document ended up valid.
#[derive(Debug, Clone)]
pub struct Repaired {
    pub text: String,
    /// What `sanitize` removed.
    pub stripped: Stripped,
    /// Whether formatting changed anything.
    pub reformatted: bool,
    /// Diagnostic codes `fix` applied, one entry per edit.
    pub applied: Vec<String>,
    /// Re-check rounds `fix` needed.
    pub rounds: usize,
    /// Errors before any layer ran.
    pub errors_before: usize,
    /// Errors after every layer ran.
    pub errors_after: usize,
    /// Diagnostics of the repaired text — what a caller should report.
    pub diagnostics: Diagnostics,
}

impl Repaired {
    /// Whether the document is now free of errors.
    pub fn ok(&self) -> bool {
        self.errors_after == 0
    }

    /// Whether any layer changed anything.
    pub fn changed(&self) -> bool {
        !self.stripped.is_empty() || self.reformatted || !self.applied.is_empty()
    }

    /// One line per layer that did something. What a CLI prints and a telemetry record stores.
    pub fn report(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.stripped.is_empty() {
            out.push(format!("sanitize: {}", self.stripped.summary()));
        }
        if self.reformatted {
            out.push("format: whitespace and indentation normalised".to_string());
        }
        if !self.applied.is_empty() {
            out.push(format!(
                "fix: {} edit(s) in {} round(s) ({})",
                self.applied.len(),
                self.rounds,
                self.applied.join(", ")
            ));
        }
        out
    }
}

fn error_count(src: &str) -> (usize, Diagnostics) {
    let (_, diags) = crate::check(src);
    let n = diags.items.iter().filter(|d| d.severity == Severity::Error).count();
    (n, diags)
}

/// Default cap on `fix`'s re-check rounds.
///
/// Three, matching the figure `ROADMAP.md` Phase 5 commits to. Fixing one problem can reveal another —
/// renaming an unknown tag is what first lets the resolver see the element's attributes — so a single
/// pass leaves free repairs on the table. Beyond three, a document is not converging and the remaining
/// errors are the answer.
pub const DEFAULT_ROUNDS: usize = 3;

/// Run every free layer, in order, and report what each one did.
///
/// A layer that would *increase* the error count is discarded rather than kept. This mirrors the rule the
/// measured model-round layer already uses — 7 of 9 model attempts failed to improve and 2 made things
/// worse, so an attempt is only kept if it lowers the count — and applies it to the free layers too,
/// because "deterministic" is not the same as "always an improvement".
pub fn repair(src: &str, max_rounds: usize) -> Repaired {
    let (errors_before, _) = error_count(src);

    let (sanitized, stripped) = crate::sanitize::sanitize(src);
    // Guard the layer rather than trusting it. Sanitising drops lines, and a rule that drops a line it
    // should have kept must not be able to make a document worse than it arrived.
    let (after_sanitize, _) = error_count(&sanitized);
    let (mut text, stripped) = if after_sanitize <= errors_before {
        (sanitized, stripped)
    } else {
        (src.to_string(), Stripped::default())
    };

    let formatted = guml_fmt::format_str(&text);
    let reformatted = formatted != text;
    if reformatted {
        let (after_format, _) = error_count(&formatted);
        // The formatter is documented never to change meaning, and `ast(fmt(x)) == ast(x)` is asserted
        // over every fixture. Checking anyway costs one parse and is the difference between a promise and
        // a guarantee at the point where it matters.
        if after_format <= errors_before.max(after_sanitize) {
            text = formatted;
        }
    }
    let reformatted = reformatted && text != src;

    let applied = crate::fix::fix(&text, max_rounds);
    let (after_fix, _) = error_count(&applied.text);
    let (text, codes, rounds) = if after_fix <= errors_before.max(after_sanitize) {
        (applied.text, applied.codes, applied.rounds)
    } else {
        (text, Vec::new(), 0)
    };

    // Diagnostics come from the text the caller is actually being handed, computed once at the end
    // rather than threaded out of whichever layer happened to run last. That is what makes
    // `errors_after` and `diagnostics` describe the same document.
    let (errors_after, diagnostics) = error_count(&text);

    Repaired {
        text,
        stripped,
        reformatted,
        applied: codes,
        rounds,
        errors_before,
        errors_after,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fenced_generation_with_commentary_repairs_with_no_model_call() {
        // The shape a model actually returns when it ignores "emit GUML only". Before this existed, the
        // product reported a parse error on line 1 while the benchmark quietly discounted it.
        let src = "Here is the counter:\n\n```guml\npage Counter\nstate count=0\n\ncard sm center\n  h Clicks\n  metric {count}\n  btn Increment primary >count++\n```\n\nThis page counts clicks.\n";
        let out = repair(src, DEFAULT_ROUNDS);
        assert!(out.ok(), "did not repair: {:?}", out.diagnostics.items);
        assert!(out.stripped.fence);
        assert!(out.text.starts_with("page Counter"), "{}", out.text);
        assert!(!out.text.contains("This page counts clicks"));
    }

    #[test]
    fn an_html_shaped_generation_repairs_with_no_model_call() {
        // The other half of the model-prior problem: every tag is an HTML element. Edit distance cannot
        // reach any of these, so before the habit table this cost a full generation to fix.
        let src = "page P\ndiv\n  span Hello\n  button Save\n  hr\n";
        let out = repair(src, DEFAULT_ROUNDS);
        assert!(out.ok(), "did not repair: {:?}", out.diagnostics.items);
        for want in ["col", "text Hello", "btn Save", "divider"] {
            assert!(out.text.contains(want), "`{want}` missing from:\n{}", out.text);
        }
    }

    #[test]
    fn a_valid_document_is_untouched() {
        // Runs on every document, so a no-op on a correct one is the property that matters most.
        let src = "page P\nstate count=0\n\ncard sm center\n  h Clicks\n  metric {count}\n";
        let out = repair(src, DEFAULT_ROUNDS);
        assert_eq!(out.text, src);
        assert!(!out.changed(), "{:?}", out.report());
        assert!(out.ok());
        assert!(out.report().is_empty());
    }

    #[test]
    fn a_layer_that_would_make_things_worse_is_discarded() {
        // The rule the measured model-round layer already uses, applied to the free layers: an attempt is
        // kept only if it does not increase the error count. "Deterministic" is not "always better".
        let src = "This is prose, not a document.\n";
        let out = repair(src, DEFAULT_ROUNDS);
        assert!(
            out.errors_after <= out.errors_before,
            "repair made it worse: {} -> {}",
            out.errors_before,
            out.errors_after
        );
    }

    #[test]
    fn the_report_names_the_layer_that_did_the_work() {
        // "The repair loop works" is not a claim worth making. Which layer handled what is.
        //
        // The tab is indented under `card`, not under `page`: a tab-indented line directly after `page P`
        // has no parent element whatever its whitespace, so it would be `GUML0011` even once formatted —
        // a case about parenting rather than about tabs.
        let src = "```\npage P\ncard A\n\tp Tabs for indentation.\n```\n";
        let out = repair(src, DEFAULT_ROUNDS);
        let report = out.report().join("; ");
        assert!(report.contains("sanitize"), "{report}");
        assert!(report.contains("format"), "{report}");
        assert!(out.ok(), "{:?}", out.diagnostics.items);
        // The content line survived rather than being mistaken for commentary and deleted.
        assert!(out.text.contains("Tabs for indentation."), "{}", out.text);
    }

    #[test]
    fn repair_is_idempotent() {
        // Running it twice must not change anything the first run did not, or a pipeline that repairs on
        // save would rewrite a file forever.
        let src = "```guml\npage P\ndiv\n  span Hello\n```\nDone.\n";
        let once = repair(src, DEFAULT_ROUNDS);
        let twice = repair(&once.text, DEFAULT_ROUNDS);
        assert_eq!(once.text, twice.text);
        assert!(!twice.changed(), "{:?}", twice.report());
    }
}
