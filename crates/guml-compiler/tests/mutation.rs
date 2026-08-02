//! The Phase 2 gate: **recovers and reports ≥90% of injected single-token mutations without cascading.**
//!
//! # Why this is a gate rather than a nice-to-have
//!
//! Invariant 1 says the parser collects every error in one pass, and the reason is economic: each repair
//! round is a full LLM generation. But "collects every error" is only useful if the errors are *the author's
//! mistake* rather than the parser's confusion about it. One mistyped tag producing eleven diagnostics
//! across the rest of the file gives a repair loop eleven things to fix, ten of which would disappear on
//! their own — and a model handed that list will edit ten lines that were correct.
//!
//! Two properties, reported separately because they fail for different reasons:
//!
//! * **Detection** — the compiler reported an error. A mutation that produces none is the worse failure: a
//!   corrupted document accepted in silence, which a repair loop has no way to notice.
//! * **Localisation** — every *new* error line is the line that was mutated. This is the operational
//!   meaning of "without cascading", and it is a stronger claim than a diagnostic count: it says the parser
//!   resynchronised at the line boundary and the rest of the document still parsed.
//!
//! # Two things the first version of this file got wrong, because they are the whole difficulty
//!
//! **A mutation has to actually be invalid.** The first version deleted a trailing token from any line, and
//! `card sm center` → `card sm` is *completely valid GUML* — a card with one fewer modifier. So is dropping
//! a button's action. Those went into the denominator as "missed detections" and dragged the figure to 79%,
//! measuring nothing but the generator's own carelessness. A benchmark whose denominator contains cases
//! where the correct answer is "no error" cannot be read in either direction.
//!
//! So every mutation here is **definitionally invalid**: the result cannot be legal GUML whatever the rest
//! of the document says. An unclosed brace is unbalanced by construction; a mutated tag is checked against
//! the registry before the mutant is counted, so a typo that lands on another real tag is discarded rather
//! than scored.
//!
//! **A mistake is *supposed* to reach other lines, semantically.** `state count=0` → `sate count=0`
//! reported errors on lines 7, 9, 10 and 11 — the lines that read `count`. `list tasks` → `lst tasks` did
//! the same to the repeater's children, which lose the row scope the repeater gave them. Neither is a
//! cascade: the resolver is doing exactly its job, and suppressing those reports would be the bug.
//!
//! So "cascading" is defined as what it means for a *parser*: **a lexical, layout or syntax error
//! (`GUML0001`–`GUML0023`) on a line the mutation did not touch.** That is the parser having lost its place
//! and started mis-reading lines that are fine — the failure that hands a repair loop spurious work, as
//! against a semantic consequence, which hands it information. Measured this way the current figure is
//! 100%: across every mutant, not one produced a stray syntax error, and the entire reach is the resolver's.
//!
//! Semantic reach is reported too, as a number rather than a gate, because it is worth watching and is not a
//! defect.
//!
//! # The mutations
//!
//! Chosen to model what a language model gets wrong rather than what a bit-flipper produces:
//!
//! | mutation | the real mistake |
//! |---|---|
//! | `tag-typo` | `crd` for `card` — a dropped character |
//! | `tag-transposed` | `cadr` — a transposition, which Levenshtein scores as two edits |
//! | `brace-unclosed` | the most common model error in any brace language |
//! | `quote-unclosed` | an unterminated string |
//! | `binding-typo` | `{cont}` where `count` is declared — a reference that does not resolve |
//! | `action-typo` | `>tasks.ad` — a mutation name that does not exist on the resource |
//!
//! Indentation gets its own test. Re-indenting a line is frequently *valid* — `card A` / `  p One` /
//! `    p Two` is a legal three-level tree — so it is not definitionally invalid and cannot be scored the
//! same way. What is asserted there is that its blast radius stays bounded.

use guml_compiler::check;
use guml_registry::{Registry, TagKind};
use std::collections::BTreeSet;

/// Directive keywords. A mistake on one of these lines has legitimate document-wide consequences.
const DIRECTIVES: &[&str] =
    &["page", "type", "data", "state", "store", "on", "route", "auth", "def"];

