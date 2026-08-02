# GUML: An AI-Native Intermediate Representation and Compiler for Token-Efficient Web Application Generation

**Research feasibility report and design document**
Prepared 2026-07-27 · Working name: **GUML** (Generative UI Markup Language)

---

## 0. How to read this document

Part 1–4 establish whether the idea is publishable and novel. Part 5–7 are the concrete design. Part 8–10 are the methodology and plan. Part 11–12 are the market case and the honest attack on the idea. Section 13 is the verdict.

Two things in this report are original measurements rather than restatements of prior work:

1. **§1.5 / §7.2 — a measured token study.** I hand-wrote three functionally equivalent app specs in idiomatic React+TypeScript+Tailwind and in the proposed GUML syntax, then tokenized both with `cl100k_base` and `o200k_base`. The reduction is 77–88% (4.4×–8.2×). I also measured GUML against a compact JSON UI IR (A2UI/SDUI-style) and against the *irreducible human copy* floor. Fixtures are reproducible; see §8.6.
2. **§4 / §12 — the central research tension**, which the literature does not currently resolve: a purpose-built DSL simultaneously (a) reduces output tokens and constrains the hypothesis space, and (b) moves the model off-distribution into low-resource-language territory where measured accuracy drops. Both effects are documented in separate papers with opposite conclusions. Nobody has characterized *where the crossover is*. That question is the paper.

---

## 1. Part 1 — Problem definition and motivation

### 1.1 The fundamental problem

LLMs generating web applications currently emit **the final artifact** (React/TSX/Tailwind) rather than **a specification of the artifact**. The final artifact is dominated by content that is:

- **Mechanically derivable** — `useState` plumbing, `onChange={(e) => setX(e.target.value)}`, fetch/try/catch/finally/loading/error scaffolds, `key={}` props, ARIA wiring.
- **Presentationally verbose** — Tailwind utility strings. In my landing-page fixture, class attributes alone account for roughly a third of all tokens.
- **Framework-ceremonial** — imports, type declarations, closing tags, component boilerplate.

None of that carries *intent*. It is the deterministic expansion of a much smaller set of decisions. The model is being paid — in tokens, latency, and error probability — to re-derive a compiler's output by hand, every time.

The problem GUML addresses: **there is no widely used representation that sits between "natural language prompt" and "framework source code."** Every AI builder today jumps the whole distance in one hop.

### 1.2 Why current approaches are inefficient for AI-generated software

| Inefficiency | Mechanism |
|---|---|
| Output-token dominance | Output tokens cost 5× input tokens on current frontier models (Claude Opus 5: $5/MTok in, $25/MTok out; Sonnet 5: $3/$15; Haiku 4.5: $1/$5). Cost is concentrated exactly where the redundancy is. |
| Latency is output-bound | Wall-clock generation time is essentially linear in output tokens. Prompt processing is parallel; decoding is sequential. An 8× output reduction is an ~8× reduction in time-to-complete for the generation phase — this is the strongest practical argument for the idea, stronger than cost. |
| Error surface scales with tokens | Every emitted token is an opportunity for a hallucinated import, a mismatched brace, a wrong prop name, a missing `key`. Fewer tokens, mechanically fewer defect sites. |
| Context exhaustion in agent loops | An agent iterating on a 6,000-token component holds the whole thing in context across turns. At 400 tokens it holds ten variants for the same budget. |
| Re-generation churn | "Make the button secondary" currently means re-emitting or diff-patching a Tailwind class soup. In a semantic representation it is a one-token edit. |

### 1.3 Why existing programming languages are not optimized for LLM generation

General-purpose languages were designed under a different cost function. Their design pressures were: human readability at scale, incremental compilation, static analysis, backwards compatibility, and expressive generality. **Token density was never a design goal**, because human typing cost and machine parsing cost do not resemble autoregressive decoding cost.

Concretely:

- **Redundancy is a feature for humans, a tax for LLMs.** Explicit imports, closing tags, and repeated type annotations aid human comprehension and tooling; they are pure token cost for a generator that has the whole file in one context.
- **Generality is unbounded where the domain is narrow.** React can express anything, so the model must *choose* among many encodings of the same UI. That choice variance is where inconsistency and bugs come from. Web app UI is, empirically, a narrow domain of ~30 recurring component archetypes and ~6 recurring data-flow patterns.
- **The styling layer is unpriced.** Tailwind optimized for human authoring ergonomics and produced the single most token-expensive convention in modern frontend work. `className="mt-6 flex items-center justify-center gap-3"` is ~14 tokens expressing "row, centered, normal gap" — 3 tokens of intent.

### 1.4 Why token efficiency matters, per constituency

- **AI coding assistants.** Latency is the dominant UX variable. Users abandon at ~30s. Output reduction is the only lever that scales linearly with perceived speed.
- **Autonomous software agents.** Agents run dozens-to-hundreds of turns. Per-turn output size compounds into context pressure, compaction events, and lost coherence. Smaller artifacts mean longer coherent horizons.
- **AI application builders (v0 / Lovable / Bolt class).** Their gross margin *is* inference cost. At ~$400M ARR (Lovable, Feb 2026) and ~$40M (Bolt), a 5× reduction in generation tokens is a material P&L line, not an optimization.
- **Enterprise AI systems.** Two additional pressures: (a) auditability — a 40-line declarative spec is reviewable by a non-engineer stakeholder, 400 lines of TSX is not; (b) governance — a compiler can *guarantee* design-system compliance, accessibility attributes, and CSP-safe output in a way "please follow our design system" in a prompt cannot.

### 1.5 Quantified benefits (measured, not estimated)

Three fixtures, hand-written both ways, tokenized with `tiktoken`:

| Fixture | React+TS+Tailwind | GUML | Reduction | Ratio |
|---|---:|---:|---:|---:|
| A — Counter card (state, 3 actions, disabled logic) | 368 | 64 | **82.6%** | 5.75× |
| B — Task CRUD (fetch, POST/PATCH/DELETE, optimistic updates, filter, loading/empty/error states) | 1,441 | 178 | **87.6%** | 8.10× |
| C — Marketing landing page (nav, hero, 3 features, 3 pricing tiers, FAQ accordion, footer) | 1,648 | 376 | **77.2%** | 4.38× |
| **Total** | **3,457** | **618** | **82.1%** | 5.59× |

`o200k_base` gives the same result within 1pp (83.2% / 87.8% / 77.9%), so this is not a tokenizer artifact.

> **Correction, recorded rather than quietly fixed.** Fixture B was published here as 175 tokens.
> Recounting the committed file with `cl100k_base` gives **182** — the original figure was wrong by
> 7 tokens and no one caught it, which is exactly the failure this project accuses other people's
> headline claims of. Running `guml fmt` over the fixture removed 12 bytes of hand-inserted column
> padding, and the file now measures **173**. The table above is the post-format count. The ratio
> moved in GUML's favour (8.19× → 8.29×), which is the direction that deserves the most suspicion,
> so the arithmetic is reproducible: `python -c "import tiktoken; print(len(tiktoken.get_encoding('cl100k_base').encode(open('fixtures/b.guml').read())))"`.

Two further measurements that matter more than the headline:

**GUML vs. a compact JSON IR.** I encoded fixture B as a declarative JSON UI spec in the style of A2UI / server-driven UI. Minified, it is **315 tokens vs GUML's 173 — GUML is 45% smaller.** JSON's structural overhead (quotes, braces, repeated keys) is real and large. This matters because "just emit JSON" is the first objection any reviewer will raise.

**The content floor.** In fixture C, 232 of GUML's 376 tokens are *irreducible human copy* — headlines, feature descriptions, FAQ answers. Structural overhead is only 144 tokens, versus ~1,416 for React. So:

> **Compression is bounded by prose content.** On structure-heavy artifacts (dashboards, CRUD, forms) reduction approaches 8–10×. On content-heavy artifacts (marketing pages, docs) it asymptotes toward 2–3× because the copy is the payload. **Any benchmark that reports a single average number is misleading.** This is a finding, and it should be stated as one rather than hidden.

**Cost and latency, worked.** Fixture B at Opus 5 output pricing ($25/MTok): 1,441 tokens = $0.0360; 178 tokens = $0.0045. Savings $0.0316/generation. The GUML language spec must be in context — assume a generous 3,000 tokens; under prompt caching it reads at ~0.1× input rate ($0.50/MTok effective) = $0.0015/request. **The spec amortizes on the first request and is ~20× cheaper than the savings it unlocks.** At 60 output tok/s, generation time drops from ~24s to ~3s.

Caveats stated plainly: I authored both sides, so favorable bias in the GUML encoding is possible; these are *authored* artifacts, not *model-generated* ones; and none of this measures whether a model can actually *produce* correct GUML. That last question is Part 8.

---

## 2. Part 2 — Existing technology landscape

### 2.1 Markdown and its extensions

| System | Architecture | Capability ceiling | Why it does not solve this |
|---|---|---|---|
| **Markdown / CommonMark** | Line-oriented text → HTML tree. No semantics beyond block/inline structure. | Static documents. | No state, no events, no data. Extremely token-efficient for prose; expresses zero application logic. |
| **GitHub Flavored Markdown** | CommonMark + tables, task lists, autolinks. | Static + light structure. | Same ceiling; extensions are presentational. |
| **markdown-it** | Pluggable tokenizer + renderer pipeline. | Whatever plugins add. | It's an *implementation strategy*, not a language design. Useful as a reference architecture for GUML's plugin layer. |
| **MDX** | Markdown superset that parses JSX inline; compiles to a JS module. | Full React power. | **This is the anti-pattern.** MDX solves *composition*, not *density* — the moment you need interactivity you drop into raw JSX and pay full token cost. MDX makes prose+components ergonomic; it does nothing for token efficiency or generation reliability. |
| **Markdoc** (Stripe) | Markdown + a *closed, schema-validated tag set*; renders to an AST you transform. | Content sites with sanctioned interactive components. | **The closest philosophical relative.** Markdoc's key insight — a validated closed tag vocabulary rather than arbitrary code — is exactly right and is a direct design ancestor of GUML. But Markdoc deliberately has no state model, no event model, and no data model; it is a documentation authoring system, and its tag syntax (`{% tag attr="v" %}...{% /tag %}`) is token-expensive. |

