# GUML build plan

Phases map 1:1 to the research report (`GUML-Research-Report.md` §10). Each phase has a **gate**
— do not start the next phase until the gate is met. Items already done are checked.

Legend: `[ ]` todo · `[~]` partly done, with the remainder named in the sub-items · `[x]` done ·
**GATE** = hard stop.

---

## Phase 0 — Kill-or-continue spike (2 weeks) ⚠️ HIGHEST PRIORITY

The whole 12-month program rests on one unproven assumption: *a model can produce valid,
semantically correct GUML from a spec in context, and the token saving survives real
generation.* Two weeks buys that answer. Protocol: `spec/PHASE0.md`.
Harness: `bench/phase0/` — `just phase0-verify` runs everything that needs no API key.

- [x] Freeze a v0.1 spec small enough to fit in context (`spec/GUML-SPEC.md`; largest assembled prompt ~2,970 est. tokens, budget enforced by `preflight.mjs`)
- [x] Write 10 task specs by hand across the 6 benchmark categories (2 structure-heavy, 2 content-heavy, 6 mixed) — `bench/phase0/tasks/index.mjs`
- [x] Write the paired React+TS+Tailwind reference for each of the 10 — all typecheck under `--strict`
- [x] Prompt harness: spec + registry slice + 3 examples, no compiler in the loop, stable prefix cached
- [x] Scoring harness: parse validity via `guml check --format json`, escape hatches, tokens, latency, gate check
- [x] Blind human scoresheet (arm and model stripped, deterministic shuffle) + rubric — `bench/phase0/rubric.md`
- [x] Scoring self-test over synthetic generations, so a miscount surfaces before the API spend
- [ ] Count tokens with the **target model's own tokenizer** (wired to `count_tokens` and `usage`; needs a run — never `tiktoken`)
- [ ] Run all 10 × {Haiku 4.5, Sonnet 5, Opus 5} × {0, 3 in-context examples} — 90 generations, needs an API key
- [ ] Score semantic correctness against each checklist, blind — needs a human grader
- [ ] Record the **escape-hatch rate**: how many of the 10 needed a construct the spec cannot express
- [ ] Write up `spec/phase0-results.md` including negative findings first

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
- [~] `spec/grammar.ebnf`: corrected and no longer claims to be normative — the executed
      **conformance suite** is. It had drifted provably (`page_decl` read `"page" IDENT NEWLINE` long
      after `page` gained metadata) because nothing executed it. `tests/grammar.rs` now fails when a
      directive, page attribute or escape hatch exists that the file does not mention. A machine-checked
      grammar (generate the parser from it, or the reverse) is still outstanding
- [x] **Conformance suite** (`spec/tests/*.txt`, CommonMark-style): 53 cases across syntax,
      directives, levels and escapes, each pinning source → AST fingerprint + exact diagnostic set +
      required HTML. Diagnostics are matched as a *set*, so a change cannot quietly add a warning to
      every document. The files are the authority; the Rust is checked against them, and so could a
      second implementation be
- [x] **Conformance levels**: `core` (markup — no I/O, no state, no behaviour, safe to render from an
      untrusted agent) and `app` (resources, actions, mutations, repeaters). One language, two levels,
      like CommonMark and GFM. The level is carried by the *registry*, so a core host cannot get
      behaviour because one call site forgot a flag; an app construct at the core level is `GUML0091`,
      an error rather than a silent strip. `spec/tests/levels.txt`, 9 tests
- [x] **Loadable registry**: `ComponentDef` is owned, `Registry::from_json` / `to_json` /
      `extend_from_json`, `--registry`. `BTreeMap<&'static str, &'static ComponentDef>` meant every new
      tag was a recompile of the compiler. Shadowing a builtin, an unusable tag name, and an app-level
      entry in a core host are all rejected rather than merged quietly. 14 tests
- [x] **Per-entry accessibility contract** (`A11y { requires_label, role, focusable,
      announces_state }`), so a third-party component declares what the compiler must guarantee
      instead of the promise stopping at the builtin vocabulary
- [x] **Stability policy** (`spec/STABILITY.md`) with the append-only rules *enforced*
      (`tests/stability.rs`): a tag may not change kind or level, a modifier or attribute may not
      disappear, a diagnostic id may not move. Changing one requires deleting a recorded line, which
      makes a breaking change visible in review
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
- [x] 91 unit + integration tests green
- [x] **Expression language**: real parser in `guml-syntax::expr` producing an `Expr` tree. The
      grammar existed twice in Rust; codegen now lowers the shared tree, and syntax outside the
      grammar is reported as `GUML0023` instead of forwarded into emitted JavaScript
