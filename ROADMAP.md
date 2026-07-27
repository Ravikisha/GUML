# GUML build plan

Phases map 1:1 to the research report (`GUML-Research-Report.md` §10). Each phase has a **gate**
— do not start the next phase until the gate is met. Items already done are checked.

Legend: `[ ]` todo · `[x]` done · **GATE** = hard stop.

---

## Phase 0 — Kill-or-continue spike (2 weeks) ⚠️ HIGHEST PRIORITY

The whole 12-month program rests on one unproven assumption: *a model can produce valid,
semantically correct GUML from a spec in context, and the token saving survives real
generation.* Two weeks buys that answer. Protocol: `spec/PHASE0.md`.

- [ ] Freeze a v0.1 spec small enough to fit in context (`spec/GUML-SPEC.md`, target ≤3,000 tokens)
- [ ] Write 10 GUML specs by hand across the 6 benchmark categories (2 structure-heavy, 2 content-heavy, 6 mixed)
- [ ] Write the paired React+TS+Tailwind reference for each of the 10
- [ ] Count tokens with the **target model's own tokenizer** (not `tiktoken` — it undercounts Claude by 15–20%)
- [ ] Prompt harness: spec + registry slice + 3 examples, no compiler in the loop
- [ ] Run all 10 × {Haiku 4.5, Sonnet 5, Opus 5} × {0, 3 in-context examples}
- [ ] Score: parse validity (by eye against the grammar), semantic correctness vs the checklist, output tokens
- [ ] Record the **escape-hatch rate**: how many of the 10 needed a construct the spec cannot express
- [ ] Write up results including negative findings

**GATE** — continue only if *all three* hold:
- [ ] ≥80% of generations are parseable GUML at 3 in-context examples on Sonnet 5
- [ ] Median output-token reduction ≥3× vs the paired React on structure-heavy tasks
- [ ] Semantic correctness is not *worse* than the React baseline on the same tasks

If the gate fails, stop and publish the negative result. That is a real contribution
(report §12.5 item 10) and costs 2 weeks instead of 9 months.

---

## Phase 1 — Research and language design (4 weeks)

- [x] Literature and landscape survey (`GUML-Research-Report.md`)
- [x] Token measurement on 3 authored fixtures (`fixtures/`, report §1.5)
- [x] Strategic framing: IR + compiler study, not "a new markdown"
- [x] Decision: A2UI/MCP-UI are compile targets, not rivals
- [ ] Formal EBNF for v0.1 (`spec/grammar.ebnf` — draft exists, needs to match the parser exactly)
- [ ] Registry schema + `TagKind` semantics documented for external component packages
- [ ] Written objective function: tokens per unit of expressed intent, subject to parseability
- [ ] Log the **negative** design results (syntaxes tried and rejected, and why) — this is paper material

**GATE**
- [ ] Grammar + registry slice + 3 examples fit in ≤3,000 tokens while covering all 6 benchmark categories

---

## Phase 2 — Front end (4 weeks)

- [x] `guml-diagnostics`: spans, stable codes, JSON output, human rendering
- [x] `guml-syntax`: indentation-sensitive line lexer, prose/structure split, error recovery
- [x] `guml-ast`: typed, span-annotated, serialisable AST
- [x] `guml-parser`: recursive descent, registry-aware, collects all errors in one pass
- [x] Directives: `page`, `type`, `state`/`store`, `data` + mutations
- [x] Elements: positionals, modifiers, attributes, actions, `|` content, text-child blocks
- [x] 49 unit + integration tests green
- [ ] Expression language: real parser for bindings (`{tasks.open.count}`, `{!draft.trim()}`) instead of pass-through
- [ ] `raw` / `js` escape-hatch blocks (report §12.1 risk 5 — measure how often they are needed)
- [ ] `route`, `auth` directives
- [ ] `def` user-defined components
- [ ] Differential fuzzing: `cargo-fuzz` target that asserts the parser never panics and always terminates
- [ ] Property test: for any parse, every reported span is inside the source and points at real text

**GATE**
- [x] 100% parse of the hand-written fixture set
- [ ] Recovers and reports ≥90% of injected single-token mutations without cascading errors
- [ ] Zero panics over 1M fuzz iterations

---

## Phase 3 — Compiler core (8 weeks)

- [x] `guml-compiler` driver with one structured result (files + all diagnostics + stats)
- [x] `guml-codegen` backend trait
- [x] React backend vertical slice: containers, text, controls, state, actions, bindings
- [x] Design-system table owned by the compiler (the token lever, report §1.5)
- [x] Unsupported constructs *warn* rather than mis-lower
- [x] **JSON UI-tree backend** — the render tree behind the browser runtime, playground and live previews
- [x] **`guml` npm package**: wasm compiler + React runtime (`<Guml>`, `useGumlTree`, `useGumlRuntime`)
- [x] Expression evaluator and action lowering in the runtime (no `eval`; mirrors the React backend)
- [x] **Resolver (lite)**: bindings/actions → state, resources, repeater item fields; `GUML0033` with a suggestion
- [ ] **Semantic analyser**: type check, exhaustiveness on enumerated state domains
- [x] **Accessibility lint as hard errors** (`GUML0050`, `GUML0051`), with severity graded by what the compiler can recover
- [ ] **Desugar pass**: the conventions that make the token saving real
  - [ ] Resource layer: fetch, cancellation, retry/backoff, cache
  - [ ] Loading skeleton / empty / error slots auto-filled
  - [ ] Optimistic apply + snapshot rollback from `optimistic:`
  - [ ] `list where=` filtering, derived counts (`tasks.open.count`)
  - [ ] Declared effects (`on mount`, `on {filter}`)
  - [ ] `form` submit wiring, `tabs` from an enumerated domain