**Conclusion:** the markdown lineage supplies the *ergonomic* precedent (line-oriented, low-ceremony, closed vocabulary) and none of the *semantic* machinery. Positioning GUML as "the next markdown" is a marketing frame, not a technical one, and as noted in §12 it is a liability with reviewers.

### 2.2 Component-based UI systems

| System | Syntax complexity | Token efficiency | Compilation | Runtime cost | AI-generation suitability |
|---|---|---|---|---|---|
| **React JSX** | High (hooks, deps arrays, closures) | Worst measured | Babel/SWC → `createElement` | VDOM diffing | **Highest training-data density** — this is its only real advantage, and it is a large one |
| **Vue templates** | Medium (directives: `v-if`, `v-for`, `v-model`) | Better — `v-model` collapses a 3-token React pattern to 1 | Template → render fn | VDOM + reactivity | Good; directives are *evidence that declarative shorthand works* |
| **Svelte** | Low-medium (`$:`, `bind:`, `{#each}`) | Good | Compile-to-imperative-DOM | Near-zero runtime | Strong compile target; less training data than React |
| **SolidJS** | Medium (JSX + signals) | ≈ React | JSX → fine-grained reactive | Minimal | Fine target; signal semantics are subtle for models |
| **Web Components** | High (class boilerplate, lifecycle, shadow DOM) | Poor | None required | Native | Best *portability* target, worst authoring density |
| **Angular templates** | High (modules, DI, `*ngIf`) | Poor | AOT | Moderate | Poor fit |
| **Marko** | Low | Good | Streaming compiler | Very low | Instructive prior art in compile-away-the-framework; tiny ecosystem |

**Reading:** Vue's directives and Svelte's `bind:` prove the industry already accepts *declarative collapse of common patterns*. GUML is that idea taken to its limit, plus a data/effect layer.

### 2.3 Declarative UI languages (the real design ancestors)

| System | Key idea GUML should steal |
|---|---|
| **SwiftUI** | Trailing-closure hierarchy + *modifier chains*. `.padding().background(.blue)` is semantic, not utility-class. Layout containers (`VStack`/`HStack`) as first-class primitives instead of flex incantations. |
| **Jetpack Compose** | `@Composable` functions with hoisted state; `remember`/`mutableStateOf` makes the state graph explicit and local. |
| **Flutter** | Everything-is-a-widget uniformity; a single composition rule to learn. Verbose in practice — a cautionary tale about nesting depth. |
| **QML** | **The single most relevant precedent.** Declarative object tree + *property bindings* (`width: parent.width / 2`) that auto-track dependencies, with JS escape hatches. QML is essentially "what if a UI markup language had a reactive expression language." GUML's `{expr}` bindings are QML's model. |
| **XAML** | Data binding + templating + resource dictionaries — and a warning: XML verbosity made it one of the least token-efficient UI languages ever shipped. |

The synthesis: **SwiftUI's semantic modifiers + QML's property bindings + Markdoc's closed validated vocabulary + indentation instead of closing tags.**

### 2.4 AI UI generation systems (what production actually does)

| System | Generation approach | Uses an IR? | Token-efficiency posture |
|---|---|---|---|
| **Vercel v0** | Prompt → React/Next + Tailwind + shadcn/ui, streamed. Component library is passed as context so the model *assembles primitives* rather than free-coding. | Partial — a shadcn component tree is a *de facto* IR, streamed and mounted as real React, exportable as code | Mitigates via component reuse; still emits full TSX |
| **Lovable** | Full-stack app scaffolding with Supabase; multi-file project generation | No public IR | Full source emission |
| **Bolt.new** | In-browser WebContainer; model writes real project files | No | Full source emission; famously token-hungry (its own docs coach users on token conservation) |
| **Replit Agent** | Multi-file agent with execution loop | No | Full source |
| **Cursor / Claude Code** | Diff-based editing over an existing repo | No | **Diff-based editing is the strongest existing mitigation** and a real competitor to GUML's thesis (see §12.4) |
| **OpenAI Codex-class agents** | Repo-scale patching | No | Full source in patches |

**The pattern worth noting:** the industry is already converging *toward* constrained assembly — v0 constrains the model to shadcn primitives; MCP-UI and A2UI (below) constrain agents to a host-approved component catalog. Nobody has taken the next step of giving that constrained vocabulary a *token-optimized surface syntax and a real compiler*.

### 2.5 The 2025–2026 agent-UI protocol wave (critical prior art)

This is the most important section for novelty assessment, and it is recent enough to be easy to miss.

- **A2UI** (Google, open-sourced 2025-12-15, with AG-UI/CopilotKit, Opal, Gemini Enterprise, and Flutter teams). A standardized **JSON format plus client renderers** so agents can emit UI that any host renders natively. Explicit design principles: *declarative data, not executable code*; **"LLM-friendly and incrementally updateable"**; framework-agnostic. Payload is a **flat list of components with ID references**; clients hold a **pre-approved component catalog** and agents may only request from it.
- **MCP-UI.** Extends MCP's embedded-resources spec with a `UIResource` interface; hosts render in sandboxed iframes or via **remote DOM** (UI + events described in JS, rendered with host-native components — currently React and Web Components).
- **AG-UI** (CopilotKit, early 2025). Bidirectional event-driven agent↔frontend protocol.
- **Server-driven UI**, the decade-old industrial ancestor: **Airbnb's Ghost Platform** (Sections + Screens over one shared GraphQL schema, native renderers in TypeScript/Swift/Kotlin), plus Lyft, Netflix, DoorDash, Nubank, Yelp. UI-as-data is a *solved, deployed* pattern at scale.

**What this means for GUML, stated bluntly:** the *concept* "agent emits a declarative UI description against a closed component catalog" is **already standardized and shipping**, with Google's weight behind it. GUML cannot claim that concept. What A2UI/MCP-UI/SDUI all share, and what remains open:

1. They are **JSON**. My measurement: JSON costs **~1.8× the tokens** of a line-oriented syntax for identical semantics. They claim LLM-friendliness without optimizing the token surface.
2. They render **ephemeral agent UI inside a host chat client**. They do not **compile to a deployable standalone application**. There is no `guml build` producing a React/Svelte project you own.
3. They have **no application logic layer** — no client state machine, no derived state, no optimistic mutation semantics, no routing, no auth model. A2UI is deliberately non-executable for security; that is correct for untrusted remote agents and insufficient for "build me an app."
4. There is **no published evaluation** of tokens-vs-accuracy for these formats against a code baseline.

That is the gap. It is narrower than the original framing assumed, and it is real.

---

## 3. Part 3 — Academic literature review

Papers below were located and verified during this research. Where I could not verify authorship, I say so rather than fabricating it.

### 3.1 The two papers that define the core tension

**(A) Anka: A Domain-Specific Language for Reliable LLM Code Generation** — Saif Khalfan Saif Al Mazrouei, arXiv:2512.23214 (Dec 2025).
*Problem:* general-purpose languages' flexibility causes systematic LLM errors on multi-step tasks via implicit state management and operation sequencing.
*Method:* a DSL for data-transformation pipelines with deliberately explicit, constrained syntax. 100 benchmark problems; **no prior model exposure to Anka**.
*Results:* Claude 3.5 Haiku — **99.9% parse success, 95.8% task accuracy**; on multi-step pipelines **100% (Anka) vs 60% (Python)** = **+40pp**. Cross-validated on GPT-4o-mini: **+26.7pp** on multi-step.
*Limitations:* single narrow domain (data pipelines, not UI); no token-count reporting; single-author, small benchmark; no human evaluation.
*Why it matters here:* this is the **existence proof** that a purpose-built constrained DSL can beat Python *for a model that has never seen the DSL*, on precisely the failure mode (multi-step state) that UI generation is full of. It is the strongest single citation for the GUML thesis.

**(B) A Survey on LLM-based Code Generation for Low-Resource and Domain-Specific Programming Languages** — arXiv:2410.03981.
*Finding:* LLM performance **degrades** on low-resource languages and DSLs; data scarcity and specialized syntax poorly represented in pretraining corpora are the causes. Corroborated by *DSL or Code? Evaluating LLM-Generated Algebraic Specifications: A Case Study in Optimization at Kinaxis* (arXiv:2601.00469), which finds LLM-generated DSL models **less accurate than mainstream-language code** due to Python's corpus dominance; and *A framework for assessing code generation of constraint DSLs with LLMs* (arXiv:2603.05278 / *Information and Software Technology*), same direction.
*Why it matters:* this is the reviewer's primary weapon against the idea, and it is well-supported.

**Reconciling A and B is the research contribution.** They are not contradictory; they measure different regimes. Hypothesis (testable, §8):

> A purpose-built DSL beats a high-resource general-purpose language when (i) the grammar is small enough to fit in-context, (ii) the target domain is *conventional* — most decisions have one right answer the compiler can supply, and (iii) the task's dominant failure mode is multi-step state coordination rather than unfamiliar syntax. It loses when the grammar is large, the domain needs open-ended escape hatches, or the model must recall DSL idioms not present in context.

UI generation sits squarely in the favorable regime on (ii) and (iii). Testing (i) empirically — how big can the spec get before in-context learning degrades — is a publishable result on its own.

### 3.2 Intermediate representations for LLM app generation

**Athena: Intermediate Representations for Iterative Scaffolded App Generation with an LLM** — Jazbo Beason, Ruijia Cheng, Eldon Schoop, Jeffrey Nichols. arXiv:2508.20263; ACM IUI 2026 (DOI 10.1145/3742413.3789133).
*Problem:* a single prompt cannot carry enough detail for a complete UI; single-shot output is one large unmaintainable file.
*Method:* three shared human-and-LLM intermediate representations — **app storyboard** (screen structure), **GUI skeletons** (per-screen layout), **data model** — used as scaffolding for generation.
*Results:* **75% of participants preferred it** over a chatbot baseline for prototyping; produces organized multi-file code with fewer errors.
*Limitations:* IRs are *scaffolding for a human-in-the-loop design tool*, not a compiled language; no compiler, no formal grammar, no token-efficiency claim, no automated quality metrics.
*Differentiation:* **this is the closest academic prior art and must be cited in the first paragraph of any GUML paper.** Athena establishes "IRs help LLM app generation" as an accepted HCI result. GUML's differentiation is: formal grammar + real compiler + token-efficiency as the objective + automated benchmark, versus Athena's design-process framing.

