# Contributing to GUML

Thanks for looking. This file is short on ceremony and long on the few things that
will get a change sent back.

## Getting set up

You need Rust (1.85+, edition 2024), Node 20+ and pnpm. `just` is optional — every
recipe in the `justfile` is a shell command you can run directly.

```sh
cargo test --workspace
cargo run -q -p guml-cli -- build fixtures/a.guml
```

## One command before you open a PR

```sh
just ci
```

That is the *whole* gate — the same set CI runs, deliberately, so that a green local
run and a red CI run cannot become normal. It is more than `cargo test`: emitted
TypeScript is typechecked under `--strict`, emitted components are actually rendered
and checked against the accessibility rules the compiler owns, the Web Components
build is executed, the docs highlighter is compared span-for-span against the real
lexer, the tree-sitter grammar parses every `.guml` in the repo, and both registry
packages are typechecked against their own components.

`just ci-rust` is the subset that needs no Node.

## Test first, and quote the output

Both bugs in the original scaffold were caught by tests written before the code was
trusted, and that has kept happening — every new *class* of check has found bugs no
existing check could. Write the failing test first.

When you say something works, paste the real output. "Tests pass" without the run is
not a claim anyone can check.

## Invariants

These are not style preferences. A change that breaks one will be sent back even if
it is otherwise good, so it is worth reading them before writing much code.

1. **The parser collects every error in one pass.** Each round of the repair loop is
   a full model generation; reporting one error at a time is a product defect.
2. **Diagnostic codes are append-only.** The repair loop keys on them. Add
   `GUML0105`; never renumber `GUML0042`.
3. **Never silently mis-lower.** A construct the compiler cannot handle produces a
   warning and a `TODO` in the output. A quietly wrong compiler destroys the
   reliability the whole project rests on.
4. **The compiler owns presentation.** No class strings, colours, spacing or ARIA
   plumbing in GUML source. This is the token lever and the correctness guarantee at
   the same time.
5. **The spec stays under 3,000 tokens**, including the registry slice and examples.
   The spec carries *rules*; the registry slice carries *vocabulary*. A tag table in
   the spec duplicates a block the prompt already generates from the compiler, so it
   costs tokens and can drift.
6. **One element table, one class table, one expression lowering — across all seven
   backends.** `guml_codegen::element_for`, `theme::active`, `guml_codegen::expr`. A
   per-backend copy is a per-backend chance to disagree about the same document, and
   all three have drifted before. In particular: if you find yourself transforming
   emitted code with string replacement, the transform belongs in the lowerer.
7. **A tag in the registry must lower somewhere.** A tag the prompt offers and the
   compiler cannot emit is worse than no tag — the model is told it exists, uses it,
   and gets a warning.
8. **`guml-codegen` must not depend on `guml-parser`.** It may depend on
   `guml-registry`, and does.

## Performance

`check` and `build` are on the hot path — the LSP calls `check` on a keystroke.
There is a criterion bench:

```sh
cargo bench -p guml-compiler
```

**Read it as a ratio, not in milliseconds.** `calibration/reference` is a fixed pure-Rust
workload measured in the same run; the number that means anything is
`check/200 ÷ calibration`, which has held at 1.25–1.63. The absolute figures on a
developer machine are not a measurement — criterion has reported a 100% regression on
a function nobody touched, and a build doing strictly more work as 22% faster.

## Claims

Three tiers, never blurred, in code comments and in docs alike:

- **Measured** — say what was measured and with which tokenizer.
- **Hypothesised** — label it. Removing the label does not remove the uncertainty.
- **Cited** — someone else's result, with its limitations named.

`guml tokens` is a ~3.6 chars/token estimate and must be described as one. `tiktoken`
is an OpenAI tokenizer and undercounts Claude; its figures do not belong in a README.

## Commits and PRs

Conventional-ish subject lines (`fix(codegen): …`), present tense, and a body that
says *why*. Small PRs. If a change touches a backend, say which of the seven you
checked — the answer should be all of them, because the shared tables make that
cheap.

## Licence

By contributing you agree your contribution is licensed under the MIT License, the
same as the project.
