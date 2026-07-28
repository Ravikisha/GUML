# GUML — Generative UI Markup Language

A token-efficient intermediate representation and compiler for LLM-generated web applications.
A model emits ~40 lines of GUML instead of ~200 lines of React; the compiler expands it into
idiomatic React (Svelte and Web Components planned) with loading, empty, error, optimistic-rollback
and accessibility behaviour supplied by the compiler rather than by the model.

```
page Counter                                    import { useState } from "react";
state count=0
                                        →       export default function Counter() {
card sm center                                    const [count, setCount] = useState(0);
  h Clicks                                        return (
  metric {count}                                    <div className="rounded-xl border …">
  row center                                          <h2 className="text-lg font-semibold …">Clicks</h2>
    btn Decrement ghost >count--                      …
    btn Increment primary >count++                  );
                                                  }
64 tokens                                       368 tokens
```

## Status

**Pre-Phase-0.** The compiler front end, the desugar pass and a React vertical slice work end to end
and are tested (91 Rust tests, 29 runtime tests, and the emitted TSX typechecks under `--strict`).
The Phase 0 harness is built and self-tested; the generations it needs have not been run, so the
research question the project exists to answer is **still open** — see `spec/PHASE0.md`.

| Component | State |
|---|---|
| Lexer, AST, parser, diagnostics | working, tested |
| Component registry + typo suggestions | working, 27 primitives |
| React backend | containers, text, controls, state, actions, bindings, layout |
| JSON UI-tree backend | working — powers the browser runtime and playground |
| `guml` npm package (wasm + React runtime) | working — 298 KB wasm, live preview and playground |
| Resolver-lite + accessibility errors | working — `GUML0033`, `GUML0050`, `GUML0051` |
| Formatter / canonicaliser (`guml fmt`) | working — formats invalid input, AST-preserving, `--check` in CI |
| Syntax classification (`guml highlight`) | working — one classifier for CLI, wasm, docs and the LSP |
| Static validator (`guml validate`) | working — 17 semantic codes, batch mode, `--strict` for CI |
| Resources / repeaters / forms / tabs / optimistic mutations | lowered: fetch + cancel, loading, empty, error, optimistic apply and snapshot rollback |
| Expression lowering | GUML expressions → JS, mirrored in the TS runtime with a parity test |
| Expression *parsing* | still pass-through; the lowering reads paths and aggregates, not arbitrary syntax (Phase 2) |
| Phase 0 harness | built and self-tested — needs an API key to run and a human to grade |
| GUML-Bench, LLM repair loop, second backend | not started |

## Try it

```sh
cargo test --workspace
cargo run -q -p guml-cli -- build fixtures/a.guml
cargo run -q -p guml-cli -- check fixtures/b.guml --format json
cargo run -q -p guml-cli -- registry
```

## What is measured, and what is not

**Measured** (hand-authored fixtures, `cl100k_base`, `GUML-Research-Report.md` §1.5):

| Fixture | React+TS+Tailwind | GUML | Reduction |
|---|---:|---:|---:|
| counter card | 368 | 64 | 82.6% |
| task CRUD | 1,434 | 173 | 87.9% |
| landing page | 1,648 | 376 | 77.2% |

Also measured: GUML is **44% smaller than a minified JSON UI IR** for the same app; and
**232 of the landing page's 376 tokens are irreducible prose**, so compression is bounded by content
— structure-heavy artifacts approach 8×, content-heavy asymptote at 2–3×.

**Not measured yet:** whether a model can actually *produce* correct GUML, and whether correctness
improves or degrades relative to a React baseline. Those are hypotheses H1–H6 in the report. Claims
about them are labelled as hypotheses everywhere in this repo, deliberately.

**Caveat that travels with every number above:** both sides of those comparisons were authored by
the same person, and they are authored artifacts rather than model generations.

## Why this might not work

Kept in the README on purpose. GUML has zero training data by construction, and the low-resource-DSL
literature consistently reports degradation on unfamiliar languages. Anka (arXiv:2512.23214) shows a
constrained DSL beating Python by +40pp on multi-step tasks — but it is a single paper in one narrow
domain. Reconciling those two findings is the actual research contribution; Phase 0 is the two-week
experiment that tells us which side GUML lands on. Full analysis: `GUML-Research-Report.md` §12.

## Repository map

| Path | Contents |
|---|---|
| `GUML-Research-Report.md` | Full feasibility study: landscape, literature, novelty, benchmark design, critical review |
| `ROADMAP.md` | Phased build plan with gates |
| `spec/PHASE0.md` | The kill-or-continue experiment |
| `spec/TECH-STACK.md` | Stack choices and rejected alternatives |
| `spec/TOOLING.md` | Formatter, syntax classification, and the developer-tool plan |
| `spec/GUML-SPEC.md` | The language spec — also the artifact fed to the model |
| `spec/grammar.ebnf` | Normative grammar |
| `crates/` | The Rust compiler |
| `fixtures/` | Paired GUML / React / JSON-IR artifacts behind the token measurement |
| `packages/guml/` | npm package: the compiler as WebAssembly plus a React runtime |
| `bench/phase0/` | The Phase 0 harness: ten tasks, paired references, prompt assembly, scoring, blind rubric |
| `bench/gen/` | Generation test: six applications through a live model, scored on parse, validation and requirements |
| `docs/` | The documentation site (Next.js). Its own git repo; gitignored here. Code samples are generated from `fixtures/` and from the compiler. |
| `.claude/agents/` | Specialist subagents for this project |
| `.claude/skills/` | Project skills: writing GUML, compiler dev, measurement, Phase 0 |

## Positioning

Not "a new markdown." MDX, Markdoc, A2UI and Vega-Lite already occupy that framing. GUML is an
intermediate representation plus a compiler, and the contribution is the **measurement**: where the
token/accuracy frontier sits for LLM-generated UI, and when a purpose-built DSL beats a
high-resource general-purpose language. Google's A2UI is treated as a compile target, not a rival.

License: Apache-2.0.