- [x] `raw` / `js` escape-hatch blocks (report §12.1 risk 5 — measure how often they are needed) —
      body emitted verbatim, never lexed or checked; `js` hoisted into the component body, `raw
      <target>` skipped by other backends; every block reports `GUML0090`, so the rate is countable
      from `check --format json`. The JSON backend emits a placeholder instead of the body: the
      browser runtime renders documents that may come from an untrusted agent, and shipping a `js`
      body to the client would erase the "actions are not Turing-complete" security boundary.
      Fixture `d.guml` exercises it end to end (typechecks under `--strict`, and the `js` helper
      really runs during SSR)
- [x] **Prose containing `=` stays prose.** The rule was "any `=` on the line means this is
      structured", so `p Set x=1 to enable the flag.` parsed as one positional, an attribute `x=1`,
      and four discarded words — emitting `<p x={1}>Set</p>`, most of the sentence deleted and an
      invalid DOM prop added, exiting ok with a warning about the attribute. An `=` now only opens an
      attribute when the name is one the registry allows on that tag, which keeps
      `text {title} strike={done}` structured and `x=1` prose. Prose being verbatim is the
      content-floor claim; a rule that drops words from it is data loss, not compression
- [x] **`def` user-defined components**: a compile-time macro. `def stat label value` + an indented
      body; positional parameters substituted by value into bindings, attribute values and prose, with a
      literal argument becoming text and a binding staying a binding. Expanded before resolution, so
      every existing pass applies to the result and no backend knows `def` exists — which is how it works
      in the no-JavaScript HTML backend for free, and why emitted output is byte-identical to writing the
      body inline (asserted). A def inherits its conformance level from its body; there is nothing to
      declare. Five codes, `GUML0093`–`GUML0097`: shadowing, arity, recursion (named cycle path), empty
      body, and the two things expansion refuses rather than guesses — a parameter inside an action
      (the call site does not say whether the argument is a variable or a literal) and children at a call
      site. **Slots are deliberately deferred**, because adding them later is additive. 15 tests, 8
      conformance cases
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
- [x] **Static HTML backend** (`--backend html`): one file, no JavaScript. Shares `classes()` with the
      React backend, so the same GUML yields the same class strings from both — which is what makes
      "GUML is an IR" a claim about the language rather than about one emitter, and a test holds them
      to it. Everything needing a runtime is reported *and* marked `data-guml-inert` rather than
      dropped: `state` renders its initial value, a repeater renders its `empty` slot, an action
      renders disabled. `faq` lowers completely, because `<details>` needs no script. The diagnostic
      names the backend, since "not yet" and "not ever" are different messages. Also the first place
      in the pipeline that has to escape prose, which the lexer never quoted
- [x] **`guml` npm package**: wasm compiler + React runtime (`<Guml>`, `useGumlTree`, `useGumlRuntime`)
- [x] Expression evaluator and action lowering in the runtime (no `eval`; mirrors the React backend)
- [x] **Resolver (lite)**: bindings/actions → state, resources, repeater item fields; `GUML0033` with a suggestion
- [x] **Static validator** (`guml validate`, always run by `check`): unknown mutations and
      types, illegal assignment targets, enum-domain violations, dangling/duplicate anchors,
      empty repeaters, unused declarations, attribute types, method and path sanity — 17 new
      codes in the 0061–0084 range, 19 tests
  - [x] Found two silent mis-lowerings in the parser: an unknown HTTP method became `GET`, and
        a non-route path became an empty URL
  - [ ] Full type inference over expressions (needs the expression parser, Phase 2)
  - [ ] Exhaustiveness on enumerated domains beyond `tabs`/`select`
- [x] **Accessibility lint as hard errors** (`GUML0050`, `GUML0051`), with severity graded by what the compiler can recover
- [x] **Desugar pass**: the conventions that make the token saving real
  - [x] Resource layer: fetch on mount with `AbortController` cancellation
  - [ ] Resource layer: retry with backoff, response cache
  - [x] Loading skeleton / empty / error slots auto-filled (`role="alert"`, `animate-pulse`)
  - [x] Optimistic apply + snapshot rollback from `optimistic:` (prepend / replace / drop)
  - [x] `list where=` filtering via `useMemo` with a derived dep list, aggregates (`tasks.open.count`)
  - [x] `form` submit wiring with a threaded pending flag, `tabs` from an enumerated domain, `faq` as `<details>`
  - [x] Expression lowering to JS, mirrored in the TS runtime and pinned by a parity test
  - [x] Emitted TSX typechecks under `tsc --strict` (`scripts/typecheck-emitted.sh`) — found two real bugs
  - [ ] Declared effects (`on mount`, `on {expr}`) as explicit syntax
