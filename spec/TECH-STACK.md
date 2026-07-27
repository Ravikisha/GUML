# Tech stack

Rust-first, with a hard performance constraint that drives most choices: **the compiler runs
inside the LLM repair loop**, so compile latency is user-visible, not a build-time nicety.

## Chosen

| Layer | Choice | Why this and not the obvious alternative |
|---|---|---|
| Compiler language | **Rust** (edition 2024, MSRV 1.85) | Sub-millisecond compiles for interactive loops; compiles to WASM so the *same* compiler runs in CLI, CI, browser, and inside an AI builder's sandbox. Rejected TypeScript (fast to write, too slow and too loose for a hot-path compiler) and Go (weaker frontend ecosystem for parsing work). |
| Lexer | **Hand-written**, line-oriented | Indentation sensitivity plus prose/structure ambiguity plus *error-recovery quality* are all worse with a generator. Deliberately no `logos` yet — the dependency buys little for a lexer this shape. |
| Parser | **Hand-written recursive descent** (+ Pratt for the expression language in Phase 2) | Error messages are a *product surface* here, not a diagnostic afterthought: they are the only input the LLM repair loop gets. Rejected ANTLR (heavy, poor recovery ergonomics) and tree-sitter as the compiler frontend (great for editors, wrong for this). Rejected LLVM outright — there is no machine-code backend; the target is source text. |
| Diagnostics | Own types now, `miette`/`ariadne` later | Codes and spans must stabilise first; the JSON shape is the contract the repair loop keys on. |
| Grammar for tooling | **tree-sitter** grammar (Phase 7) | Editor incremental parsing only — a second, deliberately redundant grammar. |
| Constrained decoding | **`llguidance`** (Rust) | Same-language integration; guarantees syntactic validity for local/open models. **Honest limitation:** hosted APIs expose no client-side CFG masking, so API arms of the benchmark use structured output plus the compiler repair loop instead. |
| CLI | `clap` derive | — |
| Serialisation | `serde` + `serde_json` | AST/IR/registry dumps and the JSON diagnostic contract. |
| Errors | `thiserror` (libs) + `anyhow` (binary) | — |
| Snapshot tests | `insta` | Codegen output is exactly the kind of thing that should be reviewed as a diff. |
| Fuzzing | `cargo-fuzz` (libFuzzer) + `proptest` | The parser must never panic on model output. |
| Benchmarks | `criterion` | Guards compile latency in the repair-loop hot path. |
| WASM | `wasm-bindgen` + `wasm-pack` | In-browser compilation for AI builders. |
| LSP | `tower-lsp` | Reuses the exact diagnostics the LLM sees, so humans and models get the same errors. |
| Emitted-code formatting | **Biome** or **oxc** (both Rust) | Fast, and keeps the toolchain single-language. Only used to verify/format generated JS/TS. |
| Task runner | `just` + plain `cargo` fallback | `just` is optional; every recipe is a one-line cargo command. |
| Benchmark harness | Rust orchestrator + Playwright (Node) + `axe-core` + Lighthouse CI | Playwright and axe have no credible Rust equivalent; the orchestrator stays Rust. |
| Token counting | Target model's own tokenizer / `count_tokens` endpoint; HF `tokenizers` for local models | `tiktoken` is an **OpenAI** tokenizer and undercounts Claude by ~15–20% on text and more on code. The `guml tokens` heuristic is explicitly labelled an estimate. |

## Deliberately deferred

- **Server, DB and auth codegen.** Attempting client + server + schema + policy in v1 is the most likely way this fails (report §12.2 challenge 5). Client-only is the honest v1 scope.
- **Incremental compilation / query engine (`salsa`).** Premature: whole-file compiles are already sub-millisecond at this scale.
- **A rich effect system.** Actions are intentionally not Turing-complete; anything more goes in a `js` escape block. That boundary is also the security boundary (A2UI's "declarative, not executable" principle, opt-in).

## Performance budget

| Operation | Budget | Why |
|---|---|---|
| `guml check` on a 200-line file | < 2 ms | Called on every repair-loop iteration and every LSP keystroke |
| `guml build` on a 200-line file | < 10 ms | Interactive builds in an AI builder's sandbox |
| Registry prompt-slice assembly | < 1 ms | Runs per generation request |

Regressions are a CI failure, not a follow-up ticket.