/// Every document in the repository that is known-good GUML.
fn corpus() -> Vec<(String, String)> {
    let dirs =
        ["../../fixtures", "../../bench/phase0/examples", "../../bench/guml-bench/reference"];
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        let mut names: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "guml"))
            .collect();
        // Sorted, so the mutant order — and therefore any failure — reproduces exactly.
        names.sort();
        for path in names {
            if let Ok(src) = std::fs::read_to_string(&path) {
                out.push((path.display().to_string(), src));
            }
        }
    }
    assert!(!out.is_empty(), "no corpus documents found — the directory list is wrong");
    out
}

/// A line that carries structure, with whether it is a directive.
struct Target {
    index: usize,
    text: String,
    is_directive: bool,
}

/// Lines whose remainder is structure rather than prose or a verbatim body.
///
/// A text-kind tag's remainder is prose taken verbatim, so most edits to it are still legal and it is
/// excluded. `js`/`raw` bodies are not GUML at all. The rule comes from the same registry the parser uses,
/// so the two cannot disagree about which lines carry structure.
fn targets(src: &str, reg: &Registry) -> Vec<Target> {
    let mut out = Vec::new();
    let mut verbatim_indent: Option<usize> = None;

    for (index, line) in src.lines().enumerate() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();

        if let Some(open) = verbatim_indent {
            if trimmed.is_empty() || indent > open {
                continue;
            }
            verbatim_indent = None;
        }
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let first = trimmed.split_whitespace().next().unwrap_or("");
        let opens_verbatim = first == "js" || first == "raw" || reg.children_are_text(first);
        if opens_verbatim {
            verbatim_indent = Some(indent);
        } else if reg.get(first).is_some_and(|d| d.kind == TagKind::Text) {
            continue;
        }
        // A `data` block's indented mutations are directive lines too: they declare names.
        let is_directive = DIRECTIVES.contains(&first) || (indent > 0 && line.contains(" /"));
        out.push(Target { index, text: line.to_string(), is_directive });
    }
    out
}

/// Drop the character at `at` from `s`, by char index.
fn drop_char(s: &str, at: usize) -> String {
    s.chars().enumerate().filter(|(i, _)| *i != at).map(|(_, c)| c).collect()
}

/// Definitionally-invalid single-token mutations of one line.
///
/// `reg` is consulted so a tag typo that happens to land on another real tag is not produced at all — that
/// mutant would be legal GUML and belongs nowhere near the denominator.
fn mutants(line: &str, reg: &Registry) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let indent = line.len() - line.trim_start().len();
    let body = line.trim_start();
    let pad = " ".repeat(indent);

    let tag: String = body.split([' ', '=']).next().unwrap_or("").to_string();
    let rest = &body[tag.len()..];
    let still_a_tag = |candidate: &str| {
        reg.get(candidate).is_some() || DIRECTIVES.contains(&candidate) || candidate.len() < 2
    };

    if tag.chars().count() >= 3 {
        let typo = drop_char(&tag, 1);
        if !still_a_tag(&typo) {
            out.push(("tag-typo", format!("{pad}{typo}{rest}")));
        }
        let mut swapped: Vec<char> = tag.chars().collect();
        swapped.swap(1, 2);
        let swapped: String = swapped.into_iter().collect();
        if swapped != tag && !still_a_tag(&swapped) {
            out.push(("tag-transposed", format!("{pad}{swapped}{rest}")));
        }
    }

    // Unbalanced by construction, whatever the rest of the document says.
    for (kind, ch) in [("brace-unclosed", '}'), ("quote-unclosed", '"')] {
        if let Some(at) = body.rfind(ch) {
            let mut b = body.to_string();
            b.remove(at);
            out.push((kind, format!("{pad}{b}")));
        }
    }

    // A reference that no longer resolves. The head identifier inside the first `{…}`.
    if let (Some(open), Some(close)) = (body.find('{'), body.find('}')) {
        if close > open + 1 {
            let inner = &body[open + 1..close];
            let head: String =
                inner.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if head.chars().count() >= 3 {
                let typo = drop_char(&head, 1);
                let b = format!("{}{}{}", &body[..open + 1], typo, &body[open + 1 + head.len()..]);
                out.push(("binding-typo", format!("{pad}{b}")));
            }
        }
    }

    // An action target that does not exist: `>tasks.ad`.
    if let Some(at) = body.find('>') {
        let after = &body[at + 1..];
        let target: String = after
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '.' || *c == '_')
            .collect();
        if let Some(dot) = target.rfind('.') {
            let (head, name) = (&target[..dot], &target[dot + 1..]);
            if name.chars().count() >= 3 {
                let typo = drop_char(name, 1);
                let lead = after.len() - after.trim_start().len();
                let b = format!(
                    "{}{}{}.{}{}",
                    &body[..at + 1],
                    &after[..lead],
                    head,
                    typo,
                    &after[lead + target.len()..]
                );
                out.push(("action-typo", format!("{pad}{b}")));
            }
        }
    }

    out
}