- [x] **Formatter and canonicaliser** (`guml-fmt`, `guml fmt`): line-stream rewriter below the
      parser, so it formats input that does not parse yet
  - [x] Comments and blank lines survive (the lexer drops them; the formatter recovers them)
  - [x] `--canonical`: same meaning → same bytes, for dedup and inter-run comparison
  - [x] `ast(fmt(x)) == ast(x)` enforced by test over ugly inputs and every fixture
  - [x] `--check` in CI, `--stdin` for editors, `--write` in place
  - [x] Format before parse in the repair loop, to fix whitespace errors with no model call
- [x] **Syntax classification** (`guml_fmt::highlight`, `guml highlight`): the compiler's own
      lexer and registry answer "what colour is this byte"; a regex grammar cannot, because
      prose-vs-structure depends on the tag
  - [x] Consumed by the CLI, the wasm build, the docs site and the playground
  - [x] Docs vocabulary generated from the compiler; parity checked span-for-span in CI
  - [x] Generated TextMate grammar for pre-LSP colour (`editors/vscode/syntaxes/guml.tmLanguage.json`,
        generated from `guml registry` so the vocabulary cannot drift from the compiler)
- [~] Optimizer: **dead-declaration elimination** and **static hoisting** done; **binding CSE**
      outstanding
  - [x] An unreferenced `state` or `data` is not emitted — a dead `data` cost ~25 lines of
        fetch/effect/callbacks *and* a request on mount. Liveness is `guml_ast::referenced_names`,
        the same function the validator uses for `GUML0074`/`GUML0075`, so nothing is elided that
        the author was not warned about, and a bare mention inside a `js` body keeps a declaration
        alive. Applied by both the React and JSON backends
  - [x] Enum option arrays hoisted to module scope (`const FILTER_OPTIONS = [...] as const`)
        instead of rebuilt per render
  - [x] Registry tree-shaking was already in the prompt path: `guml registry --tags` cuts the
        vocabulary block from **412 to 187 tokens** (cl100k) on a typical 10-tag task, and the
        `fullRegistry` flag keeps it ablatable
  - [ ] Binding CSE: `{tasks.open.count}` used twice is computed twice
- [x] **Source maps** GUML → TSX: Source Map v3 with VLQ mappings and inlined `sourcesContent`,
      emitted by `guml build --source-map`. Line granularity, because one GUML line becomes a
      *region* of TSX and a column claim would be invented
  - [x] Every declaration **and every element, nested ones included**. A repeater reclaims its own
        `<ul>`/`.map(`/closing lines after each child region, so a binding error inside a row template
        resolves to the row's line instead of to the `list` twenty lines above it. Before this,
        three constructs inside a `list` shared one attribution — a valid map that opens the right
        file at the wrong line
- [~] Snapshot tests with `insta`: 6 snapshots over `a`/`b`/`c`/`portfolio` plus the JSON tree and a
      readable source-map table. `d.guml` (the escape-hatch fixture) is not snapshotted yet

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
- [x] **Free repair layers** (`bench/gen/lib/pipeline.mjs`): sanitise → `guml fmt` → `guml fix`,
      all deterministic, no model call. Measured: 1 of 6 generations fixed outright, another
      from 8 errors to 2
- [x] `guml fix`: applies every unambiguous suggestion, refuses to replace a line span with a
      bare word, bounded re-check rounds
- [x] Repair loop with one model round, measured over trials — 7 of 9 attempts failed to
      improve and 2 made things worse, so an attempt is discarded unless it lowers the error
      count
- [ ] Repair loop: bounded at 3 rounds, wired into the product rather than the harness
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
- [x] WASM build of the compiler (`wasm-pack`, 298 KB) — `crates/guml-wasm`, shipped as the `guml` npm package
- [x] **`tower-lsp` language server** (`crates/guml-lsp`): diagnostics, semantic tokens,
      formatting, registry completion, hover, outline. Features are plain functions over text
      with 13 tests; the protocol layer is translation only
- [ ] Paper 1: *How Should LLMs Represent User Interfaces?* → EMNLP/ACL or NeurIPS D&B
- [ ] Paper 2: *Convention as Compression* → ICSE/FSE
- [ ] Release GUML-Bench standalone as a dataset artifact

---

## Cross-cutting, always on

- [ ] CI: `cargo fmt --check`, `clippy -D warnings`, `cargo test`, fuzz smoke, benchmark smoke
- [ ] `criterion` benches: compile latency must stay low — the repair loop calls the compiler in a hot path
- [ ] Every claim in `README.md` traceable to a test or a measurement
- [ ] Escape-hatch rate tracked continuously; a rising number is the early warning that the expressiveness cliff is being hit