- [ ] Optimizer: dead-state elimination, binding CSE, static hoisting, registry tree-shaking
- [ ] Source maps GUML → TSX (report §12.2 challenge 2 — absence of this kills adoption)
- [ ] Snapshot tests with `insta` over every fixture

**GATE**
- [ ] All 3 fixtures compile with zero warnings
- [ ] Emitted code passes a Playwright test per fixture
- [ ] Emitted code passes `axe-core` with zero violations
- [ ] 20 additional fixtures compile and pass their tests

---

## Phase 4 — Component registry (6 weeks)

- [x] Builtin registry with `TagKind`, per-tag attrs, modifier vocabulary, typo suggestions
- [x] `guml registry --tags a,b` emits a retrieval-sized prompt block
- [ ] Grow to ~40 primitives, shadcn-backed
- [ ] Per-entry a11y contract (required accessible name, focus behaviour, roles)
- [ ] JSON registry packages + owned mirror types for external loading
- [ ] Theme packs: modifier → design-token mapping so `primary` means *the org's* primary
- [ ] Per-entry token-cost metadata (used by the optimizer and by the benchmark)
- [ ] Retrieval layer: select the registry slice from a prompt, measure prompt-cost vs vocabulary size

**GATE**
- [ ] Registry covers ≥90% of element needs across GUML-Bench without escape hatches

---

## Phase 5 — LLM integration (6 weeks)

- [x] `--format json` diagnostics designed for machine consumption
- [ ] Prompt assembly: cache-optimised layout (stable spec first, volatile task last)
- [ ] Grammar prompting harness (Wang et al., NeurIPS 2023) — the in-context DSL teaching baseline
- [ ] Grammar-constrained decoding via `llguidance` for local/open models
  - [ ] Note honestly: hosted APIs expose no client-side CFG masking, so API arms use structured output + repair instead
- [ ] Repair loop: compile → JSON diagnostics → patch → recompile, bounded at 3 rounds
- [x] Auto-apply `suggestion` fields without a model call (`applyAllSuggestions` in the JS package)
- [ ] Telemetry: tokens in/out per attempt, cached vs uncached, repair rounds, time-to-valid

**GATE**
- [ ] ≥95% valid GUML from Sonnet 5 within ≤1 repair round
- [ ] Measured spec/registry prompt tax reported separately from generation tokens (report §7.3)

---

## Phase 6 — GUML-Bench and evaluation (10 weeks)

- [ ] 150 tasks, 6 categories × 25, each with prompt + requirements checklist + reference screenshot + Playwright test
- [ ] Seed realistic structures from Design2Code's 484 curated pages
- [ ] Nine arms: B1 React · B2 HTML · B3 JSON IR · B4 TOON IR · B5 v0 · B6 human · T1 GUML · T2 +constrained · T3 +repair
- [ ] Model grid: Haiku 4.5 / Sonnet 5 / Opus 5 (capability is a first-class variable — H6)
- [ ] Metrics harness: tokens, USD, latency, parse/compile rate, Playwright pass, visual similarity, axe-core, Lighthouse, bundle size, inter-run variance
- [ ] **Edit-locality benchmark**: 50 tasks × 3 scripted modifications, measured against *diff-based* React editing, not full regeneration
- [ ] Ablation grid: spec size × examples × constrained decoding × repair rounds × model
- [ ] Human study, n≥30: pairwise code preference, readability, timed modification task
- [ ] Non-engineer study: spec readability
- [ ] Pre-register H1–H6 before running anything
- [ ] Report per-category, never a single average (the content floor makes averages misleading)
- [ ] Publish all raw generations, not just aggregates

**GATE**
- [ ] Statistically significant result on ≥3 of H1–H6, positive **or** negative

---

## Phase 7 — Second backend and papers (8 weeks)

- [ ] Svelte backend (the compile-away-the-framework / bundle-size story)
- [ ] Web Components backend (portability / embeddable story)
- [ ] A2UI + MCP-UI emitters (turns the strongest competitor into a distribution channel)
- [ ] Static HTML/CSS/JS backend (best Lighthouse numbers for the benchmark)
- [x] WASM build of the compiler (`wasm-pack`, 216 KB) — `crates/guml-wasm`, shipped as the `guml` npm package
- [ ] `tower-lsp` language server reusing the same diagnostics
- [ ] Paper 1: *How Should LLMs Represent User Interfaces?* → EMNLP/ACL or NeurIPS D&B
- [ ] Paper 2: *Convention as Compression* → ICSE/FSE
- [ ] Release GUML-Bench standalone as a dataset artifact

---

## Cross-cutting, always on

- [ ] CI: `cargo fmt --check`, `clippy -D warnings`, `cargo test`, fuzz smoke, benchmark smoke
- [ ] `criterion` benches: compile latency must stay low — the repair loop calls the compiler in a hot path
- [ ] Every claim in `README.md` traceable to a test or a measurement
- [ ] Escape-hatch rate tracked continuously; a rising number is the early warning that the expressiveness cliff is being hit
