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
5. **Spec budget ≤3,000 tokens** including registry slice and examples. Past that, the amortisation
   math weakens and in-context learnability degrades.
6. **Compile latency**: `check` < 2 ms, `build` < 10 ms on 200 lines. Hot path.
7. `guml-codegen` must not depend on `guml-parser` (cycle through the driver).

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
