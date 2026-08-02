# GUML build plan

Phases map 1:1 to the research report (`GUML-Research-Report.md` §10). Each phase has a **gate**
— do not start the next phase until the gate is met. Items already done are checked.

Legend: `[ ]` todo · `[~]` partly done, with the remainder named in the sub-items · `[x]` done ·
`[⛔]` blocked on something outside the code, with the blocker named on the line · **GATE** = hard stop.

A `[⛔]` is **not** a soft `[x]`. It marks a question this repository has not answered, and the distinction
matters most where it is least convenient — see Phase 0.

---

## Phase 0 — Kill-or-continue spike (2 weeks) ⚠️ HIGHEST PRIORITY

The whole 12-month program rests on one unproven assumption: *a model can produce valid,
semantically correct GUML from a spec in context, and the token saving survives real
generation.* Two weeks buys that answer. Protocol: `spec/PHASE0.md`.
Harness: `bench/phase0/` — `just phase0-verify` runs everything that needs no API key.

> ### ⛔ BLOCKED, and out of scope for current work
>
> The sweep needs **funded API access**. An attempt with a valid key returned
> `Your credit balance is too low to access the Anthropic API` — the key authenticated, the account had no
> credit, and zero of the 90 generations ran. Nothing was billed.
>
> Marked blocked rather than removed, on purpose. The gate is what tells a reader which of this project's
> claims are *measured* and which are *hypothesised*, and deleting it would make the roadmap describe a
> smaller set of open questions than there are. **Nothing below this line has been answered, and no figure
> anywhere in the repository depends on it having been** — every published number is an authored-artifact
> measurement with no model in the loop, and labelled as such.
>
> Everything that does not need the key is done and runs in CI: the frozen spec, the ten tasks, the ten
> paired React references, the prompt assembly, the scoring harness, the blind scoresheet, and a scoring
> self-test over synthetic generations. `just phase0-verify` is that subset.
>
> To unblock: fund the account, set `ANTHROPIC_API_KEY`, and run `just phase0-run`. It is resumable —
> completed generations are skipped and failed ones retried. Criteria 1 and 2 come out automatically;
> criterion 3 needs a blind human grader, which is not a step the project's own claim discipline lets the
> author take.

- [x] Freeze a v0.1 spec small enough to fit in context (`spec/GUML-SPEC.md`; largest assembled prompt ~2,970 est. tokens, budget enforced by `preflight.mjs`)
- [x] Write 10 task specs by hand across the 6 benchmark categories (2 structure-heavy, 2 content-heavy, 6 mixed) — `bench/phase0/tasks/index.mjs`
- [x] Write the paired React+TS+Tailwind reference for each of the 10 — all typecheck under `--strict`
- [x] Prompt harness: spec + registry slice + 3 examples, no compiler in the loop, stable prefix cached
- [x] Scoring harness: parse validity via `guml check --format json`, escape hatches, tokens, latency, gate check
- [x] Blind human scoresheet (arm and model stripped, deterministic shuffle) + rubric — `bench/phase0/rubric.md`
- [x] Scoring self-test over synthetic generations, so a miscount surfaces before the API spend
- [x] **The sweep is resumable across failures**, which one live attempt earned. The resume check was
      `existsSync(file)` and a *failed* run writes its error to that same path — so a rate limit, a dropped
      connection or an expired card was baked in permanently, and a later credited run would have skipped all
      90 generations and reported an empty result set with nothing saying why. It now resumes past
      **successful** runs only and logs each retry.
  - [x] And it fails fast on an account-level error — credit, authentication, permission — instead of
        grinding through 89 identical failures and leaving 90 files for someone to delete
- [⛔] Count tokens with the **target model's own tokenizer** — wired to `count_tokens` and `usage`, needs a
      funded run. Never `tiktoken`: it is an OpenAI tokenizer and undercounts Claude, so every figure in this
      repository is labelled as a ~3.6 chars/token estimate until this runs