**Generative Interfaces for Language Models** — Jiaqi Chen, Yanzhe Zhang, Yutong Zhang, Yijia Shao, Diyi Yang. arXiv:2508.19227; ACL 2026 Findings. Uses "structured interface-specific representations and iterative refinements" to turn queries into task-specific UIs; **up to 72% improvement in human preference** over conversational interfaces. Establishes that generated UI beats chat for information-dense tasks — the *demand-side* justification.

**Generative and Malleable User Interfaces with Generative and Evolving Task-Driven Data Model** — Yining Cao, Peiling Jiang, Haijun Xia. CHI 2025 (DOI 10.1145/3706598.3713285); system named **Jelly**. Pipeline: prompt → LLM-generated **task-driven data model** → **UI specification** → concrete interface, with NL modification. Strong precedent for *data-model-first* generation — directly informs GUML's `type` + `data` blocks.

**SpecifyUI** — arXiv:2509.07334 (Sept 2025). Extracts a structured **SPEC** from UI references via segmentation + VLMs; composes across sources; targeted edits at global/regional/component level; multi-agent generator renders SPEC → design. Precedent for *spec as the editable unit*, which is GUML's iteration story.

**Bridging Design and Development with Automated Declarative UI Code Generation (DeclarUI)** — Ting Zhou, Yanjie Zhao, Xinyi Hou, Xiaoyu Sun, Kai Chen, Haoyu Wang. arXiv:2409.11667. CV + MLLM + **iterative compiler-driven refinement**; Page Transition Graphs for inter-page relations. React Native: **96.8% PTG coverage, 98% compilation success, +123% PTG coverage / +55% visual similarity / +29% compilation success over baseline MLLMs**; also Flutter and ArkUI.
*Directly transferable:* the **compiler-feedback repair loop** is the single most valuable borrowed mechanism, and it is what makes GUML's reliability claim credible rather than hopeful.

### 3.3 Constrained generation and grammar conditioning

- **Grammar Prompting for Domain-Specific Language Generation with LLMs** — Bailin Wang, Zi Wang, Xuezhi Wang, Yuan Cao, Rif A. Saurous, Yoon Kim (MIT + Google). arXiv:2305.19234, **NeurIPS 2023**. Each in-context example is augmented with a **minimal specialized BNF grammar** sufficient to generate that output; at inference the model first predicts a grammar, then generates conforming to it. Evaluated on semantic parsing (SMCalFlow, Overnight, GeoQuery), PDDL planning, molecule generation. **This is the canonical method for teaching an unfamiliar DSL in-context and is a required baseline for GUML.**
- **SynCode** (arXiv:2403.01632) — DFA-based masks precomputed from a CFG; guarantees syntactic validity across languages.
- **Domino / Guiding LLMs The Right Way** (arXiv:2403.06988) — fast non-invasive constrained decoding; handles subword/grammar token misalignment with precomputation + speculative decoding.
- **CRANE** (arXiv:2502.09061) — alternates *unconstrained* generation for reasoning with *constrained* generation for structure, recovering the reasoning loss that naive constraint imposes.
- **TreeCoder** (arXiv:2511.22277) — systematic exploration of decoding/constraint configurations for code generation.
- **Grammar-Constrained Decoding Makes LLMs Better...** — ACL 2025 Industry Track.
- **Leveraging LLMs for Multi-File DSL Code Generation: An Industrial Case Study** (arXiv:2604.24678) — multi-file DSL generation in practice.
- **From Text to DSL: Evaluating Grammar-Based Model Generation Using Open LLMs** (arXiv:2605.15865, SOMET 2024).
- **DSL-Xpert / DSL-Xpert 2.0** (*JSS*, S0950584925002939) — LLM-driven generic DSL code generation; **Microsoft `dsl-copilot`** — vetted recipe for teaching an LLM a proprietary DSL.

**Combined implication:** GUML can *guarantee* 100% parse validity via grammar-constrained decoding (SynCode/Domino), and *teach* the grammar in-context (grammar prompting), and *repair* semantic errors via compiler feedback (DeclarUI). The reliability story is assembled from proven parts. **But see CRANE and §3.5 — constraint is not free.**

### 3.4 Format, serialization, and token efficiency

- **Let Me Speak Freely? A Study on the Impact of Format Restrictions on Performance of LLMs** — Tam et al., **EMNLP 2024** (Industry Track). Format-restricting instructions (JSON/XML/YAML) **degrade reasoning**, reportedly on the order of 10–15% on affected tasks; stricter constraints, larger declines. A key mechanism is **field ordering** — if the answer field precedes the reasoning field, the model commits before reasoning.
- **Capacity, Not Format: Rethinking Structured Reasoning Failures** (arXiv:2606.09410) — important refinement: the same schema is **absorbed at no cost by capable models while severely degrading weaker ones on the same task**. The variable is *spare capacity*, not format per se.
- **TOON (Token-Oriented Object Notation)** — `toonformat.dev`, `github.com/toon-format/toon`. YAML-style indentation for nesting + CSV-style tabular rows for uniform data. Community benchmarks: **72.2% accuracy vs JSON's 71.4% at 42.6% fewer tokens** on 244 retrieval questions across 4 models; **29.2 acc%/1K tok vs 23.8 (compact JSON), 20.1 (YAML), 16.6 (pretty JSON)**.
  **But:** *Token-Oriented Object Notation vs JSON: A Benchmark of Plain and Constrained Decoding Generation* — Ivan Matveev, arXiv:2603.03306 (Feb 2026) — finds **plain JSON has the best one-shot and final accuracy**; constrained decoding has the lowest token usage at slightly reduced accuracy; TOON suffers a **"prompt tax"** of instructional overhead in short contexts and only pays off past a structural-complexity threshold. Independent tests on *nested* data rank TOON **last** (43.1%) behind JSON (50.3%), Markdown (54.3%), YAML (62.1%).
- **Less Is More: DocString Compression in Code Generation** — *ACM TOSEM* (10.1145/3735636). Compression research generally: ~1–2% accuracy loss at moderate ratios; aggressive pruning removes load-bearing information.

**Implication, and it is uncomfortable:** the empirical record on "novel token-efficient formats" is **genuinely mixed**. TOON is the closest analogue to GUML's bet and its benchmarks do not cleanly replicate. GUML must therefore (a) benchmark against JSON honestly, (b) report the prompt tax explicitly, and (c) expect the win to be *conditional*. Claiming an unconditional win will not survive review.

### 3.5 UI-generation benchmarks and NL2VIS

