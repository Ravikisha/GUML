# GUML — working notes for Claude

Read `README.md` for what this is, `ROADMAP.md` for what is next, `GUML-Research-Report.md` for why
any of it is designed this way.

## Available agents and skills

Project subagents in `.claude/agents/`: `guml-lang-designer` (language surface, owns the spec token
budget), `guml-compiler-dev` (front-end Rust), `guml-codegen-dev` (backends, design system),
`guml-registry-curator` (component vocabulary), `guml-llm-loop-dev` (prompting, constrained decoding,
repair loop), `guml-token-analyst` (any number), `guml-bench-runner` (GUML-Bench),
`guml-paper-writer` (drafts, related work), `guml-adversary` (attack it before a referee does).

Project skills in `.claude/skills/`: `guml-write` (authoring/reviewing `.guml`), `guml-dev` (compiler
work), `guml-measure` (before quoting any number), `guml-phase0` (the gate experiment).

## Commands

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -q -p guml-cli -- build fixtures/a.guml
cargo run -q -p guml-cli -- check fixtures/b.guml --format json
```

## Invariants — do not break these

1. **The parser collects every error in one pass.** Each repair-loop round is a full LLM generation;
   single-error reporting is a product defect, not a style preference.
2. **Diagnostic codes are append-only** — the repair loop keys on them.
3. **Never silently mis-lower.** Unsupported construct → warning + TODO in output. A quietly wrong
   compiler destroys the reliability claim the whole project rests on.
4. **The compiler owns presentation.** No class strings, colours, spacing or ARIA plumbing in GUML
   source — that is the token lever and the correctness guarantee simultaneously.
5. **Spec budget ≤3,000 tokens** including registry slice and examples — currently **~2,935**, and it
   took work to keep. Growing the vocabulary 28 → 49 pushed it to ~3,221. The resolution that matters
   for anyone editing `spec/GUML-SPEC.md`: **the spec carries rules, the registry slice carries
   vocabulary.** The assembled prompt already appends an `Available tags` block generated from the
   compiler, so a tag table in the spec is a duplicate that both costs tokens and can drift. Maintainer
   notes go in an HTML comment — `readSpec` strips them, so they are free.

   Past that budget, the amortisation
   math weakens and in-context learnability degrades.
6. **Compile latency**: `check` < 2 ms, `build` < 10 ms on 200 lines. Hot path — the LSP calls
   `check` on a keystroke and the repair loop calls it between model rounds.

   **Judge it as a ratio, not in milliseconds.** Absolute timings on this machine are not a
   measurement: criterion has reported a 100% regression on a function that was not touched between
   runs, and a build doing strictly *more* work as 22% faster. `calibration/reference` is a fixed
   pure-Rust workload measured in the same run, and `check/200 ÷ calibration` has held at **1.44–1.63**
   across runs where the absolute spanned 1.19–3.77 ms.

   Latest, after the 0.2 vocabulary and two new analysis passes. Two consecutive runs, which is the
   clearest possible demonstration of why the ratio is the metric:

   | run | calibration | check/200 | ratio |
   |---|---|---|---|
   | 1 | 2.60 ms | 2.18 ms | 0.84 |
   | 2 | 1.32 ms | 1.87 ms | 1.41 |
   | 3 | 1.00 ms | 1.26 ms | 1.25 |

   Run 3 is after `check_modifier_in_prose`, the field-chain aggregate rule, the free-text `where=` filter
   and the single-object resource check. **Ratio 1.25**, below the historical band, so none of it regressed
   `check` — and note that run 3's *absolute* 1.26 ms would look like a 33% improvement over run 2's 1.87 ms
   if anyone read the milliseconds, on a compiler doing strictly more work.

   The calibration workload — *fixed, pure Rust, unchanged between runs* — differed by 2× between two
   invocations minutes apart. Ratio 1.41 sits at the low end of the historical 1.44–1.63 band, so adding
   `check_positionals`, `check_children` and a JSON-parsed registry did not measurably regress `check`.
   Anyone reading the absolute 1.87 ms as "now under the 2 ms budget" is reading noise.

   Attributed by stage (`cargo bench -p guml-compiler -- stage`): **lex 686 µs, parse 1.37 ms (lex
   included), analyse 364 µs**. The lexer is still the largest single share, and where to look first.

   The bench earns its place: it caught a per-element `Vec` allocation in the React backend that had
   pushed check to 2.47 ms and nobody had noticed, because every test still passed. Run it after
   touching the analysis or codegen path.
7. `guml-codegen` must not depend on `guml-parser` (cycle through the driver). It *may* depend on
   `guml-registry`, and does — a comment in `theme.rs` claimed otherwise for a while, which is why the
   focus-ring list and the "needs a runtime" list were hardcoded there instead of read from the entries
   that declare them.
8. **One element table, one class table, one expression lowering — across *all seven* backends**
   (`react`, `svelte`, `html`, `wc`, `json`, `a2ui`, `mcp-ui`). Element via
   `guml_codegen::element_for`, classes via `theme::active`, expressions via `guml_codegen::expr`. A
   per-backend copy of any of the three is a per-backend chance to disagree about the same document, and
   all three have already drifted:

   - Three copies of the *element* mapping meant `nav`/`hero`/`footer` were `<div>` in the static-HTML
     backend where React emitted landmarks — so the no-JavaScript build shipped a page with none.
     `every_tag_lowers_to_the_same_element_in_every_backend` pins it.
   - The subtlest was *expression* lowering. The Web Components backend needs `count` to become
     `this.#state.count`, and doing that by rewriting the **lowered string** cannot tell an identifier
     from the contents of a string, the literal text of a template, or a lambda's own parameter — it got
     all three wrong at once. `Ctx::with_scope` applies the prefix during lowering, where the tree says
     "path head". If you find yourself transforming emitted code with string replacement, the transform
     belongs in the lowerer.
9. **A tag in the registry must lower somewhere.** A tag the prompt offers and the compiler cannot emit
   is worse than no tag: the model is told it exists, uses it, and gets a warning plus a `TODO`.
   `every_registry_tag_lowers_in_the_react_backend` fails on an entry added without a lowering.

## Claim discipline

Three tiers, never blurred:

- **Measured** — fixture token counts, passing tests. Say what was measured and with which tokenizer.
- **Hypothesised** — H1–H6: correctness, hallucination, consistency, quality floor, capability
  threshold. Not yet tested. Label them.
- **Cited** — someone else's result, with its limitations named.

Before quoting any number: `guml tokens` is a ~3.6 chars/token estimate and must be labelled as one.
`tiktoken` is an OpenAI tokenizer and undercounts Claude — never let its figures into a README or
paper. Load the `guml-measure` skill.

## Things already decided

- Framing is "IR + compiler + empirical study", not "a new markdown".
- A2UI / MCP-UI are compile targets and distribution channels, not competitors.
- v1 is client-only. Server, DB and auth codegen are deferred — trying all four in v1 is the most
  likely way this fails.
- Actions are deliberately not Turing-complete; anything more goes in a `js` escape block. That
  boundary is also the security boundary.
- Phase 0 is a hard gate with three criteria. Do not soften it after seeing results.

## Test-first, and quote the output

Both bugs in the initial scaffold were caught by tests written before the code was trusted. When
reporting work complete, paste real `cargo test` output. Never assert green without running it.