- [⛔] Run all 10 × {Haiku 4.5, Sonnet 5, Opus 5} × {0, 3 in-context examples} — **90 generations**
      (the runner's own header said 120 for a while; the React arm has one condition, not two). Needs a
      funded account
- [⛔] Score semantic correctness against each checklist, blind — needs a **human grader**. Not a step the
      author can take: scoring one's own project's output is the authorship bias the report requires be
      disclosed, so doing it would invalidate the result rather than produce one
- [⛔] Record the **escape-hatch rate**: how many of the 10 needed a construct the spec cannot express.
      Blocked on the generations — though the same measurement over the *authored* GUML-Bench references is
      done, and is what the vocabulary decisions have actually been made on
- [⛔] Write up `spec/phase0-results.md` including negative findings first. Blocked on having results

**GATE** — continue only if *all three* hold:
- [⛔] ≥80% of generations are parseable GUML at 3 in-context examples on Sonnet 5
- [⛔] Median output-token reduction ≥3× vs the paired React on structure-heavy tasks
- [⛔] Semantic correctness is not *worse* than the React baseline on the same tasks

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
- [x] Registry schema + `TagKind` semantics documented for external component packages —
      `spec/REGISTRY.md`, rewritten against the compiler rather than from the source. Every field,
      default and rejection, with the audit output pasted from a real run. See Phase 4 for what the
      rewrite found: the previous version denied three features that had shipped
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
- [x] Differential fuzzing: four `cargo-fuzz` targets in `fuzz/` (`parse`, `format`, `expr`, `roundtrip`),
      outside the workspace because libFuzzer needs a nightly toolchain. `just fuzz-guided` runs one.
      The `parse` target enumerates backends from `backend_names()` now — it named four by hand while the
      compiler had eight, so `wc`, `a2ui` and `mcp-ui` had never been fuzzed at all
- [x] Property test: every reported span is inside the source, on a char boundary, and has a line ≥ 1 —
      asserted over the generated corpus in `tests/fuzz.rs`. A span past the end makes an editor throw
      when it highlights it, and makes `guml fix` slice out of bounds

**GATE**
- [x] 100% parse of the hand-written fixture set
- [x] **Recovers and reports ≥90% of injected single-token mutations without cascading errors** —
      `tests/mutation.rs`, in CI. **1,031 definitionally-invalid mutants** injected into every known-good
      document in the repository: **95.5% detected**, **99.0% with no stray syntax error**.
  - [x] Getting the *measurement* right was the whole difficulty, and two versions of it were wrong in
        opposite directions. Both are recorded in the file, because a gate whose definition is unstated is
        a number anyone can move.
    - [x] *The denominator has to be invalid.* The first version deleted a trailing token from any line,
          and `card sm center` → `card sm` is completely valid GUML — a card with one fewer modifier. Those
          scored as missed detections and dragged the figure to 79%, measuring nothing but the generator's
          carelessness. Every mutation is now definitionally invalid, and a tag typo that happens to land
          on another real tag is discarded rather than counted
    - [x] *"Cascading" means a parse desync, not semantic reach.* `state count=0` → `sate count=0` reports
          errors on every line that reads `count`, and `list tasks` → `lst tasks` on every child that
          loses the row scope. Both are the resolver doing exactly its job. So the gate is **no lexical,
          layout or syntax error (`GUML0001`–`GUML0023`) on a line the mutation did not touch** — the
          parser losing its place, which is the failure that hands a repair loop spurious work
  - [x] Two real defects found by it, both silent:
    - [x] A `{…}` inside a **quoted attribute value** was resolved by nothing. `aria="Delete {ttle}"`
          compiled clean and emitted a read of a field that does not exist, so the accessible name was the
          string "undefined" at runtime. Codegen had always interpolated these; only the resolver did not
          know they were references
    - [x] `GUML0011` was reported **per line** rather than per run, so one mis-indented `section` produced
          16 diagnostics for one mistake — 15 of them on lines that were correct and would have been fixed
          by the first edit. Now one report naming the run's extent, and the widest blast radius across
          352 indentation mutants fell from **15 lines to 1**
- [x] **Zero panics over 1M fuzz iterations.** `just fuzz-long`, seed `0x123456789abcdef1`: 1,000,000
      generated documents in 453 s, no panic, every span inside its source and on a char boundary. 32,318
      compiled clean and were lowered by **all eight backends**.

      Stated precisely because the phrasing invites an overclaim: this is the **seeded** generator —
      deterministic and reproducible from its seed, but *not* coverage-guided. A libFuzzer run explores
      paths it cannot reach, which is what `fuzz/` and `just fuzz-guided` are for. Two different claims,
      and only the first has been run to a million.

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
  - [~] Type inference over expressions (`types.rs`): `Num`/`Str`/`Bool`/`List`, string concat vs
        arithmetic, ordering, attribute types, content interpolations. `Unknown` is never an error, so
        it is deliberately incomplete rather than unsound — full inference needs the type of every
        resource field threaded through aggregates
  - [x] Exhaustiveness on enumerated domains: a comparison against a value outside the domain is
        `GUML0080`. Assignment was already checked; comparison is the more dangerous half, because
        `{filter == "opne"}` is not a type error but **dead code** — the branch never runs, the page
        renders the wrong thing, and nothing else in the pipeline had an opinion
- [x] **Accessibility lint as hard errors** (`GUML0050`, `GUML0051`), with severity graded by what the compiler can recover
- [x] **Desugar pass**: the conventions that make the token saving real
  - [x] Resource layer: fetch on mount with `AbortController` cancellation
  - [x] Resource layer: retry with backoff, and a **response cache** with the four things every
        application needs from a fetch layer and nobody writes on the first pass — in-flight
        deduplication, stale-while-revalidate, invalidation on mutation, and serving stale on a network
        failure. Generated once per file in both a typed and an untyped spelling, shared by React, Svelte
        and the Web Components backend.
    - [x] The subtle one is invalidation. Without it the refetch after a mutation is a cache *hit* on the
          pre-mutation list, so the row the user just added visibly disappears — and it reads as a broken
          optimistic update rather than a stale cache
    - [x] An `alive` flag guards the state writes as well as the `AbortController`. A cache hit can
          resolve after unmount without the abort ever firing, which is where React's
          "setState on an unmounted component" warning comes from
  - [x] **Error boundary**, and deliberately *only* for a document that uses an escape hatch. Generated
        render code comes from a typechecked expression tree and has nothing in it to throw; a `js` or
        `raw` body is the one place the compiler's guarantees stop, and one throw there blanks the page.
        Wrapping every page would be ceremony rather than safety
  - [x] Loading skeleton / empty / error slots auto-filled (`role="alert"`, `animate-pulse`)
  - [x] Optimistic apply + snapshot rollback from `optimistic:` (prepend / replace / drop)
  - [x] `list where=` filtering via `useMemo` with a derived dep list, aggregates (`tasks.open.count`)
  - [x] `form` submit wiring with a threaded pending flag, `tabs` from an enumerated domain, `faq` as `<details>`
  - [x] Expression lowering to JS, mirrored in the TS runtime and pinned by a parity test
  - [x] Emitted TSX typechecks under `tsc --strict` (`scripts/typecheck-emitted.mjs`) — found four real bugs.
      Node rather than bash: the shell version needed `bash`, `TMPDIR` and `wc`, so on a Windows checkout
      the strongest check on the code generator silently did not run — and this project's CI tests on
      Windows. Rewriting it immediately found two more: a `select` over an enumerated state emitted
      `setSeverity(e.target.value)` where the state's type is a union (`TS2345`), and a row-scoped mutation
      called from outside a repeater was handed `{}` where the row type was expected. The second is now
      `GUML0101`, reported against the *document* — relying on the emitted code failing to compile assumes
      somebody runs that step
  - [x] Declared effects (`on mount`, `on {expr}`) as explicit syntax — `guml-ast::Effect`, lowered to
        `useEffect` in React and `onMount`/`$effect` in Svelte, 17 tests in `tests/effects.rs`. This line sat
        unchecked long after the feature shipped, which is the drift the stability tests exist to catch and
        this file has no equivalent for.

        The point of the directive is the **absence of a dependency array**: `useEffect(fn, [deps])` is wrong
        in two directions — a missing entry reads a stale value, a spurious one loops forever — and the
        correct list is not derivable from the lines nearby, so a model gets it wrong readily. Here the
        dependency *is* the trigger, so there is no second list to keep in sync. Svelte needed `untrack` to
        get the same semantics, because `$effect` tracks every reactive read in its body rather than the
        declared one
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
  - [x] Go to definition, find references, **rename** and **range formatting**. Rename is *verified*
        rather than trusted: occurrences are found lexically — the only approach that cannot miss one,
        including inside a `js` body or an action, where nothing resolves names — and the renamed
        document is re-checked. If it gained an error the original did not have, the rename is refused
        with the reason, because an editor cannot undo an edit it has already applied. Range formatting
        is exact because GUML's formatter is line-local, and returns nothing rather than a wrong range
        when that assumption breaks. 27 tests
  - [x] **tree-sitter grammar** (`editors/tree-sitter-guml/`): **14 of 14 corpus cases**, and every one of
        the repository's 10 real `.guml` documents parses with **zero ERROR or MISSING nodes**. `just
        grammar-test` runs both and CI runs it. Nothing depends on it — editor colour comes from the LSP's
        semantic tokens and from the generated TextMate grammar, both of which use the compiler's own lexer
        — so it is here for tree-sitter consumers (Neovim, Helix, Zed, GitHub).

        **The methodological point is worth more than the grammar.** Seven bugs; the corpus passed 12 of 12
        with three of them live, because a hand-written corpus can only hold cases someone thought of.
        `npm run check:fixtures` — parse every real document, fail on any error node — is what found them,
        and is the same argument as `highlight-parity`: agree with the compiler on real input, not on
        examples. Each diagnosis is recorded in `grammar.js` and `src/scanner.c`:
    - [x] *Pending DEDENTs at end of file.* The last block in a document was never closed, so every parse
          ended `(MISSING _dedent)`. Easy to miss because a document ending at column zero parses fine
    - [x] *A stale generated tag list.* `src/tags.h` held 8 text tags against a registry of 16, so half
          the vocabulary's prose lines lexed as piles of identifiers. `check:tags` now fails on staleness
          and CI runs it — and the generator emits a second list (`tier`/`faq` content children) that the
          scanner needed and did not have
    - [x] *`_raw_line` gated on scanner state rather than on the grammar*, which made `content_line` — the
          rule for a `tier` or `faq` body — unreachable by construction. Splitting the token into
          `_indent`/`_verbatim_indent` is what fixed it: the scanner reads which body the *grammar* wants
          out of `valid_symbols` instead of trying to remember which tag opened it
    - [x] *Cross-line scanner state was being discarded.* tree-sitter persists external-scanner state only
          for a call that **returns a token**, so the prose decision — set at line start, read mid-line —
          vanished whenever no indent token was due. Top-level prose therefore never worked at all, and it
          looked fine nested because INDENT happened to be returned on the same call. Now decided while
          emitting the NEWLINE that ends the line above, which always returns a token
    - [x] *A sibling closed its own block* (`<=` where `<` was meant), so a body could hold only one child.
          All ten corpus cases passed regardless — each happened to have exactly one child per level
    - [x] *Two top-level siblings nested*, because at depth zero the indent branch fired for any line.
          Invisible to the corpus for a subtle reason: every case had a `page` directive before its first
          indent, and a directive has no body, so `valid_symbols[INDENT]` was false there
    - [x] *A bare text tag had no possible parse.* `divider` and `skeleton` are text-kind and normally
          carry no text; the PROSE branch returned false on an empty remainder instead of falling through
          to NEWLINE, which is external-only. Nine ERROR nodes in `fixtures/e.guml`
    - [x] *`identifier` was narrower than the compiler's word rule, and positionals had to precede
          attributes.* `$0/mo` is one `Word` to the compiler and matched nothing here, and
          `tier Pro … cta="Go Pro" /signup` puts a positional after a keyed attribute. 14 ERROR nodes in
          `fixtures/c.guml`
    - [~] Known limitation, **accepted and recorded** rather than worked around: in the tree-sitter grammar, a
      document whose *first* line is a text tag colours its remainder as words rather than one `prose` node.
      Not open work — the decision is to leave it. tree-sitter persists external-scanner state only for a
      call that returns a token, so the prose decision is made on the previous line's `NEWLINE`, and the
      first line of a document has none. It is reachable only in a document the compiler rejects with
      `GUML0041` (no `page` directive), and it degrades to imperfect colour rather than to a parse error
          colours its remainder as words rather than one `prose` node, because the decision is made on the
          previous line's NEWLINE and there is no previous line. Only reachable in a document the compiler
          rejects with `GUML0041`, and it degrades to imperfect colour rather than to an error

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
  - [x] Binding CSE: a repeated *aggregate* is hoisted into one `useMemo`.
        `{tasks.open.count}` lowers to an O(n) `filter().length`, so three uses were three scans of the
        list per render for one number. Only aggregates qualify — memoising `{count}` would buy a hook
        for nothing — and row-scoped expressions are excluded, because inside a repeater the value
        depends on `item` and hoisting it would be wrong rather than unhelpful. Substitution happens on
        the *lowered* JavaScript, so two spellings that lower the same share the memo
- [x] **Source maps** GUML → TSX: Source Map v3 with VLQ mappings and inlined `sourcesContent`,
      emitted by `guml build --source-map`. Line granularity, because one GUML line becomes a
      *region* of TSX and a column claim would be invented
  - [x] Every declaration **and every element, nested ones included**. A repeater reclaims its own
        `<ul>`/`.map(`/closing lines after each child region, so a binding error inside a row template
        resolves to the row's line instead of to the `list` twenty lines above it. Before this,
        three constructs inside a `list` shared one attribution — a valid map that opens the right
        file at the wrong line
- [x] Snapshot tests with `insta`: 8 snapshots over every fixture, plus the JSON tree and a readable
      source-map table. `d.guml` is covered through both React and the no-JavaScript backend, because
      its output is the part the compiler promises *not* to touch — a `js` block hoisted verbatim, a
      `raw react` block left in place, a `raw svelte` block that must not appear at all

**GATE**
- [x] All fixtures compile with zero warnings. The last one was real: `b.guml`'s task input was named
      only by its placeholder, which disappears the moment someone types. The paired React and JSON
      representations shared the defect, so all three gained an accessible name — fixing only the GUML
      would have biased the comparison in its favour. Re-measured: **b 178 / react 1,441 / json IR 324**
      (cl100k), 87.6%, 8.10×, and 45% vs the JSON IR — unchanged. Twelve published figures updated
- [x] **`table` lowers to a real `<table>`**, with `cols="Client, Amount, Due"` giving the `<thead>` and the
      compiler adding `scope="col"`. Until this, `table` emitted a `<ul>` in **every** backend: a document
      that asked for tabular data got rows with no columns, no headers and no header association for a
      screen reader, and `render-emitted.mjs` had asserted "table without header cells" since the day it was
      written *without that assertion ever running once*, because no `<table>` was ever emitted to check.

      `cols=` is the one attribute with a per-tag type. A grid's columns are a **count** the compiler
      generates (`grid cols=3`); a table's are **names** only the author knows. Same question, two answers,
      one attribute — and it is why `validate::numeric_on` and `types::attribute` now take the tag.
  - [x] Two failures reported rather than guessed at: a `table` with **no** `cols=` (an accessibility defect,
        and the compiler cannot invent the names), and a `cols=` whose **count does not match** the row
        template — every header would sit one column left of its data, which reads as correct at a glance and
        is worse than an unlabelled table. The second fired on four of the eleven tables in this repository
        when the column lists were first written, including three of my own
  - [x] The loading skeleton takes the table's shape, **with its headers**. They are static, so rendering
        them while loading removes the layout shift instead of shrinking it — which is what the wc backend
        already did by keeping its `<thead>` outside the `<tbody>` it rewrites
  - [x] Four defects surfaced on the way, each caught by a different gate and none by the tests:
    - [x] *Wrapping per emitted line instead of per child.* A row child can render as many lines — a `modal`
          in a row is `{cond && (` across six — so per-line cells tore them apart and produced JSX that did
          not parse. **42 type errors**, found by `typecheck-emitted` and nothing else
    - [x] *`grid cols=3` was React-only.* It emitted `md:grid-cols-3` there and plain `grid gap-6` in html,
          svelte and wc — three columns in one representation and an unspecified number in the
          no-JavaScript build that ships to a browser. Now one shared `class_list`/`layout_classes`
    - [x] *`inline_row_bindings` requires one element per line*, and the first table row put them all on one.
          The substitution targets "the first `></` on the line", and `</span></td>` contains one — so the
          second column's value landed inside the first column's cell and the second rendered empty. The
          requirement is now stated at the function
    - [x] *The render check's own `<th` regex matched `<thead>`*, reporting `th without scope: <thead>`. A
          latent bug that could only surface once something emitted a `<thead>` — the same reason the
          assertion beside it had never run
  - [x] `fixtures/e.guml` had `state severity=all|urgent` over a `Job {…, failed:bool}`: no `severity` field
        and not the open/done idiom, so the compiler warned that the table was **not filtered at all**. The
        fixture demonstrated a control that did nothing. Naming the boolean (`all|failed`) makes it filter
- [x] **A repeater over a derived array** — `list matches of=Event`. The largest finding the GUML-Bench
      reference corpus produced, and the construct turned out to be small.

      A repeater's source had to be a declared **resource**, which made *more than one client-side filter
      inexpressible*: `where=` takes a single enumerated state, a predicate over three can only live in a `js`
      block, and that block's array could not be iterated. So `v01-event-filters` and `v02-cohort` filtered on
      the **server** and failed their own "one fetch, not one per change" criterion, each with a note in the
      file saying so. Both filter client-side now and both criteria pass.
  - [x] `of=` names the **row type**, so `{name}` resolves against `Event`'s fields exactly as a resource's
        rows do. A meaning change to an attribute that was read as an alternative *source name* and used by
        no fixture, no conformance case and no test — done before 1.0 and recorded here rather than slipped in
  - [x] `GUML0104` when the source is not a resource and there is no `of=`: nothing can infer the row type,
        because the compiler does not read a `js` body. Gated on the tag being in the *active* registry —
        at the `core` level `list` is not in the vocabulary at all, so `GUML0030` is the only useful
        diagnostic and adding "give it a row type" would send a repair loop to fix a tag it cannot use.
        Threading the registry into `validate` is what that cost
  - [x] Three defects on the way, each caught by a different thing:
    - [x] *`js` blocks emitted after the derived values.* `const visibleMatches = matches;` above
          `const matches = …` is a temporal dead zone error that throws on first render — an **ordering**
          bug, so neither the Rust tests nor `tsc` could see it. Blocks now emit before the derived group in
          both React and Svelte
    - [x] *The wc backend cannot support it, and now says so.* It emits a class body, so a `js` block has
          nowhere to live — it already reported that for `js` itself. The first attempt emitted
          `const rows = s.matches`, a read of `#state.matches` that is never assigned, so the list would
          have rendered its empty state forever with no diagnostic. It refuses and names `raw wc`
    - [x] *A derived source must not get the fetch scaffolding.* There is no request, so no `matchesLoading`
          and no `matchesError` exist; emitting either references an undeclared name — a compile error in
          React and a silent `undefined` in Svelte, which is worse. The empty state still applies, because
          "no rows matched" is a real thing to say about a derived array
  - [x] The escape-hatch ratchet **tightened** rather than loosened: `--max-escapes 1` per document, down
        from 4. Six of the nine reference answers use exactly one `js` block and three use none. The earlier
        comment described the flag as a total across the corpus, which it is not — corrected in the justfile
        and in CI
- [ ] Emitted code passes a Playwright test per fixture
- [ ] Emitted code passes `axe-core` with zero violations
- [ ] 20 additional fixtures compile and pass their tests

---

## Phase 4 — Component registry (6 weeks)

- [x] Builtin registry with `TagKind`, per-tag attrs, modifier vocabulary, typo suggestions
- [x] `guml registry --tags a,b` emits a retrieval-sized prompt block
- [x] **Grown to 49 primitives.** The vocabulary is now a data file (`crates/guml-registry/components.json`)
      rather than a `const` array, which is the same argument the theme table makes: a vocabulary compiled
      into a binary is one nobody can inspect, diff or publish — and it means the builtins travel the
      *same* load path a third-party package does, so a regression in `from_json` fails 400 tests instead
      of hiding. Added: `alert` `badge` `note` `avatar` `img` `divider` `skeleton` `grid` `sidebar`
      `toolbar` `breadcrumb` `pagination` `menu` `stepper`/`step` `stat` `progress` `option`
      `modal` `drawer` `toast`
  - [x] Every entry lowers in React, JSON and Svelte, and the three now share one **element table** —
        they had drifted, and `nav`/`hero`/`footer` were `<div>` in the static-HTML backend where React
        emitted landmarks, so the no-JavaScript build had none at all
  - [x] `select` emits its options. It emitted **none**, in every backend, and leaked the bound state's
        name as the element's text — a dropdown with nothing to choose. Options come from the state's
        domain or from `option` children, reconciled in one place
  - [x] `if={expr}` is lowered. It had been a declared global attribute lowered by *nothing*: it fell
        through to the DOM as a literal `if={open}` attribute, so a conditional element rendered always.
        No fixture used it, so neither the snapshots nor `typecheck-emitted` had ever seen one
  - [x] **shadcn/ui is the default theme** (`themes/shadcn.json` + `shadcn.css`), covering all 51 styled tags
        and every modifier. Class strings and tokens taken from shadcn's own `registry/new-york-v4/ui/*.tsx`:
        the button's six variants and four sizes, the input's border and ring treatment, the badge's tone set,
        the table's cell metrics, the `focus-visible:ring-[3px]` focus ring.

        The token names are shadcn's exactly — `--background`/`--foreground` pairs, `--primary`, `--secondary`,
        `--muted`, `--accent`, `--destructive`, `--border`, `--input`, `--ring`, `--card`, `--popover`,
        `--sidebar`, `--radius`, in `oklch`. So a host already running shadcn deletes the `:root` block and its
        own palette applies: **the variables are the interface**, which is the whole reason to theme in tokens
        rather than in colour literals. `slate` is still shipped, still tested, and selected with `--theme`.
    - [x] What a class table deliberately cannot carry, stated in the theme itself: Radix *behaviour*.
          shadcn's dialog traps focus and its select is a keyboard-navigable listbox — components, not
          classes. GUML emits a real `<select>` and a `<template>` in the no-JavaScript build, which is more
          honest than a `<div role="listbox">` with no key handling. A host wanting Radix points a registry
          package at its own components with `element`/`import` — `packages/guml-widgets` shows how
    - [x] **And now that package exists: `@guml/shadcn`.** The theme gave every document shadcn's *look*;
          this gives it the components a class table cannot express. `shadcn add --all` wrote all 61 real
          files (Radix, Base UI, cmdk, embla, recharts, react-day-picker, sonner, vaul — not a
          reimplementation, and `shadcn add <name>` still updates one in place), over Tailwind v4 CSS-first
          with the token blocks in `styles.css`.

          **26 tags**, ~600 estimated prompt tokens: only what GUML has no builtin for. `card`, `btn`,
          `input`, `select`, `table`, `tabs`, `modal` and the rest are already vocabulary and already wear
          these classes, a package may not shadow a builtin (`GUML0092`), and a second spelling of each
          would split every document's vocabulary in two.

          The interesting part is `src/guml/` — eight adapters. The compiler emits one shape for every
          `field`-kind tag (`value`, and `onChange` taking the *value*), which is what lets one lowering
          serve every field anyone contributes. shadcn's components carry their upstream primitive's API
          instead: Radix's Slider is `number[]` and `onValueChange`, a raw `<textarea>` is a React
          `ChangeEvent`, Base UI's Combobox is a compound of six elements. The reconciliation belongs in the
          package, in the language the components are written in — not as a table of prop spellings in the
          compiler (a copy of shadcn's API, stale the day shadcn changes, needing a branch per package) and
          not as a mapping language in JSON (which cannot express `number[]`, let alone a compound). That is
          *why* `element`/`import` point at a component rather than a DOM tag
    - [x] Typechecking the emitted TSX against **someone else's** components found four things, and the
          fourth was a compiler bug neither the audit nor `tsc` could have reported on its own:
          `DatePicker` declared in the registry but absent upstream (shadcn's date picker is a *recipe*
          composing Popover, Button and Calendar — there is no `date-picker.tsx`); `onChange` emitted as a
          value callback where a raw `<textarea>` wants an event; `value={n}` where Radix wants `number[]`;
          and **a `radio` emitted with no options at all** — bound correctly to a state and offering the
          reader no way to change it. An empty `<RadioGroup>` is valid TypeScript, which is exactly why it
          survived. A `field`-kind host component now receives its alternatives as `options`, reconciled
          from `option` children or the bound state's domain by the same function `select` uses, so the two
          spellings cannot disagree about one element. `just shadcn-test` and
          `cargo test -p guml-compiler --test package_shadcn` hold it
    - [x] 159 utilities implemented in `shadcn.css`, every one enforced present by
          `every_class_the_builtin_theme_emits_has_a_rule_in_its_stylesheet` — the static-HTML backend has no
          build step, so a class with no rule is a silently unstyled page
    - [x] Fixed the completeness check's own selector escaping, which had been wrong **twice** the same way:
          a denylist of characters to escape, missing `.` first (so `py-0.5` failed) and then `[`/`]` (so
          every arbitrary-value utility failed — `ring-[3px]`, `h-[1.15rem]`, which shadcn uses throughout).
          Both times the symptom was a *false* report telling an author to edit a correct stylesheet. Now an
          allowlist of what needs no escape, which cannot grow a bug per character
    - [x] Seven tests were pinning slate's **palette** rather than the property they were about —
          `bg-slate-900` where the claim was "a modifier selects a role". Retargeted to tokens, and the
          cross-backend agreement test now reads the expected classes *from the theme*, so it holds for any
          theme instead of breaking whenever the default changes
  - [ ] **58 colour literals are compiled into the backends**, bypassing the theme — the error banner is
        `bg-red-50 text-red-700`, the loading skeleton `bg-slate-100`, a `tier` card `border-slate-200`. This
        module's own docs say "a colour literal inside a compiler is a theme nobody can override", and these
        are exactly that: a host loading its own palette gets it applied to most of a page and not to those
        parts.

        Pre-existing, and invisible for as long as the default *was* slate, because the literals matched it.
        Making shadcn the default is what exposed it. Fixing it means ~14 pseudo-tag roles across five
        backends, and half of it would leave the backends disagreeing about one document, which invariant 8
        forbids — so it is one task rather than an opportunistic edit. **A ratchet holds the count** meanwhile
        (`no_new_colour_literal_enters_a_backend`, per file, verified to fail on an added literal).
        **4 are already gone**: `<body>` carried `bg-slate-50 dark:bg-slate-950 text-slate-900
        dark:text-slate-100`, which made the *page background* the one thing no theme could change. The
        theme's reset sets it from `--background`/`--foreground` instead, so that one was a deletion rather
        than a new pseudo-tag
  - [x] `chart`, `calendar`, `date`, `upload`, `command` **ship as a registry package**
        (`packages/guml-widgets`) rather than as builtins. Still not builtins, and for the original reason:
        no honest lowering exists without a design decision the registry should not make for a host, and
        there is no neutral answer to "which charting library". A package is the answer to exactly that
        shape of problem, so this doubles as the worked example `spec/REGISTRY.md` describes.

        Every entry uses a PascalCase `element` with an `import`, and `src/index.tsx` is a small real
        dependency-free implementation — so the package is *provable* rather than illustrative. `wizard`
        stays out: it is a flow across several documents, which is a routing question and not a component.
    - [x] The check that earns its place: `pnpm typecheck:example` compiles the package's own example and
          typechecks the **emitted component against the components**. A package declares `attrs`, the
          compiler emits them as props, and nothing verified the component accepted them. It found three
          compiler bugs on its first run, all silent:
      - [x] `of=revenue` and `kind=line` were **dropped**. The React attribute loop encodes what each name
            means *for a builtin* — `of` belongs to a repeater, `kind` folds into an `<input>`'s `type` — and
            it applied that to a component the compiler knows nothing about. Two declared props gone, no
            diagnostic, a chart plotting nothing. A host component's declared attrs now pass through verbatim
      - [x] The title positional never reached `aria-label`, so a component whose entry says
            `requires_label` was emitted with **no accessible name**. The compiler enforced the contract on
            the document and then dropped it on the way out
      - [x] `date from` emitted the state *name* as children instead of `value`/`onChange`. Only `input` and
            `select` were wired for two-way binding, so a package's own `field` was decorative — the same
            shape as the `select` that once leaked its bound state name as element text
    - [x] `if=` and the other compiler-owned globals are never forwarded as props. Forwarding `if=` put an
          unknown attribute on someone else's component *and* guarded the same subtree twice
    - [x] `guml registry --validate` accepts a **directory** like `guml add` already did. They disagreed, so
          auditing before installing — the order anyone would use them in — failed with "Access is denied",
          which reads like a permissions problem rather than a path convention
    - [x] `tests/package.rs` proves the compiler half without Node, so `cargo test` alone catches a
          regression
- [x] Per-entry a11y contract — `A11y { requires_label, role, focusable, announces_state }`, declared
      per component so a *loaded* entry states what the compiler must guarantee rather than the promise
      stopping at the builtin vocabulary
- [x] JSON registry packages — `ComponentDef` is owned rather than `&'static`; `Registry::from_json`
      / `extend_from_json` / `to_json`, `--registry`. Shadowing a builtin, an unusable tag name and an
      app-level entry in a core host are rejected rather than merged
- [x] Theme packs — `crates/guml-codegen/themes/*.json` + `--theme`, so `primary` can mean *the org's*
      primary. A theme declares a focus treatment and a contrast floor and is refused without them,
      which is what keeps a themeable compiler able to promise accessible output; it may also carry the
      stylesheet the no-JavaScript backend inlines. `guml theme --classes` lists what a host's build has
      to keep, because a utility-class framework cannot discover classes produced at runtime
- [x] **Per-entry metadata**: `children` (allow/deny/require), `slots`, `capabilities`
      (`needs_runtime`/`network`/`storage`/`backends`), `positionals`, `element`/`import`, `since`, and an
      `approx_prompt_tokens` estimate per entry. Each is declared *by the entry* rather than known by the
      compiler, which is the only version of a rule a loaded third-party component can also use:
      `select` accepts only `option` because its own entry says so, and a host's `combobox` gets the same
      checking for free. Two codes, `GUML0099`–`GUML0100`
  - [x] `positionals` was not a nicety. Without it `btn Add task primary` compiled with **zero
        diagnostics** and emitted `<button>Add</button>` — the word `task` deleted from the output. Four
        instances existed in this repo's own `portfolio.guml`, one truncating the author's name to `Ravi`.
        Now `GUML0099` with an applicable quoting suggestion, so `guml fix` repairs it with no model call
  - [x] `capabilities.needs_runtime` is what the static-HTML backend now checks, rather than a list
        inside that backend — so a `modal` and a host's own `combobox` are refused by the same three lines
- [x] **Registry packages first-class**
  - [x] `guml add <path>` — audits, checks against the project's existing vocabulary (two packages can
        each be valid alone and collide), then records it in `guml.json`
  - [x] `guml registry --validate` reports *every* problem at once, for the same reason the parser does
  - [x] `guml registry --docs` generates Markdown reference for the active vocabulary
  - [x] `guml.json` states the project's registries, theme, backend and conformance level **once**.
        Before it, every `check`, `build`, editor and CI invocation had to be handed the same paths, and a
        document could be valid in the editor and invalid in CI — the worst failure a closed vocabulary
        can have, since the point of closing it is that everyone agrees what the words are
  - [x] An entry declares **what it lowers to**: a lowercase `element` is an HTML element, a PascalCase
        one is the host's own component, emitted with a generated import. Without this a package bought
        validation and no output — `guml check` accepted `callout` and `guml build` warned "does not yet
        lower". Deliberately no network: a registry decides what a document may say and what classes get
        emitted, so resolving one over HTTP at build time is the wrong trade for this project's claim
  - [x] **`spec/REGISTRY.md` documents the schema**, and the rewrite was overdue rather than cosmetic: the
        old version's "Not yet in the schema" section denied three features that had shipped — `children`
        constraints, a version field, and per-entry token cost — while omitting `positionals`,
        `capabilities`, `element`/`import`, `slots` and `since` entirely. A spec that describes a smaller
        language than the compiler implements is the same drift as a stale tag table, and a registry author
        reading it would have concluded the extension points did not exist.

        Every claim in it was verified against the compiler rather than written from the source: a
        seven-entry package that trips all four error classes and all three warning classes, and a working
        two-entry package compiled end to end to confirm `<Callout>` arrives with its import and `figurebox`
        lowers to `<figure>`. The audit output in the file is pasted from that run.
  - [x] **Version pinning** in `guml.json`: a registry entry may be `{ "path": …, "version": … }`, and
        loading **fails** rather than warns when the package declares a different version. `guml add`
        records the pin from the version it just audited, so the common path needs no extra step.

        Refusing rather than warning because a document compiled against the wrong vocabulary is not a
        degraded build — it is a different document, and the failure would surface somewhere unrelated. The
        check runs *before* the vocabulary is extended, so a mismatched package never contributes a tag.

        **Exact equality, not a range.** A range needs a resolver, a lockfile and a policy for what
        "compatible" means for a vocabulary, and semver's answer — additive is a minor bump — is the one this
        project has evidence against: adding a tag is not purely additive, because a `def` may not shadow
        one, and growing the vocabulary 28 → 49 broke exactly that in three places here.

        A bare path still serialises back as a bare path, so `guml add` does not rewrite a hand-written
        config into a shape its author did not choose. The docs page advised "pin a registry" for a while
        before any of this existed; advice a tool cannot carry out is worse than none
- [ ] Retrieval layer: measure prompt-cost vs vocabulary size (the mechanism works; the *measurement* of
      how slice size scales with a 49-entry vocabulary is not done — see the budget note below)

**The 3,000-token budget got tight, and that is worth recording.** Growing the vocabulary 28 → 49 pushed
the largest assembled prompt from ~2,964 to ~3,221 est. tokens, over invariant 5. It fits again at
**~2,961** because the spec stopped enumerating the vocabulary — the assembled prompt already appends a
generated `Available tags` block, so the table was a duplicate that could drift — and because maintainer
notes moved into an HTML comment that `readSpec` strips. The lesson generalises: the spec should carry
*rules*, and the registry slice should carry *vocabulary*, or the two compete for the same budget.

**GATE**
- [ ] Registry covers ≥90% of element needs across GUML-Bench without escape hatches

---

## Phase 5 — LLM integration (6 weeks)

- [x] `--format json` diagnostics designed for machine consumption
- [ ] Prompt assembly: cache-optimised layout (stable spec first, volatile task last)
- [ ] Grammar prompting harness (Wang et al., NeurIPS 2023) — the in-context DSL teaching baseline
- [⛔] Grammar-constrained decoding via `llguidance` for local/open models — needs a **local/open model**.
      Hosted APIs expose no client-side CFG masking, which is why arm T3 runs as *T1 + repair* and is
      labelled that way rather than as T3
  - [x] Note honestly: hosted APIs expose no client-side CFG masking, so API arms use structured output +
        repair instead. Recorded where it can mislead rather than only where it is convenient:
        `schema.mjs` carries it as arm T2's `unavailable` reason, so `report.mjs` omits the arm *with the
        reason* instead of printing a seven-arm table labelled nine, and T3 is reported as "T1 + repair"
        because calling it T3 would imply constrained decoding was involved
- [x] **Free repair layers** (`bench/gen/lib/pipeline.mjs`): sanitise → `guml fmt` → `guml fix`,
      all deterministic, no model call. Measured: 1 of 6 generations fixed outright, another
      from 8 errors to 2
- [x] `guml fix`: applies every unambiguous suggestion, refuses to replace a line span with a
      bare word, bounded re-check rounds
- [x] Repair loop with one model round, measured over trials — 7 of 9 attempts failed to
      improve and 2 made things worse, so an attempt is discarded unless it lowers the error
      count
- [x] **Repair loop bounded at 3 rounds and wired into the product**: `guml repair` (and `repair()` in the
      npm package) run sanitise → format → fix, report which layer did the work, and discard any layer
      that would *raise* the error count — the same rule the measured model round already used, applied to
      the free layers because "deterministic" is not "always an improvement".
      A fenced, prose-wrapped, HTML-shaped generation goes from 7 errors to 0 with no model call.
  - [x] The sanitiser moved out of `bench/gen/lib/pipeline.mjs`. That mattered more than deduplication:
        the *measured* pipeline could unwrap a ``` fence the shipped tool still choked on, so the harness
        was more capable than the product — the one direction a benchmark must not be wrong in
  - [x] **HTML-habit table**: `div`→`col`, `span`→`text`, `button`→`btn`, `ul`→`menu`, `hr`→`divider`, and
        30 more, each with a note on why the language has no such tag. Edit distance reaches none of them
        (`button`→`btn` is three edits), so before this the most common wrong tag in generated GUML got no
        suggestion at all and cost a full generation to fix
  - [x] Fixed an *unsafe* auto-fix found while testing the above: `btn Click me` warned that `me` was a
        mistyped `md` and attached it as applicable, so `guml fix` rewrote the label to `Click md`. At
        distance 1 any two-letter word matches one of the five two-character modifiers, so the heuristic
        carried no signal at that length; it now requires four characters, and the short case surfaces as
        `GUML0099` (quote the label) instead — a safe suggestion rather than a destructive one
- [x] Auto-apply `suggestion` fields without a model call (`applyAllSuggestions` in the JS package)
- [⛔] Telemetry: tokens in/out per attempt, cached vs uncached, repair rounds, time-to-valid — the
      plumbing is a code task, the *numbers* need generations

**GATE**
- [⛔] ≥95% valid GUML from Sonnet 5 within ≤1 repair round — needs a funded account
- [⛔] Measured spec/registry prompt tax reported separately from generation tokens (report §7.3) — the
      assembly is done and its estimate is enforced in CI; the *measured* figure needs the target model's
      tokenizer

---

## Phase 6 — GUML-Bench and evaluation (10 weeks)

- [~] **The harness is built and runnable** (`bench/guml-bench/`): task schema with validation, the nine
      arms declared with a reason attached to each of the three that cannot run, the model grid, a metrics
      module, per-category reporting, and a preflight that refuses a malformed dataset. **12 of the 150
      tasks**, two per category — enough to exercise it end to end and not enough to publish a
      per-category figure from, which is why `report.mjs` prints the `n` beside every number. All 12 now
      have an authored GUML answer, so the report measures the whole dataset it describes
  - [x] The two rules the harness *enforces* rather than documents: **one prompt per task, shared by
        every arm** (a task carrying a per-arm prompt is rejected — that is the defect that rigs a
        comparison in a way no aggregate reveals), and **no overall average, ever** (the content floor
        makes one actively misleading, and `report.mjs` has no code path that prints one)
  - [x] A measurement hazard the first report run surfaced, now printed as a caveat every run: the
        compression ratio **rises when the compiler generates more code**. `c01-tasks` reads ~15.9× where
        the report publishes 8.10× *for the same fixture*, because the compiler gained a response cache.
        A ratio with "code the compiler writes" as its numerator is gameable by writing more of it, so it
        is a size measurement and not a quality one
  - [x] **An authored GUML reference for every task** (`bench/guml-bench/reference/`), so `report` measures
        12 of 12 rather than 3 of 12. Three point at repository fixtures; nine are new documents. Every one
        compiles with no error, formats clean, and its emitted TSX passes `tsc --strict`.

        **Writing them was the best bug-finding exercise this project has run.** Nine documents produced
        ten defects, and every one was *silent* — the compiler accepted the document and emitted something
        wrong, which is the failure class invariant 3 exists to prevent. A fixture is written to exercise a
        feature somebody already thought of; a whole task answer is written to be correct and goes wherever
        the task goes.
    - [x] `{invoices.paid.amount.sum}` — the sum of a field over filtered rows, which every dashboard and
          cart total needs — emitted `invoices.paid.amount.reduce(…)`. `.paid` on an array is `undefined`,
          so it threw at runtime. Only the field *immediately* before the aggregate was recognised
    - [x] `{members.admin.count}` counted rows where `it.admin` is truthy, on a row type with no `admin`
          field: permanently zero, and the banner it guarded was permanently visible. Now `GUML0065`
    - [x] A state interpolated into a request URL reached `fetch` with its braces intact —
          `?channel={channel}` asked the server for a channel literally named `{channel}`. Fixed, and the
          emitted `useCallback`'s dependency array is now *derived* from the URL, because the first version
          fixed the URL and left a stale closure
    - [x] `>channel = all` on `state channel=all|web|ios` emitted `setChannel(all)` — an undefined
          identifier. A bare domain member is a string
    - [x] A numeric field assigned a string: `input qty kind=number` on `state qty=1` emitted
          `setQty(e.target.value)`. `TS2345`, and at runtime a number that becomes a string on the first
          keystroke
    - [x] `data subscription:Subscription` — a single object rather than a list — emitted
          `useState<Subscription[]>([])`. Now `GUML0103`, with the expressible spelling in the message
    - [x] An `optimistic` mutation with no row in scope emitted `it === item` with no `item` declared. The
          patch applies to every row now, which is the only reading a collection-level endpoint supports
    - [x] `badge danger Breaking` rendered the string "danger Breaking" while this tag's own registry doc
          said to use those modifiers for tone and `themes/slate.json` carried three tone rules keyed on
          them. `badge` takes positionals now; `GUML0102` covers the rest of the text kind
    - [x] `>editing = id` — copying a row id into a string state, the only way a per-row dialog knows which
          row it belongs to — was three false `GUML0065` errors, because the check judged the right-hand
          side by its spelling
    - [x] `where={query}` over a domain-less string state was two warnings and an unfiltered table. A
          free-text search over the row's string fields is lowered now, in all four backends, from one
          shared `search_fields`
    - [x] Two formatter defects found by formatting them: `on` was missing from the directive list, so an
          effect sorted below the element tree in canonical form and took a blank line above it; and the
          blank marking the declaration/tree seam was inserted *between* a comment and the line it
          introduces, stranding the comment
  - [x] **What the references could not express, recorded in each file's header.** This is the vocabulary
        evidence the escape-hatch rate is supposed to produce, and it is specific: an aggregate over an
        *expression* (a cart subtotal is `Σ unitPrice × quantity` and aggregates apply to fields); a
        repeater over a *derived* array, which is what blocks composing more than one client-side filter;
        a lookup into a collection by two keys; `select` options projected from fetched rows; counting rows
        by a value rather than by truthiness. Four hatches across twelve complete applications, budgeted at
        4 in CI as a ratchet.

        Two checklist items are failed *on purpose* and say so — `v01` and `v02` filter on the server
        because three composed client-side filters are not expressible. A reference answer that quietly
        claimed a criterion it does not meet would be worth less than nothing here
  - [x] The escape hatch composes with the language now: a `js` block's top-level `const`/`let`/`function`
        names are in scope for bindings. Without that the hatch was a dead end — a block could compute a
        subtotal and no binding could read it, leaving `raw <backend>` as the only route, and a document
        that has to pick one backend has given up being an IR
  - [x] **Arm B4 — TOON** (`bench/guml-bench/toon.mjs`), the arm the report names as the strongest
        compact-serialisation rival and the one that could have sunk the thesis. It encodes **the same
        payload B3 measures**, because a hand-tuned structure for this arm would measure the tuning.

        | | |
        |---|---|
        | TOON vs the identical JSON | **30% smaller** — the objection is real and worth this much |
        | GUML vs that TOON | **63% smaller** — the saving is structural, not punctuation |
        | TOON's tabular form reaches | **10% of object rows** — this IR's arrays are not uniform |

        The third row is stated in TOON's favour, and so is the note that key folding and alternate
        delimiters are unimplemented: these figures are a **lower bound** on the rival. `report.mjs` prints
        both caveats every run.
    - [x] A **decoder** ships with it, and `selftest.mjs` asserts encode → decode → deep-equal on all
          twelve payloads plus ten edge cases — a string `"true"` must not return as a boolean, a value
          containing a comma must survive, leading space must not be eaten. Without it, "TOON is 30%
          smaller" and "we deleted 30% of the characters" are the same claim
    - [x] In CI, because the *rival's* arm is the one that most needs a test: a bug that makes GUML look
          better is a bug nobody reports
  - [ ] Paired *human-expert React* references (arm B6). The GUML side is done; the React side is what
        gives a reviewer "the model did well" versus "the task was easy"
  - [ ] The remaining 138 tasks, 23 per category
- [ ] Seed realistic structures from Design2Code's 484 curated pages
- [~] Nine arms: B1 React · B2 HTML · B3 JSON IR · B4 TOON IR · B5 v0 · B6 human · T1 GUML · T2 +constrained · T3 +repair
      — **seven runnable**, each unavailable one carrying its reason in `schema.mjs` so a generated table
      cannot imply nine were compared
- [⛔] Model grid: Haiku 4.5 / Sonnet 5 / Opus 5 (capability is a first-class variable — H6) — needs a
      funded account
- [ ] Metrics harness: tokens, USD, latency, parse/compile rate, Playwright pass, visual similarity, axe-core, Lighthouse, bundle size, inter-run variance
- [~] **Edit-locality benchmark** (`bench/guml-bench/edit-locality.mjs`): 8 scripted modifications against
      *diff-based* React editing. The baseline matters more than the number — comparing regeneration to
      regeneration would just restate the compression ratio, and nobody edits React by regenerating it.
      Both sides are hand-written minimal diffs, so this measures the *representations*, not the models.
  - [x] The result shape is already informative and matches the report's prediction: 1.78× for adding
        prose (the content floor), 2.25× for a rename, **13.6× for adding an empty state** — the changes
        GUML wins on are the ones where the convention is the compiler's. Median 5.5× over 7 comparable
        edits
  - [x] One edit is not an edit in GUML at all: adding a loading state to a resource. The declaration
        already generated it. Scoring that as "0 tokens vs 6 lines" would be measuring the right thing and
        describing it wrongly, so it is reported with the reason instead of as a ratio
  - [ ] 50 tasks × 3 modifications, as the report specifies
- [⛔] Ablation grid: spec size × examples × constrained decoding × repair rounds × model — needs a funded
      account, and is the largest single spend in the plan
- [⛔] Human study, n≥30: pairwise code preference, readability, timed modification task — needs
      **participants**
- [⛔] Non-engineer study: spec readability — needs **participants**
- [⛔] Pre-register H1–H6 before running anything — needs a **registry entry someone can sign** (OSF or
      similar). The text is a drafting task; registering it is not. Must happen *before* any funded run, or
      pre-registration means nothing
- [x] Report per-category, never a single average — enforced by the absence of a code path, not by a note
- [⛔] Publish all raw generations, not just aggregates — blocked on there being generations. The harness
      already writes one file per run under `results/raw`, so this is a publishing step rather than a build
      one, and it is the step that lets a reviewer disagree with the scoring rather than only with the total

**GATE**
- [⛔] Statistically significant result on ≥3 of H1–H6, positive **or** negative — needs the sweep and the
      graders

---

## Phase 7 — Second backend and papers (8 weeks)

- [x] **Svelte 5 backend** (`--backend svelte`): runes rather than stores, so a `state` is `$state`, a
      `where=` filter is `$derived` and a resource is `$effect` — the same declaration that needs
      `useMemo` plus a hand-built dependency array in React needs neither here. Shares the theme table
      and the expression lowering with the other backends; optimistic apply and rollback are
      byte-comparable in meaning. 14 tests. Writing it found two real gaps: a form submit button was
      emitted as `type="button"` (a form no keyboard could submit) and a `check` toggle posted `{}`
      instead of the negated field
- [x] **Web Components backend** (`--backend wc`): a custom element, no framework and no build step —
      the portability target no other backend can serve. Also the fourth independent check on "GUML is an
      IR": React has hooks, Svelte has runes, static HTML has no runtime, and this has manual DOM
      updates, from one AST.
  - [x] **No shadow DOM**, and that is the interesting decision. The theme's classes are global
        utilities, and a shadow root isolates the document's stylesheet from them — so the component
        would render completely unstyled while looking, from outside, like everything worked
  - [x] Text and attributes update; a bound field's `value` is written **only** at first paint. Rebuilding
        `innerHTML` on every keystroke destroys the cursor, which breaks typing after the first character
  - [x] One delegated listener per event type, dispatched by a `data-g-act` index — so a repeater can
        replace its rows without leaking listeners or re-binding
  - [x] `scripts/check-wc.mjs` **runs** the output. No DOM library: the generated code touches a small
        known surface, and a shim of it executes `connectedCallback`, every binding and every action body
        for real. It caught a JSX interpolation leaking into a plain-JavaScript file on its first run
  - [x] Which forced a real fix in the shared expression lowering: prefixing state reads by rewriting the
        *lowered string* cannot tell an identifier from the contents of a string, the literal text of a
        template, or a lambda's own parameter — and got all three wrong at once. `Ctx::with_scope` now
        applies the prefix during lowering, where the tree says "path head"
- [x] **A2UI + MCP-UI emitters** — the strategic move the report argues for (§13): A2UI is
      simultaneously the strongest competitor and the strongest partner, so emitting it turns
      "GUML vs A2UI" into "GUML compiles to A2UI".
  - [x] A2UI: a flat, id-referenced component list with a pre-approved catalog. Non-executable **by
        construction** — a `js` block is refused rather than stripped, an action becomes a declared
        intent carrying the author's statements unlowered, and a resource becomes a declared data
        requirement the host may decline
  - [x] MCP-UI: no new format invented. The protocol's documented rendering modes are a sandboxed HTML
        iframe and a remote-DOM script, and the compiler already emits both — so the emitter *composes*
        the existing backends and picks between them by capability. A document needing no runtime becomes
        `text/html`; one that does becomes a remote-DOM custom element
  - [~] The A2UI payload targets the **shape** the report documents and is self-describing about it
        (`"format": "a2ui-shaped"`), because it was written from a description rather than against the
        published JSON schema. Pinning it to the schema is mechanical once the schema is vendored, and
        claiming conformance before then would be exactly the unsupported claim `CLAUDE.md` forbids
  - [x] Both uncovered two real bugs. `https://api.example.com/rows` lost its scheme in the lexer and
        arrived as `//api.example.com/rows` — a *protocol-relative* URL, so the emitted code fetched
        cross-origin while `validate::check_url`'s `starts_with("http")` branch sat unreachable. The
        lexer now tokenises `http`/`https` only, and a protocol-relative URL is `GUML0084`: a document
        that means "a path" should not be one character from meaning "somebody else's server"
- [x] WASM build of the compiler (`wasm-pack`, 298 KB) — `crates/guml-wasm`, shipped as the `guml` npm package
- [x] **`tower-lsp` language server** (`crates/guml-lsp`): diagnostics, semantic tokens,
      formatting, registry completion, hover, outline. Features are plain functions over text
      with 13 tests; the protocol layer is translation only
  - [x] **Document-level code actions.** The per-diagnostic quick fix is the wrong shape for the common
        case: a pasted generation has six unknown tags, and fixing them one keystroke at a time is six
        keystrokes for six edits the compiler had already described. `source.fixAll` applies them all —
        and is the kind an editor can be configured to run on save. `repair` is offered as a plain
        `source` action and never as `fixAll`, because it also *deletes* (a fence, trailing prose) and
        silently removing lines on save under an action named "fix" would be indefensible
  - [x] The VS Code extension is **in the pnpm workspace and typechecked in CI**. It was outside it, so
        its dependencies were never installed and `tsc` never ran on it — the one piece of first-party
        TypeScript in the repository with no typecheck, and the piece users install
- [⛔] Paper 1: *How Should LLMs Represent User Interfaces?* → EMNLP/ACL or NeurIPS D&B — blocked on
      results, and on an author to submit it
- [⛔] Paper 2: *Convention as Compression* → ICSE/FSE — blocked on results, and on an author to submit it
- [ ] Release GUML-Bench standalone as a dataset artifact

---

## Cross-cutting, always on

- [x] CI: five jobs, 24 steps — `fmt --check`, `clippy -D warnings`, `cargo test` (Linux + Windows),
      the LSP builds, benches compile, every `.guml` formatted and valid, emitted TypeScript typechecks
      *and* renders, the Phase 0 preflight and scoring self-test, the docs build with the highlighter
      parity / theme-class / doc-preview checks, and the npm package
- [x] `criterion` benches, now **calibrated**. Absolute milliseconds on a dev laptop are not a
      measurement — criterion reported a 100% regression on a function that had not been touched, and a
      build doing strictly *more* work as 22% faster. `calibration/reference` is a fixed pure-Rust
      workload measured in the same run, and the budget is the ratio: `check/200 ÷ calibration` held at
      **1.44–1.63** across runs whose absolutes spanned 1.19–3.77 ms. Per-stage benches attribute the
      cost rather than leaving it to argument: **lex 934 µs, parse 2.21 ms (lex included), analyse
      597 µs, check 2.30 ms** — the lexer is ~40% of `check`
  - [~] `check` latency. **Judge it as a ratio, never in milliseconds** — invariant 6 exists because
        `calibration/reference`, a fixed pure-Rust workload, has differed by 2× between two runs
        minutes apart. Latest: calibration 1.00 ms, `check/200` 1.26 ms, **ratio 1.25**, below the
        historical 1.44–1.63 band — so `GUML0102`, the field-chain aggregate, free-text `where=` and
        the single-object resource check cost nothing measurable.

        This line used to read "over the 2 ms budget (2.30 ms)". Deleting that is the point: the
        absolute now reads 1.26 ms, which anyone quoting milliseconds would call a 45% improvement on
        a compiler doing strictly more work. The lexer is still the largest single stage (686 µs of
        parse's 1.37 ms) and is where to look first if the ratio ever moves
- [x] **Capability manifest and CSP** (`guml capabilities`). `core`/`app` answers "may an untrusted agent
      send me this at all" — one bit, and far too coarse to act on. A host needs the origins a document
      will contact, whether it contains script, and whether it reads storage, because those are the terms
      a Content-Security-Policy is written in. All derived from the AST, so there is no declaration to get
      wrong.
  - [x] The CSP is **generated, not documented**. Prose telling a host "add your origins to `connect-src`"
        puts the compiler's own knowledge in a paragraph and asks a human to reproduce it; a wrong-by-
        omission CSP is the failure mode found in production. Where the compiler's own output needs a
        loosening (the static-HTML backend inlines its stylesheet) the policy says so with the reason
  - [x] `--assert-inert` is the safe-render gate: one command, one answer, and it names *which* property
        failed rather than sending a reader back to the manifest
  - [x] `--max-escapes` is the escape-hatch budget, wired into CI at 3 (`d.guml`'s count, and `d.guml`
        exists to exercise `js`/`raw`). A ratchet: lower it when the vocabulary grows enough to make a
        hatch unnecessary, and never raise it without saying which construct could not be expressed
- [ ] **Signed registry and theme packages.** Not attempted, and deliberately: it needs a signing scheme
      and a key-distribution story, and picking either without input would be inventing policy rather than
      implementing it. `guml add` refusing network fetches is the mitigation that does not need a design
      decision — a package arrives as a file, from wherever the project's existing supply chain puts it
- [ ] Every claim in `README.md` traceable to a test or a measurement
- [x] Escape-hatch rate tracked continuously — a warning nothing fails on is a statistic, so this is a
      **ratchet in CI**: `--max-escapes 3` per fixture and `--max-escapes 1` per GUML-Bench reference answer,
      each raisable only by naming the construct that forced the hatch. `report.mjs` prints the count every
      run, and every hatch in the reference corpus is named in its file's header with what could not be
      expressed