- **Design2Code: Benchmarking Multimodal Code Generation for Automated Front-End Engineering** (arXiv:2403.03163; `salt-nlp.github.io/Design2Code`) — **484 manually curated real-world webpages**, automatic metrics + human pairwise ranking and direct assessment; evaluates GPT-4o, Gemini, LLaVA, CogAgent, WebSight VLMs.
- **WebSight** (arXiv:2403.09029) — **2M synthetic HTML/screenshot pairs**; scale over realism.
- **Web2Code** — NeurIPS 2024 Datasets & Benchmarks.
- **Vega-Lite as the canonical success case.** A declarative grammar that LLMs target instead of emitting plotting code: **LIDA** (arXiv:2303.02927, grammar-agnostic viz generation), **NL4DV**, **VegaChat** (arXiv:2601.15385 — generates declarative Vega-Lite specs, citing *security benefits and easier interactive modification*, and **significantly reducing invalid/empty visualizations vs LIDA**), **Raiven: LLM-Based Visualization Authoring via Domain-Specific Language Mediation** (arXiv:2604.10008 — "DSL mediation" is precisely GUML's thesis, one domain over), **VL2NL** (arXiv:2309.10245, CHI 2024).

> **Vega-Lite is the strongest analogy available and should anchor the paper's argument.** It proves the pattern works in production and in the literature: a compact declarative DSL, targeted by LLMs, in a narrow conventional domain, yielding fewer invalid outputs than free-form code generation. GUML is the same move applied to interactive applications instead of charts. Raiven even names the mechanism — DSL mediation.

### 3.6 Adjacent but relevant

- **Oracular Programming: A Modular Foundation for Building LLM-Enabled Software** (arXiv:2502.05310) — programming-language-theoretic framing for LLM-in-the-loop software.
- **ShortCoder: Knowledge-Augmented Syntax Optimization for Token-Efficient Code Generation** (arXiv:2601.09703).
- **When to Stop? Towards Efficient Code Generation in LLMs with Excess Token Prevention** (arXiv:2407.20042).
- **Biscuit: Scaffolding LLM-Generated Code with Ephemeral UIs in Computational Notebooks** (arXiv:2404.07387).
- **Component retrieval / RAG for UI:** shadcn/ui as an AI-oriented design system — full component schema passed as agent context, model *chooses and assembles primitives*, output is a structured primitive tree streamed and mounted as React and exportable as code; shadcn MCP servers for component discovery/generation. This is the retrieval-augmented component-registry pattern GUML needs for its stdlib.

---

## 4. Part 4 — Novelty analysis

### A. Already solved — do not claim these

| Claim | Prior art that owns it |
|---|---|
| Declarative UI description emitted by an LLM | A2UI, MCP-UI, AG-UI; Jelly (CHI'25); SpecifyUI |
| UI-as-data, host-side renderers, closed component catalog | Airbnb Ghost Platform, Lyft, Netflix SDUI (2021–2025); A2UI |
| Intermediate representations improve LLM app generation | **Athena** (IUI 2026) |
| Compact declarative DSL as an LLM target in a narrow domain | **Vega-Lite** + LIDA/NL4DV/VegaChat/Raiven |
| Teaching an LLM an unseen DSL in-context via grammar | **Grammar Prompting** (NeurIPS 2023) |
| Guaranteeing syntactic validity of generated DSL | SynCode, Domino, CRANE, xgrammar |
| Compiler-feedback repair loops for generated UI | **DeclarUI** (arXiv:2409.11667) |
| Compiling a high-level language to multiple frontend frameworks | Svelte, Marko, Mitosis, Stencil |
| Token-efficient serialization formats | TOON, and a mixed empirical record |
| Component-library-constrained generation | v0 + shadcn; A2UI catalogs |
| Declarative UI grammars with reactive bindings | QML, SwiftUI, Compose |

That is a long list. **The unqualified framing "design an AI-native markdown for interactive web apps" is not novel in 2026.**

### B. Partially solved — contestable ground

1. **Token efficiency of a UI IR has never been measured against a code baseline.** A2UI claims LLM-friendliness with no numbers. Design2Code/WebSight measure *quality*, never *tokens*. My §1.5 measurement appears to be the first of its kind for UI IRs. **This is a real, defensible, easily-verified contribution.**
2. **Application logic in an agent-emitted IR.** A2UI is deliberately non-executable; MCP-UI's remote DOM is executable but framework-bound and host-scoped. Nothing standardized expresses client state machines, derived state, optimistic mutations, routing, or auth declaratively for agent emission.
3. **Deployable output.** Every agent-UI protocol renders ephemerally in a host. None compiles to a project you own and deploy.
4. **Where the DSL crossover lies.** Anka (+40pp) vs the low-resource survey (degradation) is unresolved. Nobody has characterized the boundary.

### C. Genuinely open research contributions

Ranked by publishability:

1. **★ The token/accuracy Pareto frontier for UI generation representations.** Systematically compare: raw React · raw HTML · JSON UI IR (A2UI-shaped) · TOON-encoded IR · GUML. Measure output tokens, compile/parse validity, functional correctness, visual fidelity, and accessibility, across model scales (Haiku 4.5 → Sonnet 5 → Opus 5) and with/without grammar-constrained decoding. **This is a clean empirical paper with a real result regardless of which representation wins**, and it directly extends both Tam et al. and *Capacity, Not Format* into a new domain. Ship this first.
2. **★ Characterizing the DSL crossover.** Ablate grammar size (200 → 5,000 spec tokens), in-context-example count, model capability, and task type (structure-heavy vs content-heavy). Produce the decision rule for *when* a DSL beats a general-purpose language. Reconciles Anka with the low-resource survey. High citation potential — it answers a question the field is currently confused about.
3. **★ Semantic-density-optimized language design as a discipline.** A design methodology with a measurable objective function: *tokens per unit of expressed intent*, subject to model-parseability constraints. Includes the negative results (which compressions *hurt* accuracy — e.g. sigil-heavy syntax that fragments badly under BPE). Design-space papers are publishable at PL and HCI venues.
4. **Convention-as-compression.** Formalize the observation that a compiler supplying defaults (loading states, error boundaries, optimistic rollback, focus management, ARIA labels, contrast-safe palettes) removes those tokens from the model's burden *and* guarantees correctness properties the model was previously asked to remember. Measurable as: accessibility score and error-state coverage of GUML output vs LLM-authored React, at 8× fewer generated tokens.
5. **The content floor.** My measurement that irreducible prose bounds compression (232/376 tokens on a landing page) is a small but genuinely novel and useful result, and it reframes how the field should report compression numbers.
6. **Edit-locality as a second-order metric.** How many tokens does a *modification* cost? "Make the CTA secondary and add a testimonial section" in GUML vs a React diff. Agents modify far more than they create; nobody has benchmarked representation choice for *editing*. Possibly the most industrially valuable contribution and the least explored.
7. **Bidirectional lifting (React → GUML).** Decompilation enables adoption on existing codebases and creates the training corpus that solves the low-resource problem. Hard, valuable, and a natural follow-up paper.

**Novelty verdict:** the *language* is not novel. The *measurement*, the *crossover characterization*, and the *convention-as-correctness-guarantee* framing are. Reposition accordingly (§12.5).

---

## 5. Part 5 — Proposed language design

### 5.1 Design objective, stated formally

Minimize expected output tokens subject to: (1) unambiguous parse under a small CFG; (2) high in-context learnability from a ≤3,000-token spec; (3) no expressiveness cliff — every construct has a typed escape hatch; (4) all conventional behavior (states, a11y, error handling) supplied by the compiler, never by the model.

### 5.2 Design rules

1. **Indentation replaces closing tags.** Saves ~1 token per element and removes a whole class of mismatch errors.
2. **Semantic modifiers, not utility classes.** `primary`, `quiet`, `sm`, `center` — the compiler owns the design system. This is the single largest lever; Tailwind class strings are ~⅓ of React tokens in my fixtures.
3. **Positional-then-named arguments.** `btn Decrement ghost disabled={!count}` — the first bare token is the label.
4. **One-character sigils for the highest-frequency operations**, chosen for BPE-friendliness: `>` action, `{}` binding, `|` content separator, `#` anchor, `/` route.
5. **Convention over configuration, aggressively.** A `list` bound to a `data` resource *automatically* gets loading skeleton, empty state, error banner, and optimistic rollback. The model writes zero tokens for any of it.
6. **No imports. No types unless they add semantics.** Component vocabulary is ambient; the registry is in context.
7. **Escape hatch at every level.** `raw` blocks (verbatim target-framework code), `js` expression blocks, `style` overrides. Non-negotiable — see §12.3.

### 5.3 Comparison, side by side

Traditional React:

```jsx
function Dashboard() {
  const [count, setCount] = useState(0);
  return (
    <button
      className="rounded-md bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800"
      onClick={() => setCount(count + 1)}
    >
      {count}
    </button>
  );
}
```

GUML:

```
page Dashboard
state count=0

btn {count} primary >count++
```

### 5.4 Worked examples (these are the measured fixtures)

**A — Counter card (64 tokens; React equivalent 368):**

```
page Counter
state count=0

card sm center
  h Clicks
  p Press the buttons to change the value.
  metric {count}
  row center
    btn Decrement ghost disabled={!count} >count--
    btn Increment primary >count++
    btn Reset quiet >count=0
```

**B — Task CRUD with optimistic updates (178 tokens; React equivalent 1,441):**

```
page Tasks

type Task {id, title, done:bool, createdAt:date}
data tasks:Task[] GET /api/tasks
  add  POST   /api/tasks         {title}  optimistic:prepend
  save PATCH  /api/tasks/{id}    {done}   optimistic
  drop DELETE /api/tasks/{id}             optimistic

state draft=""
state filter=all|open|done

head Tasks — {tasks.open.count} open

form >tasks.add{title:draft}; draft=""
  input draft placeholder="Add a task…"
  btn Add primary disabled={!draft.trim()} busy="Adding…"

tabs filter

list tasks where={filter}
  check {done} >tasks.save
  text {title} strike={done}
  btn Delete quiet aria="Delete {title}" >tasks.drop
  empty Nothing here yet.
```

Everything the React version spends 1,259 extra tokens on — `useEffect` with cancellation, three `fetch` blocks with headers and JSON bodies, `useMemo` filtering, optimistic update + snapshot rollback per mutation, skeleton loaders, empty state, error banner with `role="alert"`, `remaining` count — is **compiler output**, derived from `optimistic:`, `where=`, `empty`, and the resource declaration.

**C — Landing page (376 tokens, of which 232 is irreducible copy; React equivalent 1,648):**

```
page Landing

nav Northwind
  link Features #features
  link Pricing #pricing
  btn "Get started" primary /signup

hero
  h1 Build the interface, skip the boilerplate
  p Northwind turns a short description into a working, accessible web app you actually own.
  btn "Start free" primary /signup
  btn "Watch demo" outline /demo

section #features cols=3
  card "Ship in minutes" | Describe the page, get a deployable build.
  card "Own your output" | Every project exports as plain framework code.
  card "Accessible by default" | Focus states, labels and contrast handled.

section #pricing Pricing cols=3
  tier Pro $24/mo "For working developers" cta="Go Pro" /signup featured
    Unlimited projects
    Custom domains

section #faq Questions
  faq open=1
    Can I export the code? | Yes. Every build is plain source you can download.

footer "© 2026 Northwind Labs"
  link Privacy /privacy
```

### 5.5 Grammar sketch (EBNF, abbreviated)

```ebnf
program     ::= directive* block*
directive   ::= 'page' ident meta?
              | 'type' ident '{' field (',' field)* '}'
              | 'data' ident ':' typeref verb url mutation*
              | 'state' ident '=' literal ('|' literal)*
              | 'route' path '->' ident
              | 'auth' provider guard*
block       ::= INDENT? element NEWLINE (INDENT block DEDENT)?
element     ::= tag positional* attr* action* content?
tag         ::= IDENT                        (* resolved against component registry *)
positional  ::= WORD | STRING | binding | modifier | route | anchor
attr        ::= IDENT '=' (literal | binding)
action      ::= '>' stmt (';' stmt)*
binding     ::= '{' expr '}'
content     ::= '|' TEXT | TEXT
mutation    ::= IDENT verb url body? ('optimistic' (':' strategy)?)?
modifier    ::= 'primary'|'ghost'|'quiet'|'outline'|'featured'|'sm'|'md'|'lg'|'center'|...
expr        ::= (* small pure expression language: paths, arithmetic, comparison,
                   boolean, ternary, safe-nav, collection aggregates (.count/.sum/.where) *)
```

Design constraint: the *full* grammar plus the component registry plus 3 in-context examples must fit under ~3,000 tokens. Above that, spec cost begins competing with the savings and in-context learnability degrades (this is an ablation, not an assumption — §8).

### 5.6 AST

```
Program
├── Meta        { name, title, theme, framework }
├── Types       [ TypeDecl { name, fields[] } ]
├── Resources   [ Resource { name, type, list:Endpoint, mutations[], optimistic } ]
├── State       [ StateDecl { name, init, domain? } ]
├── Routes      [ Route { path, page, guard? } ]
└── Tree        Element {
                  tag, registryRef, props{}, bindings[], actions[],
                  children[], slots{ empty, loading, error }, source:{line,col}
                }
```

Every node carries source spans — required for error messages that an LLM can act on (§6.7).

### 5.7 Type system

Gradual and inference-first. Three levels:
- **Untyped** — `state draft=""` infers `string`.
- **Shape-typed** — `type Task {id, title, done:bool}`; untyped fields default to `string`.
- **Checked bindings** — `{done}` inside `list tasks` resolves against `Task`; unknown field is a compile error with a did-you-mean suggestion.

Types exist to catch model mistakes and to drive codegen (form widgets from field types, date formatting), **not** to satisfy a soundness theorem. Unsound-but-useful is the correct trade here.

### 5.8 Component model

- **Registry-based.** Tags resolve against a component registry (name, props schema, slots, variants, a11y contract) — the shadcn-schema-as-agent-context pattern, formalized.
- **Registry is the in-context vocabulary.** Only registry entries actually referenced need be loaded — retrieval-augmented, so a 300-component design system does not cost 300 components' worth of context.
- **User components:** `def Card(title, body) ... ` with the same call syntax as builtins.
- **Escape hatch:** `raw react` / `raw svelte` blocks pass through verbatim.

### 5.9 Event and state model

- **State** is explicit, declared, and page-scoped by default (`state`), or app-scoped (`store`).
- **Derived state** is a binding, never a variable: `{tasks.open.count}`. No manual memoization, so no stale-dependency bugs — an entire React failure class deleted by construction.
- **Actions** are `>stmt` — assignment, increment, resource mutation, navigation, sequenced with `;`. Deliberately *not* Turing-complete; anything more goes in a `js` block. This is the security boundary (A2UI's "declarative, not executable" principle, with an opt-in).
- **Effects** are declared, not written: `on mount >tasks.list`, `on {filter} >tasks.list`.

### 5.10 API, data, auth, and DB models

```
data users:User[] GET /api/users?q={query} poll=30s cache=60s
  create POST /api/users {name,email} optimistic:prepend
  update PATCH /api/users/{id} {name,email} optimistic
```

Compiler emits: fetch layer, request cancellation, retry/backoff, cache, optimistic apply + rollback, loading/error state, and typed hooks.

```
auth clerk
  guard /admin role=admin
  guard /app signedIn
```

```
db postgres
  table tasks from Task
  policy tasks owner=user.id
```

The DB layer should be **deliberately deferred to v2**. Attempting client + server + schema + policy in one language in v1 is the most likely way to fail (§12.2).

---

## 6. Part 6 — Compiler architecture

### 6.1 Pipeline

```
GUML source
   ↓  Lexer          indentation-sensitive; emits INDENT/DEDENT; preserves spans
   ↓  Parser         recursive descent + Pratt for expressions; error-recovering
   ↓  AST            typed, span-annotated
   ↓  Resolver       tag → registry entry; binding → state/resource/type
   ↓  Semantic       type check, unknown-field diagnostics, a11y lint, exhaustiveness
   ↓  Desugar        convention expansion: states, optimistic rollback, effects, ARIA
   ↓  Optimizer      dead state elimination, binding dedup, static hoisting, tree-shake registry
   ↓  Codegen        pluggable backend
   ↓  Emit           React+TS · Svelte · Web Components · static HTML/CSS/JS
```

### 6.2 Lexer

Python/YAML-class significant indentation. Tabs rejected. Sigils tokenized as single characters. Two lexer modes: structural and *content* — after `|` or a bare-text position, the remainder of the line is a text literal, so prose never needs escaping. This detail matters: it is why GUML's content overhead is near zero, and why the content floor (§1.5) is achievable at all.

### 6.3 Parser

Recursive descent with **error recovery as a first-class requirement.** Because an LLM produced the input, the parser must (a) recover and continue to collect *all* errors in one pass, and (b) emit machine-actionable diagnostics. A parser that reports one error per round trip converts the repair loop from 1 iteration into N.

### 6.4 Semantic analyzer

- Resolve every tag; unknown tag → nearest-registry-match suggestion.
- Resolve every binding path; unknown field → "`Task` has no field `title2`; did you mean `title`?"
- Check action targets exist and mutations match declared bodies.
- **Accessibility lint as a hard error, not a warning:** icon-only control without `aria`, form input without label, non-text content without alt. This is where the "correct by construction" claim is earned.
- Exhaustiveness on `state x=a|b|c` switch-like bindings.

### 6.5 Optimizer

Modest by compiler standards, high value in practice: dead state elimination, common-binding CSE, static subtree hoisting, registry tree-shaking, and (per-backend) compile-time CSS extraction so the emitted app ships no utility-class bloat.

### 6.6 Code generator

Visitor per backend with a shared prelude library:

| Backend | Strategy | Priority |
|---|---|---|
| React + TS + Tailwind | Hooks + generated data hooks; shadcn primitives | **v1** — maximum ecosystem gravity, easiest human hand-off |
| Svelte | Runes + stores; smallest bundle | v2 — best demo of "compile away the framework" |
| Web Components | Custom elements + signals; framework-free | v2 — enterprise/embeddable |
| Static HTML/CSS/JS | Progressive enhancement; no framework | v1.5 — best Lighthouse numbers for the benchmark |

Emitting *idiomatic, human-editable* code is a requirement, not a nicety. It is the answer to "what happens when I outgrow your language" (§12.3) and it is what makes the export story credible.

### 6.7 The LLM-facing loop (the part that makes reliability real)

```
prompt + spec(cached) + registry(retrieved)
        ↓
   LLM emits GUML  ←──────────────┐
        ↓                          │  structured diagnostics
   grammar-constrained decode      │  (span + message + suggested fix)
        ↓                          │
   parse ──► semantic ──► compile ─┘  (bounded: ≤3 repair rounds)
        ↓
   emitted app ──► headless render ──► visual/a11y/Lighthouse check
```

Three published mechanisms, composed: grammar-constrained decoding for syntax (SynCode/Domino/xgrammar), grammar prompting for in-context learnability (Wang et al., NeurIPS 2023), compiler-feedback repair for semantics (DeclarUI: 98% compilation success, +29pp over baseline). **Note CRANE's warning:** constrain the *structured* output, not the reasoning — let the model think in free text, then emit constrained GUML.

### 6.8 Runtime architecture

Thin, per-backend. React backend: ~4–6KB of generated helpers (resource hook factory, optimistic reducer, focus manager). Svelte/WC: near-zero. **No interpreter ships to the browser** — GUML is compiled away entirely, which distinguishes it from SDUI/A2UI (both of which require a client renderer at runtime) and is a genuine architectural difference worth stating.

### 6.9 Extensibility

- **Registry packages** — publishable component sets with prop schemas, variants, a11y contracts, and *token cost metadata*.
- **Compiler plugins** — AST visitors at resolve/desugar/codegen phases.
- **Theme packs** — modifier → design-token mappings, so `primary` means the org's primary.
- **Language server** — LSP for humans; the same diagnostics feed the LLM loop.

---

## 7. Part 7 — The AI optimization angle

### 7.1 The two pipelines

```
Today:     prompt → LLM → ~1,400 output tokens of React → maybe runs
Proposed:  prompt → LLM → ~173 output tokens of GUML → compiler → guaranteed-shaped React
```

### 7.2 What is measured, and what is only hypothesized

**Measured (§1.5):** output-token reduction of 77–88% on authored fixtures; 44% advantage over minified JSON IR; content floor at 232/376 on a landing page; spec cost amortizes ~20:1 under prompt caching.

**Hypothesized, and requiring the Part 8 experiments:**

- *H1 — Validity.* Grammar-constrained decoding yields ~100% parse validity vs measured React compile failure rates. Supported by SynCode/Domino guarantees and DeclarUI's 98%; near-certain.
- *H2 — Correctness.* Functional correctness improves because state/effect coordination moves into the compiler. Supported by Anka's **+40pp on multi-step tasks**. **This is the crux hypothesis.** If H2 fails, the project is a cost optimization, not a research contribution.
- *H3 — Hallucination.* Fewer hallucinated APIs, because the vocabulary is a small closed registry and unknown tags are compile errors rather than runtime failures. High confidence.
- *H4 — Consistency.* Inter-run variance drops, because the compiler fixes all presentational decisions. High confidence and easy to measure (pairwise DOM/screenshot distance across N runs at fixed prompt).
- *H5 — Quality floor.* Accessibility and error-state coverage *rise* while tokens fall, because conventions are compiled in. High confidence; strong headline result if it holds ("8× fewer tokens **and** better a11y" is a much better paper than "8× fewer tokens").
- *H6 — Capability threshold.* Per *Capacity, Not Format*, the GUML advantage should be **largest on small models** (Haiku-class) and may **vanish or invert on frontier models** that write correct React effortlessly. This is the most important thing to measure and the most likely source of a negative result. Measure it early; if the win is small-model-only, that is still a valuable and publishable finding — and commercially it points at cheap-model + compiler as the cost play.

### 7.3 The prompt tax, stated honestly

Matveev (arXiv:2603.03306) names the failure mode for exactly this class of idea: instructional overhead can swamp per-token savings in short contexts, and the benefit follows a **non-linear curve** that only turns positive past a complexity threshold. GUML must therefore report:

- spec + registry tokens (input, cacheable) **separately from** generated tokens (output, not cacheable),
- total-cost-per-successful-app including repair rounds,
- the break-even artifact size below which raw React is simply cheaper.

Any paper that omits the break-even point will be correctly rejected.

---

## 8. Part 8 — Benchmark design

### 8.1 Dataset: GUML-Bench

150 tasks, 6 categories × 25, each with a natural-language prompt, a functional-requirements checklist, a reference screenshot, and a Playwright interaction test:

1. Landing / marketing pages (content-heavy — expect low compression)
2. Dashboards (chart + stat + table composition)
3. CRUD applications (state-heavy — expect high compression)
4. E-commerce flows (cart, variants, checkout)
5. SaaS app screens (settings, teams, billing, auth-gated routes)
6. Data-visualization apps (filter → query → chart interaction)

Sourcing: seed real-world structures from **Design2Code**'s 484 curated pages for realism; synthesize interaction requirements. Reuse Design2Code's automatic metrics and human-evaluation protocol so numbers are comparable to existing literature rather than incommensurable.

### 8.2 Systems compared

| Arm | Description |
|---|---|
| B1 | LLM → React+TS+Tailwind (direct) |
| B2 | LLM → HTML/CSS/JS (direct) |
| B3 | LLM → JSON UI IR (A2UI-shaped) → reference renderer |
| B4 | LLM → TOON-encoded IR → renderer |
| B5 | v0 (where API access permits) |
| B6 | Human expert React (quality ceiling) |
| **T1** | LLM → GUML → compiler |
| **T2** | T1 + grammar-constrained decoding |
| **T3** | T2 + compiler-feedback repair (≤3 rounds) |

Model grid: Haiku 4.5, Sonnet 5, Opus 5 — capability is a first-class independent variable (H6), not a footnote.

### 8.3 Metrics

**Efficiency:** input tokens (split: spec / registry / prompt), output tokens, cached vs uncached input, USD per successful app, wall-clock latency, repair rounds, tokens-per-successful-app.

**Correctness and quality:** parse/compile success; functional correctness (Playwright pass rate); visual fidelity (CLIP + block-match, per Design2Code); **axe-core accessibility violations**; Lighthouse performance; bundle size; error-state coverage (does it handle loading/empty/failure at all?); consistency (pairwise distance across 5 runs at temperature-equivalent settings).

**Human evaluation (n≥30 frontend developers):** pairwise preference on emitted code; readability rating; **timed modification task** (implement a specified change; measure time and success — this is the maintainability metric that actually matters); and a separate spec-readability task with non-engineer stakeholders.

**Edit-locality (novel, §4.C.6):** for 50 tasks, apply 3 scripted modifications each; measure output tokens and success rate per representation. I expect this to be GUML's strongest quantitative result and it is currently unmeasured by anyone.

### 8.4 Ablations

Spec size (500/1,000/2,000/3,000/5,000 tokens) × in-context examples (0/1/3/5) × grammar-constrained decoding (on/off) × repair rounds (0/1/3) × model (3 levels). This grid *is* the crossover characterization (§4.C.2).

### 8.5 Statistics and honesty requirements

Paired comparisons per task; bootstrap CIs; Holm correction across metric families. Pre-register hypotheses H1–H6. **Report per-category breakdowns, never a single average** (the content floor makes averages actively misleading). Publish all raw generations, not just aggregates.

### 8.6 Reproducing the preliminary measurement

Fixtures used for §1.5 are three paired files each (`{a,b,c}.react.tsx`, `{a,b,c}.guml`) plus `b.spec.json`, tokenized with `tiktoken` (`cl100k_base` and `o200k_base`). Note for the paper: use the target model's own tokenizer/counting endpoint for final numbers rather than `tiktoken`, which is an OpenAI tokenizer and will misestimate Claude token counts (typically undercounting by 15–20% on text, more on code). The direction and magnitude of the effect here is far larger than that error, but published figures should be measured with the right tokenizer.

---

## 9. Part 9 — Paper contributions

### 9.1 Positioning (do this, not the alternative)

**Do not submit:** "GUML: A New Markdown for Interactive Web Applications." Reviewer response is immediate and fatal: *MDX, Markdoc, A2UI, Vega-Lite, and JSX exist.*

**Submit instead:**

> **"How Should LLMs Represent User Interfaces? A Token/Accuracy Study of Intermediate Representations for Generative UI"**

Empirical, falsifiable, has a result whichever way it goes, and directly extends Tam et al. (EMNLP'24), *Capacity, Not Format* (2606.09410), and Athena (IUI'26). The language is the *instrument*, not the claim.

### 9.2 Paper ladder

| # | Title | Venue | Core contribution |
|---|---|---|---|
| 1 | How Should LLMs Represent User Interfaces? A Token/Accuracy Study of IRs for Generative UI | **EMNLP / ACL** (or NeurIPS D&B) | GUML-Bench + the Pareto frontier across 8 arms × 3 model scales; the content-floor finding |
| 2 | Convention as Compression: Compiler-Supplied Correctness in LLM-Generated Interfaces | **ICSE / FSE** | Formalization + measurement that compiled conventions raise a11y/error-coverage while cutting tokens 8× |
| 3 | Where Does a DSL Beat Python? Characterizing the Crossover for LLM Code Generation | **ICLR / NeurIPS** | Reconciles Anka vs. the low-resource survey; the decision rule |
| 4 | Editing, Not Writing: Representation Choice for Iterative LLM UI Development | **CHI / UIST** | Edit-locality benchmark + developer study |
| 5 | GUML: An AI-Native IR and Multi-Target Compiler for Web Application Generation | **PLDI / OOPSLA / SLE** (or tool paper) | The artifact: grammar, compiler, registry, backends |

Papers 1 and 2 are the ones that establish the work. Paper 5 is the artifact paper and should come *after* the evidence, not before.

### 9.3 Enumerated contributions

1. **GUML-Bench** — 150 tasks × 6 categories with functional tests, reference screenshots, and a11y/perf harness (reusable dataset contribution).
2. **The first token-efficiency measurement of UI IRs against a code baseline**, including the JSON-IR comparison and the content floor.
3. **A language design with a stated objective function**, plus the negative results from the design search.
4. **A multi-target compiler** with grammar-constrained decoding, in-context grammar prompting, and a compiler-feedback repair loop composed into one measured pipeline.
5. **The crossover characterization** — when purpose-built DSLs beat high-resource general-purpose languages for LLM generation.
6. **Convention-as-correctness** — evidence that a compiler-supplied convention layer improves accessibility and robustness while reducing generated tokens.
7. **The edit-locality metric** for representation evaluation.

---

## 10. Part 10 — Implementation roadmap

| Phase | Duration | Deliverable | Gate to proceed |
|---|---|---|---|
| **0 · Kill-or-continue spike** | **2 weeks** | Hand-write 10 GUML specs. Give the spec + 3 examples to Haiku 4.5, Sonnet 5, Opus 5 as pure prompting — **no compiler yet.** Measure: can they produce valid GUML? Compare output tokens and semantic correctness against React for the same 10 tasks. | **If frontier models cannot learn the syntax in-context, or if output-token savings do not survive contact with real generations, stop here.** Two weeks buys the answer to the single highest-risk question. Do not skip this. |
| **1 · Research + design** | 4 wks | Full related-work matrix; frozen v0 grammar; registry schema; objective function | Grammar ≤3,000 spec tokens with full coverage of the 6 benchmark categories |
| **2 · Parser** | 4 wks | Lexer + error-recovering parser + AST + span-accurate diagnostics; fuzz + differential tests | 100% parse of 50 hand-written specs; recovers from ≥90% of injected mutations |
| **3 · Compiler core** | 8 wks | Resolver, semantic analyzer, a11y lint, desugaring, React+TS backend | 20 fixtures compile to code that passes their Playwright tests |
| **4 · Component registry** | 6 wks | 40 primitives (shadcn-backed) with prop schemas, variants, a11y contracts; retrieval layer | Covers ≥90% of GUML-Bench element needs |
| **5 · LLM integration** | 6 wks | Grammar-constrained decoding (xgrammar/SynCode-class), grammar prompting harness, repair loop, cache-optimized prompt layout | ≥95% valid GUML from Sonnet 5 with ≤1 repair round |
| **6 · Benchmark + eval** | 10 wks | GUML-Bench; all 9 arms; ablation grid; human study (n≥30) | Statistically significant result on ≥3 of H1–H6 (positive *or* negative) |
| **7 · Second backend + paper** | 8 wks | Svelte or Web Components backend; papers 1–2 submitted | — |

**~12 months to first submission with 1–2 engineers.** Phase 0 is 2 weeks and de-risks the whole program; treating it as optional is the most common way projects of this shape die at month 9.

### 10.1 Technology choices, with reasoning

| Decision | Choice | Why |
|---|---|---|
| Compiler language | **Rust** | Fast enough for interactive loops; excellent parser ecosystem (`chumsky`, `logos`); compiles to WASM so the same compiler runs in browser, Node, and CI — which matters enormously for an AI builder product. Cost: slower iteration than TypeScript. |
| Alternative | TypeScript | Pick this if the team is TS-native and speed-to-paper dominates. It is a legitimate choice; do not over-engineer for performance you cannot yet measure. |
| Rejected | Go | Weaker parser/frontend ecosystem for this task |
| Parser tech | **Hand-written recursive descent + Pratt** | Indentation sensitivity and *error recovery quality* are the two hardest requirements, and both are worse with generators. Error messages are the LLM's feedback channel — they are a product surface, not a diagnostic afterthought. |
| Rejected | ANTLR / tree-sitter as the primary parser | ANTLR: heavy, poor recovery ergonomics. tree-sitter: excellent for *editor* incremental parsing — ship a tree-sitter grammar for tooling, not as the compiler frontend. |
| Rejected | LLVM | Category error. There is no machine-code backend here; the target is source text. |
| Constrained decoding | **xgrammar / SynCode-class CFG masking** | Only mechanism that *guarantees* syntactic validity. Apply per CRANE: constrain the emission, not the reasoning. |
| v1 codegen target | **React + TS + Tailwind + shadcn** | Largest ecosystem, best human hand-off, best baseline comparability, and the registry can be shadcn-backed rather than built from scratch. |
| v2 targets | **Svelte** (bundle-size story), **Web Components** (portability story) | |
| Testing | Playwright + axe-core + Lighthouse CI + `insta` snapshots | Benchmark harness and CI are the same infrastructure |

---

## 11. Part 11 — Open source and product analysis

### 11.1 Market context (as of mid-2026)

- **Lovable** ≈ **$400M ARR** (Feb 2026); reportedly $20M ARR in 2 months, fastest European startup on record.
- **Bolt.new** ≈ **$40M ARR** in its first ~6 months.
- **Vercel** valued ≈ **$9.3B** (whole business; v0 revenue not broken out).
- **AI app builder** category revenue ≈ **$4.7B (2026)**, projected **$12.3B by 2027**.
- **No-code AI platform** market ≈ **$8.6B (2026)** → **$75.1B by 2034** at ~31% CAGR.

The category is large, fast-growing, and its unit economics are inference-bound. That is the commercial thesis in one line.

### 11.2 Which product shape?

| Shape | Viability | Assessment |
|---|---|---|
| **Open-source compiler + spec** | **High** | The right entry. Languages win by adoption, not by licensing. Establishes the standard, builds the corpus, generates citations and contributors. |
| **Infrastructure layer for existing AI builders** | **Highest commercial leverage** | Lovable/Bolt/v0/Replit each have a direct margin and latency incentive. Selling *into* them beats competing with them. Pitch: "same output, 5× fewer generation tokens, better a11y floor." |
| **Agent UI toolkit** (MCP/A2UI/AG-UI-adjacent) | **High, and the most defensible near-term wedge** | Agents emitting UI is a live standards fight *right now*, and GUML's specific advantage — token efficiency + a compiled, deployable target — is exactly what those protocols lack. Ship a `guml → A2UI` and `guml → MCP-UI` emitter and you are complementary rather than competitive. |
| **Standalone AI website builder** | **Low** | Competing with $400M-ARR incumbents on distribution and polish is not where a language project wins. |
| **Enterprise AI dev platform** | **Medium-high, slow** | The governance story (guaranteed design-system compliance, a11y, auditable specs, no arbitrary generated JS) is genuinely compelling to enterprises and genuinely slow to sell. |
| **"React replacement for agents"** | **Low as stated** | Overreach. React is not the competitor; *un-representational code emission* is. |

### 11.3 Competitive positioning

| Against | GUML's claim | Their counter |
|---|---|---|
| React / Next.js | Not a competitor — a compile target | — |
| Flutter | Different platform focus | — |
| Webflow / Bubble | Visual no-code, human-driven; GUML is LLM-driven and text-first | Vastly better UX for non-developers |
| v0 / Lovable / Bolt | 5–8× fewer generation tokens; a11y floor; auditable spec | Distribution, polish, and "our output is real React that developers already trust" |
| **A2UI / MCP-UI / AG-UI** | 44% fewer tokens than JSON; compiles to a deployable app; has an application-logic layer | **Google-backed standard, shipping, multi-vendor coalition, security-first non-executable design** |
| Diff-based agents (Cursor/Claude Code) | Edit-locality advantage on *semantic* edits | Diffs already solve most of the token problem for iteration on existing code |

**The single most important strategic read:** A2UI is both the strongest competitor and the strongest partner. Being "the token-efficient surface syntax and compiler for the agent-UI ecosystem" is a far better position than being "a rival UI protocol."

---

## 12. Part 12 — Critical review

Written as adversarially as I can, because these are the objections that will actually arrive.

### 12.1 Why this might fail

**(1) The out-of-distribution penalty may dominate the token win.** The low-resource-DSL literature (arXiv:2410.03981, 2601.00469, 2603.05278) is consistent: models are worse at languages they have not seen. GUML has *zero* training data by construction. Anka's +40pp is one paper, one narrow domain, one author, no token accounting. If GUML produces 82% fewer tokens with 30% worse functional correctness, it is a worse system. **This is the primary risk and Phase 0 exists to find it in two weeks rather than nine months.**

**(2) Frontier models may make the problem obsolete.** Opus 5-class models write correct React at high reliability with 1M-token context. *Capacity, Not Format* is explicit: format costs are absorbed by capable models and only devastate weak ones. If the benefit is Haiku-only, the research contribution shrinks to "cheap models + compiler ≈ expensive model," which is real and commercially interesting but is not a language-design contribution.

**(3) Prompt caching has already eaten the input-side argument.** Cached input reads at ~0.1×. Long-context is standard. Half the original motivation ("context window savings") is largely solved by infrastructure. Only the *output* side remains — which is why the output measurement must carry the paper.

**(4) Diff-based editing has already eaten much of the iteration argument.** Real agents do not regenerate whole files; they patch. A well-executed diff loop captures a large share of the available token savings without any new language. GUML's edit-locality claim must be measured *against diff-based React editing*, not against naive full regeneration, or the comparison is a straw man.

**(5) The expressiveness cliff.** Every UI DSL in history hits the case it cannot express. When it does, users drop to `raw` blocks, and the token advantage evaporates precisely on the hard tasks that matter most. If 30% of real tasks need escape hatches, the effective compression is far below 8×. **Measure the escape-hatch rate on GUML-Bench and report it.** A benchmark composed only of expressible tasks is a rigged benchmark.

**(6) Ecosystem gravity.** React's value is node_modules, Stack Overflow, hiring, and the model's own priors. A new language competes with all of it simultaneously. Vega-Lite succeeded because charts are narrow and conventional; *applications* are neither.

**(7) The compiler is the easy part.** A working compiler is maybe 20% of the effort. The registry, themes, LSP, docs, migration path, debugging story, and source-map story are the other 80% — and they are what determine whether anyone adopts it.

**(8) Standards risk.** If A2UI becomes the de facto agent-UI format, a competing surface syntax is stranded. Mitigation: emit *to* A2UI/MCP-UI, do not fight them.

### 12.2 Biggest technical challenges

1. **Semantic gap.** "Add a testimonial section that matches the brand" is not expressible in any finite vocabulary. The registry will always be behind demand.
2. **Debuggability.** When the compiled React misbehaves, the user debugs generated code. Source maps from GUML → TSX → DOM are hard and unglamorous, and their absence kills adoption.
3. **Diagnostics as an LLM interface.** Error messages must be simultaneously human-readable and machine-actionable, span-accurate, and complete in one pass. This is a genuinely novel design problem and the literature offers little guidance.
4. **Registry versioning.** A component's props changing invalidates cached specs, cached prompts, and in-context examples all at once.
5. **Server/client boundary.** v1 client-only is honest. Auth, DB, and policy in the same language is a second project, and attempting it in v1 is how this fails.
6. **Grammar-size ceiling.** As the vocabulary grows to cover real apps, spec tokens grow, in-context learnability degrades, and the amortization math weakens. Retrieval mitigates but does not eliminate this.
7. **Circularity risk in evaluation.** If the same team authors the language, the benchmark, and the reference implementation, favorable bias is nearly unavoidable. Pre-register, publish all generations, and recruit external evaluators.

### 12.3 Assumptions that are probably wrong

| Assumption | Reality |
|---|---|
| "Token reduction ⇒ better generation" | Contested. Tam et al. show constraint can *hurt*; TOON's benchmarks do not cleanly replicate; the mechanism is capacity-dependent. |
| "LLMs will learn a new DSL easily from a spec" | Partly. Grammar prompting works; the low-resource literature says fluency lags. Anka is encouraging but is n=1. |
| "Compression scales uniformly" | **Measurably false** — content-heavy pages floor out at 2–3× (my §1.5 measurement). |
| "Token cost is the bottleneck" | For builders, partly. For most developers, *quality and trust* are the bottleneck; they will pay 5× tokens for output they can maintain. |
| "This is a new markdown" | It is a UI IR. The markdown framing invites the MDX/Markdoc dismissal and buys nothing. |
| "The compiler guarantees quality" | It guarantees *conventions*. Design quality, information architecture, and copy remain model outputs. |
| "One language covers client + server + DB" | Ambitious to the point of unshippable in v1. |

### 12.4 What could make this unnecessary

- **Better diff-based editing** — largely solves iteration-token cost with no new language.
- **Cheap, fast frontier models** — if Haiku-class quality reaches Opus-class, the cost argument thins.
- **A2UI winning outright with a token-optimized binding** — Google adding a compact encoding to A2UI would occupy this space directly.
- **Model-native UI output** — a model fine-tuned to emit a compact internal UI format would obviate a user-facing language entirely. (This is arguably the *correct* long-term answer, and GUML's real contribution may be *defining the target format for such a fine-tune* — a framing worth taking seriously rather than defending against.)
- **Speculative decoding / cheaper output tokens** — reduces the cost pressure, though not the error-surface argument.

### 12.5 Modifications that increase research value

1. **Reframe from "new language" to "empirical study of UI representations."** The measurement is the contribution; the language is the instrument. Single highest-value change.
2. **Make the DSL crossover the headline question.** It is unresolved in the literature and the field wants an answer.
3. **Add the JSON-IR and TOON arms.** Without them, "just use JSON" sinks the paper. With them, you have a Pareto frontier.
4. **Sweep model capability as a first-class variable.** Directly extends *Capacity, Not Format* into a new domain.
5. **Lead with the accessibility/quality result if it holds.** "8× fewer tokens **and** better a11y" is a far better paper than "8× fewer tokens."
6. **Report the content floor and the escape-hatch rate.** Reporting your own limits is what makes the rest of the numbers credible.
7. **Benchmark editing, not just authoring.** Most under-explored, most industrially relevant.
8. **Emit to A2UI/MCP-UI as an additional backend.** Turns the strongest competitor into a distribution channel and an evaluation baseline.
9. **Publish GUML-Bench standalone.** A dataset paper is a durable citation asset independent of whether GUML wins.
10. **Pre-register H1–H6 and commit to publishing negative results.** A rigorous negative result here is genuinely publishable and genuinely useful.

### 12.6 Reviewer-style criticism, by venue

**ACM CHI (Reject / Major Revision).** *"The submission proposes a language and compiler but evaluates neither with users. Athena (IUI 2026) already established that intermediate representations improve LLM app generation, with a user study; Jelly (CHI 2025) already generates UI specifications from a task-driven data model; SpecifyUI already supports spec-level editing. The token-efficiency contribution is a systems concern with no demonstrated human benefit. What is the developer's experience of debugging compiled output? Of hitting the expressiveness cliff? The 30-developer study is described but its instrument is not. Without a study of iteration and repair with real users, this is not a CHI contribution."*

**IEEE Software (Accept with revisions).** *"Practical and timely. The economic argument is concrete and the compiler-feedback loop is well-grounded in DeclarUI. Practitioners will want: migration path from existing React codebases; what happens at the expressiveness boundary; source-map/debugging story; registry governance. Trim the research framing, expand the adoption and operations sections. Report the break-even artifact size — practitioners will ask immediately."*

**ICSE (Weak Reject → Accept if fixed).** *"The token measurement is on author-written fixtures, not model generations — that is a preliminary observation, not a result. The core claim (fewer tokens ⇒ higher correctness) is untested. The benchmark is described but not run. Threats to validity are severe and under-discussed: the authors designed the language, the benchmark, and the reference implementation. Selection bias in task construction is unaddressed — did tasks get chosen because GUML expresses them? Report escape-hatch frequency. Compare against diff-based editing, not only full regeneration. If the full evaluation in §8 were executed with pre-registration and external evaluators, this would be a solid ICSE paper. As submitted, the evidence does not support the claims."*

**NeurIPS workshop (Accept).** *"Nice framing of representation choice as a token/accuracy trade-off, with a clear link to format-restriction results (Tam et al. 2024; Capacity-Not-Format 2026). The Anka-vs-low-resource-survey tension is well identified and the crossover question is genuinely interesting. Preliminary numbers are suggestive. Needs: actual model generations, more than one domain, and per-model-scale breakdown. Good workshop paper; strong basis for a full submission."*

**PLDI / OOPSLA (Reject as submitted).** *"The language design is reasonable engineering but the novel PL content is thin: indentation-based nesting, reactive property bindings, and semantic modifiers are all established (QML, SwiftUI, Python). The type system is explicitly unsound and no metatheory is offered. No formal semantics, no soundness result, no novel compilation technique. Consider SLE or a tools track, or develop the formal semantics of the convention-desugaring layer — the claim that compiled conventions guarantee correctness properties is the one place where genuine PL content could live, and it is currently asserted rather than proven."*

---

## 13. Final recommendation

### Scores

| Dimension | Score | Reasoning |
|---|---|---|
| **Research worthiness** | **7 / 10** | 4/10 as "a new markdown language" — that idea is occupied by MDX, Markdoc, A2UI, and Vega-Lite. **8/10** as "an empirical study of how LLMs should represent UIs, with a measured token/accuracy frontier and a characterization of the DSL crossover." The reframing is worth four points. GUML-Bench alone is a publishable artifact. |
| **Industry potential** | **7 / 10** | Real economics in a $4.7B category with inference-bound margins; measured 5–8× output reduction with ~20:1 amortization; latency win is the strongest practical hook. Discounted for: A2UI/Google standards risk, diff-based editing already capturing much of the value, and ecosystem gravity. Best path is infrastructure-into-builders or agent-UI toolkit, not a standalone builder. |
| **Difficulty** | **8 / 10** | Compiler is tractable (6–9 months). Everything else is not: registry breadth, escape-hatch design, source maps, LSP, LLM-actionable diagnostics, plus a rigorous multi-arm benchmark with human evaluation. 12 months to first submission with 1–2 strong engineers, and that assumes Phase 0 passes. |

### Recommendation

**Proceed — with the reframing, and with Phase 0 as a hard gate.**

Three concrete commitments:

1. **Run the two-week Phase 0 spike before anything else.** Ten hand-written specs, three models, pure prompting, no compiler. Answer one question: *can models produce valid, semantically correct GUML from a spec in context, and does the token saving survive real generation?* If yes, the remaining 11 months are justified. If no, you have spent two weeks instead of nine months, and the negative result is itself worth writing up.

2. **Reframe the thesis before writing anything for publication.** Not *"an AI-native markdown language."* Instead: *"an intermediate representation and compiler for LLM-based software generation, and an empirical study of what representation LLMs should target."* The instrument is the language; the contribution is the evidence. This is the correct instinct from the original brief and it should be followed all the way through.

3. **Treat A2UI as a partner, not a rival.** Ship `guml → A2UI` and `guml → MCP-UI` backends alongside React. It gives you a baseline for the benchmark, a distribution channel, and immunity from the standards risk that is otherwise the single largest strategic threat.

The strongest version of this project is not "a better way to write UIs." It is: **a measured answer to the question of what LLMs should emit when asked to build software — with a working compiler as the proof that the answer is implementable.** That question is open, the field is confused about it, and the evidence needed to settle it is within reach of a small team in a year.

---

## Sources

**Papers**
- [A Survey on LLM-based Code Generation for Low-Resource and Domain-Specific Programming Languages (arXiv:2410.03981)](https://arxiv.org/abs/2410.03981)
- [Anka: A Domain-Specific Language for Reliable LLM Code Generation (arXiv:2512.23214)](https://arxiv.org/abs/2512.23214)
- [Athena: Intermediate Representations for Iterative Scaffolded App Generation with an LLM (arXiv:2508.20263)](https://arxiv.org/abs/2508.20263) · [ACM IUI 2026](https://dl.acm.org/doi/10.1145/3742413.3789133)
- [Generative Interfaces for Language Models (arXiv:2508.19227)](https://arxiv.org/abs/2508.19227)
- [Generative and Malleable User Interfaces with Generative and Evolving Task-Driven Data Model — CHI 2025](https://dl.acm.org/doi/10.1145/3706598.3713285)
- [SpecifyUI (arXiv:2509.07334)](https://arxiv.org/abs/2509.07334)
- [Bridging Design and Development with Automated Declarative UI Code Generation — DeclarUI (arXiv:2409.11667)](https://arxiv.org/abs/2409.11667)
- [Grammar Prompting for Domain-Specific Language Generation with LLMs (arXiv:2305.19234)](https://arxiv.org/abs/2305.19234) · [NeurIPS 2023 proceedings](https://proceedings.neurips.cc/paper_files/paper/2023/file/cd40d0d65bfebb894ccc9ea822b47fa8-Paper-Conference.pdf)
- [SynCode: LLM Generation with Grammar Augmentation (arXiv:2403.01632)](https://arxiv.org/pdf/2403.01632)
- [Guiding LLMs The Right Way: Fast, Non-Invasive Constrained Generation — Domino (arXiv:2403.06988)](https://arxiv.org/html/2403.06988v1)
- [CRANE: Reasoning with Constrained LLM Generation (arXiv:2502.09061)](https://arxiv.org/html/2502.09061v3)
- [TreeCoder (arXiv:2511.22277)](https://arxiv.org/pdf/2511.22277)
- [Leveraging LLMs for Multi-File DSL Code Generation: An Industrial Case Study (arXiv:2604.24678)](https://arxiv.org/pdf/2604.24678)
- [From Text to DSL: Evaluating Grammar-Based Model Generation Using Open LLMs (arXiv:2605.15865)](https://arxiv.org/html/2605.15865v1)
- [DSL or Code? Evaluating LLM-Generated Algebraic Specifications: A Case Study at Kinaxis (arXiv:2601.00469)](https://arxiv.org/html/2601.00469v2)
- [A framework for assessing code generation of constraint DSLs with LLMs (arXiv:2603.05278)](https://arxiv.org/html/2603.05278) · [ScienceDirect](https://www.sciencedirect.com/science/article/pii/S0164121226001044)
- [DSL-Xpert 2.0 — JSS](https://www.sciencedirect.com/science/article/pii/S0950584925002939) · [microsoft/dsl-copilot](https://github.com/microsoft/dsl-copilot)
- [Token-Oriented Object Notation vs JSON: A Benchmark of Plain and Constrained Decoding Generation (arXiv:2603.03306)](https://arxiv.org/abs/2603.03306) · [TOON spec](https://toonformat.dev/) · [toon-format/toon](https://github.com/toon-format/toon) · [independent benchmarks](https://www.improvingagents.com/blog/toon-benchmarks/)
- [Capacity, Not Format: Rethinking Structured Reasoning Failures (arXiv:2606.09410)](https://arxiv.org/html/2606.09410)
- [Less Is More: DocString Compression in Code Generation — ACM TOSEM](https://dl.acm.org/doi/10.1145/3735636)
- [Design2Code (arXiv:2403.03163)](https://arxiv.org/pdf/2403.03163) · [project page](https://salt-nlp.github.io/Design2Code/) · [repo](https://github.com/NoviScl/Design2Code)
- [WebSight technical report (arXiv:2403.09029)](https://arxiv.org/pdf/2403.09029) · [Web2Code — NeurIPS 2024 D&B](https://proceedings.neurips.cc/paper_files/paper/2024/file/cb66be286795d71f89367d596bf78ea7-Paper-Datasets_and_Benchmarks_Track.pdf)
- [VegaChat (arXiv:2601.15385)](https://arxiv.org/html/2601.15385v1) · [Raiven: LLM-Based Visualization Authoring via DSL Mediation (arXiv:2604.10008)](https://arxiv.org/pdf/2604.10008) · [VL2NL (arXiv:2309.10245)](https://arxiv.org/pdf/2309.10245)
- [Oracular Programming (arXiv:2502.05310)](https://arxiv.org/pdf/2502.05310) · [ShortCoder (arXiv:2601.09703)](https://arxiv.org/html/2601.09703) · [When to Stop? (arXiv:2407.20042)](https://arxiv.org/abs/2407.20042) · [Biscuit (arXiv:2404.07387)](https://arxiv.org/html/2404.07387v2)
- Tam et al., *Let Me Speak Freely? A Study on the Impact of Format Restrictions on Performance of Large Language Models*, EMNLP 2024 (Industry Track)

**Protocols, systems, industry**
- [Introducing A2UI — Google Developers Blog](https://developers.googleblog.com/introducing-a2ui-an-open-project-for-agent-driven-interfaces/)
- [MCP-UI: A Technical Deep Dive — WorkOS](https://workos.com/blog/mcp-ui-a-technical-deep-dive-into-interactive-agent-interfaces) · [AG-UI vs MCP-UI](https://blog.niradler.com/ag-ui-protocol-vs-mcp-ui-which-one-should-you-use)
- [A Deep Dive into Airbnb's Server-Driven UI System](https://medium.com/airbnb-engineering/a-deep-dive-into-airbnbs-server-driven-ui-system-842244c5f5) · [SDUI at Airbnb, Netflix, Lyft](https://medium.com/@aubreyhaskett/server-driven-ui-what-airbnb-netflix-and-lyft-learned-building-dynamic-mobile-experiences-20e346265305)
- [shadcn/ui docs](https://ui.shadcn.com/docs) · [Directed Context Programming for AI-Generated UIs](https://medium.com/@peter_84200/directed-context-programming-for-ai-generated-uis-588505a1172b)
- [Lovable vs Bolt.new vs v0: $400M vs $40M ARR](https://tech-insider.org/au/lovable-vs-bolt-new-vs-v0-2026/) · [Lovable revenue tracker](https://aifundingtracker.com/lovable-vibe-coding-revenue/) · [Bolt.new statistics 2026](https://www.getpanto.ai/blog/bolt-new-statistics)
- [Generative UI research paper index](https://awesomegenerativeui.com/papers)

**Model pricing** — Claude Opus 5 $5/$25 per MTok, Sonnet 5 $3/$15 ($2/$10 introductory through 2026-08-31), Haiku 4.5 $1/$5, Fable 5 $10/$50. Output is 5× input across the line.
