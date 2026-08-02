---
name: guml-dev
description: Use when building, testing, benchmarking, or debugging the GUML Rust compiler — the crate map, the commands, the invariants that must hold, and where to add what. Load before editing anything under crates/.
---

# Working on the GUML compiler

## Commands

```sh
cargo build --workspace
cargo test  --workspace                     # 49 tests as of scaffold
cargo test  -p guml-parser                  # one crate
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -q -p guml-cli -- build fixtures/a.guml
```

With `just` installed: `just test`, `just check`, `just demo`, `just tokens`.

## Crate map — dependency order, no cycles

| Crate | Owns | Depends on |
|---|---|---|
| `guml-diagnostics` | `Span`, `Code`, `Diagnostic`, JSON + human rendering | — |
| `guml-syntax` | indentation-sensitive line lexer | diagnostics |
| `guml-ast` | typed span-annotated AST, serialisable | diagnostics |
| `guml-registry` | closed tag vocabulary, modifiers, typo suggestions | — |
| `guml-parser` | recursive descent, registry-aware, error-recovering | syntax, ast, registry, diagnostics |
| `guml-codegen` | `Backend` trait, React backend, design-system table | ast, registry, diagnostics |
| `guml-compiler` | driver: one structured result | all of the above |
| `guml-cli` | the `guml` binary | compiler, syntax, registry |

`guml-codegen` must **not** depend on `guml-parser` — that is a cycle through the driver. Codegen
unit tests build ASTs by hand; end-to-end tests from source text live in
`crates/guml-compiler/tests/`.

## Invariants that must not regress

1. **All errors in one pass.** The parser never early-returns on error. Each round trip in the LLM
   repair loop is a full generation, so single-error reporting is a product defect.
2. **Spans are real.** Every diagnostic span slices to the text it names. Test it.
3. **No silent mis-lowering.** Unsupported construct → `unsupported()` warning + a TODO in output.
4. **Diagnostic codes are append-only.** They are a public contract for the repair loop.
5. **Compiler owns presentation.** No class strings, colours, or ARIA plumbing in GUML source.
6. **Compile latency**: `check` < 2 ms, `build` < 10 ms on 200 lines. Hot path for the repair loop.

## Where things go

| Task | Location |
|---|---|
| New token form / lexical rule | `guml-syntax/src/lib.rs` + a unit test |
| New directive or element form | `guml-parser` `try_directive` / `fill_element` |
| New tag or modifier | `guml-registry` `COMPONENTS` / `MODIFIERS` — then check fixtures still parse |
| New diagnostic | `guml-diagnostics` `Code` (append) + emit site + a test asserting the span |
| Emitted-code change | `guml-codegen/src/react.rs` |
| Design system / theme | `classes()` in `react.rs` — swappable wholesale |
| New backend | new module in `guml-codegen`, register in `backend()` |
| End-to-end behaviour | `crates/guml-compiler/tests/end_to_end.rs` |

## Test-first, always

Both bugs found during the initial scaffold were caught by tests written before the code was
trusted: transposed-letter typos produced no suggestion (fixed by optimal string alignment), and a
"trailing tokens after action" diagnostic was unreachable because `>` consumes the line (fixed by
detecting a swallowed modifier instead). Write the failing test first.

Quote real `cargo test` output when reporting completion. Never claim green without running it.