/// Source lines carrying at least one *error*, and separately those carrying a *syntax* error.
///
/// The split is the whole metric. `GUML0001`–`GUML0023` are lexical, layout and syntax: the parser reading
/// the line wrongly. Everything above that is the resolver reading it correctly and objecting to what it
/// says. Only the first kind can cascade.
fn error_lines(src: &str) -> (BTreeSet<u32>, BTreeSet<u32>) {
    let (_, diags) = check(src);
    let mut all = BTreeSet::new();
    let mut syntax = BTreeSet::new();
    for d in diags.items.iter().filter(|d| d.severity == guml_diagnostics::Severity::Error) {
        all.insert(d.span.line);
        // The id is `GUML####`; the syntax decades are 0001-0023.
        if d.id.strip_prefix("GUML").and_then(|n| n.parse::<u32>().ok()).is_some_and(|n| n <= 23) {
            syntax.insert(d.span.line);
        }
    }
    (all, syntax)
}

#[derive(Default)]
struct Tally {
    total: usize,
    detected: usize,
    /// Detected, and with no stray *syntax* error — the gate.
    localised: usize,
    /// Total extra lines reached semantically, for the informational figure.
    semantic_reach: usize,
    /// Mutants landing on a declaration line, where semantic reach is legitimate by design — a `state` no
    /// longer declared genuinely invalidates every reference to it. Reported so the reach figure below can
    /// be read: most of it comes from these.
    on_declarations: usize,
    widest_semantic_reach: usize,
    missed: Vec<(String, &'static str, String)>,
    /// Mutants that desynced the parser: a syntax error on a line they did not touch.
    cascaded: Vec<(String, &'static str, String, Vec<u32>)>,
}

fn run(kinds: &[&str], indentation: bool) -> Tally {
    let reg = Registry::builtin();
    let mut t = Tally::default();

    for (name, src) in corpus() {
        // Subtracted rather than assumed to be empty, so this stays correct if a corpus document ever
        // acquires a legitimate error.
        let (before, before_syntax) = error_lines(&src);
        let lines: Vec<&str> = src.lines().collect();

        for target in targets(&src, &reg) {
            let generated: Vec<(&'static str, String)> = if indentation {
                vec![("over-indented", format!("  {}", target.text))]
            } else {
                mutants(&target.text, &reg)
            };
            for (kind, mutated) in generated {
                if !kinds.contains(&kind) || mutated == target.text {
                    continue;
                }
                let mut copy = lines.clone();
                copy[target.index] = mutated.as_str();
                let doc = format!("{}\n", copy.join("\n"));

                t.total += 1;
                if target.is_directive {
                    t.on_declarations += 1;
                }
                let (after, after_syntax) = error_lines(&doc);
                let fresh: Vec<u32> = after.difference(&before).copied().collect();

                if fresh.is_empty() {
                    t.missed.push((name.clone(), kind, mutated));
                    continue;
                }
                t.detected += 1;

                let touched = target.index as u32 + 1;
                let stray_syntax: Vec<u32> = after_syntax
                    .difference(&before_syntax)
                    .copied()
                    .filter(|l| *l != touched)
                    .collect();
                let reach = fresh.iter().filter(|l| **l != touched).count();
                t.semantic_reach += reach;
                t.widest_semantic_reach = t.widest_semantic_reach.max(reach);

                if stray_syntax.is_empty() {
                    t.localised += 1;
                } else {
                    t.cascaded.push((name.clone(), kind, mutated, stray_syntax));
                }
            }
        }
    }
    t
}

const TOKEN_KINDS: &[&str] = &[
    "tag-typo",
    "tag-transposed",
    "brace-unclosed",
    "quote-unclosed",
    "binding-typo",
    "action-typo",
];

#[test]
fn single_token_mutations_are_detected_and_stay_on_their_own_line() {
    let t = run(TOKEN_KINDS, false);
    assert!(t.total >= 200, "only {} mutants — the generator is not reaching the corpus", t.total);

    let detection = 100.0 * t.detected as f64 / t.total as f64;
    let localisation = 100.0 * t.localised as f64 / t.detected as f64;

    // Printed unconditionally. The gate is a number in `ROADMAP.md`, and a number whose current value
    // nobody can see is a number that gets quoted from memory.
    println!(
        "single-token mutations: {} definitionally-invalid mutants, {} on declaration lines\n           detected                      {:.1}%  ({} produced no error)\n           no stray syntax error         {:.1}%  ({} desynced the parser)\n           semantic reach, mean          {:.2} extra lines (widest {})",
        t.total,
        t.on_declarations,
        detection,
        t.missed.len(),
        localisation,
        t.cascaded.len(),
        t.semantic_reach as f64 / t.detected.max(1) as f64,
        t.widest_semantic_reach,
    );
    for (name, kind, line) in t.missed.iter().take(8) {
        println!("  MISSED    {kind:<16} {name}\n            {line}");
    }
    for (name, kind, line, strayed) in t.cascaded.iter().take(8) {
        println!("  CASCADED  {kind:<16} {name} -> also lines {strayed:?}\n            {line}");
    }

    // The Phase 2 gate. Detection first: a missed mutation is the worse failure, because a cascade gives a
    // repair loop too much work while a miss hands it a corrupted document it believes is fine.
    assert!(
        detection >= 90.0,
        "detection {detection:.1}% is below the 90% gate — {} of {} definitionally-invalid mutants produced no error",
        t.missed.len(),
        t.total
    );
    assert!(
        localisation >= 90.0,
        "localisation {localisation:.1}% is below the 90% gate — {} of {} detected mutants produced a syntax error on a line they did not touch",
        t.cascaded.len(),
        t.detected
    );
}

#[test]
fn an_indentation_mistake_has_a_bounded_blast_radius() {
    // Its own test with its own claim, because re-indenting a line is frequently *valid* GUML — `card A` /
    // `  p One` / `    p Two` is a legal three-level tree. A detection rate here would be asserting that
    // legal documents are rejected, so what is asserted instead is that when it *is* wrong, the report does
    // not spread across the file.
    let t = run(&["over-indented"], true);
    assert!(t.total >= 20, "only {} indentation mutants", t.total);

    println!(
        "indentation mutations: {} mutants · {} reported an error · {} with no stray syntax error",
        t.total, t.detected, t.localised
    );
    for (name, kind, line, strayed) in t.cascaded.iter().take(5) {
        println!("  SPREAD    {kind:<16} {name} -> also lines {strayed:?}\n            {line}");
    }

    let worst = t.cascaded.iter().map(|(_, _, _, s)| s.len()).max().unwrap_or(0);
    println!("  widest blast radius: {worst} other line(s)");
    assert!(
        worst <= 4,
        "an indentation mistake reported errors on {worst} other lines: {:?}",
        t.cascaded.iter().max_by_key(|(_, _, _, s)| s.len())
    );
}

#[test]
fn no_mutant_panics_or_hangs() {
    // The other half of the Phase 2 gate. `run` calls `check` on every mutant, so reaching here without
    // unwinding *is* the assertion — but it earns its own name, because "the fuzz test passed" and "nothing
    // panicked on 1,900 malformed documents" are the same fact with very different legibility in a CI log.
    let total = run(TOKEN_KINDS, false).total + run(&["over-indented"], true).total;
    println!("{total} malformed documents compiled without a panic");
    assert!(total >= 200);
}
