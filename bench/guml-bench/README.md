# GUML-Bench

The benchmark from the research report §8, as a runnable artifact. **12 of the 150 tasks it specifies.**

That number leads because it is the most important thing here. Everything below works; the dataset is a
seed set. `preflight.mjs` prints the coverage, `report.mjs` prints the `n` beside every figure, and neither
will produce a per-category number without saying how thin it is.

```sh
node bench/guml-bench/preflight.mjs        # is it runnable, and how big is it really
node bench/guml-bench/report.mjs           # what can be measured with no API key
node bench/guml-bench/edit-locality.mjs    # cost of a *change*, not of a first draft
```

## What is here

| file | what it is |
|---|---|
| `schema.mjs` | The 6 categories, the 9 arms, the model grid, and the validation that keeps a task honest |
| `tasks.mjs` | 12 tasks, two per category, each with a prompt and a scoring checklist |
| `metrics.mjs` | Everything measurable without a browser or a key; everything else returns `null` **with a reason** |
| `edit-locality.mjs` | 8 scripted modifications, measured as diffs against diff-based React editing |
| `preflight.mjs` | Refuses a malformed dataset; reports coverage and which arms cannot run |
| `report.mjs` | Per-category figures, never an overall average |

## The two rules the harness enforces

**One prompt per task, shared by every arm.** If the React arm is asked for less than the GUML arm the
comparison is rigged, and it is rigged in a way no aggregate can reveal. `validateTask` rejects any task
carrying a per-arm prompt or checklist.

**No overall average, ever.** The content floor makes one actively misleading: a landing page is mostly
prose, prose is incompressible, and its ratio asymptotes at 2–3× while a CRUD app approaches 8×. A mean over
both describes neither and moves with the *category mix* rather than with anything about the language.
`report.mjs` has no code path that prints one.

## What the compression ratio is, and what it is not

It is a **size** measurement. It rises when the compiler generates *more* code, which makes it gameable by
emitting boilerplate — the opposite of a quality signal.

This is not hypothetical. `c01-tasks` currently reads ~15.9× where the report publishes 8.10× **for the same
fixture**. Nothing about the language changed; the compiler gained a response cache, so the emitted side
grew. `report.mjs` prints that caveat every run.

The two figures are also produced by different counters — the published ones use `cl100k` on a fixed
compiler revision, these use a ~3.6 chars/token estimate on the current one — so they must never be quoted
together. See `.claude/skills/guml-measure`.

## Arms

Seven of nine run from this repository. The other two are recorded with the reason, so a generated table
never implies nine were compared:

- **B5 v0** — needs API access this harness does not have.
- **T2 grammar-constrained decoding** — hosted APIs expose no client-side CFG masking, so it needs a local
  model with `llguidance`. T3 therefore runs as *T1 + repair* and is labelled that way.

## What is not measured, and why

Stated rather than approximated, because a benchmark reporting an estimate where it promised a measurement
is worse than one with a visible gap:

- **Every generation metric** — model parse rate, repair rounds, USD, latency. Needs an API key. This is the
  measurement the whole project turns on, and it is the one this harness cannot make alone.
- **Playwright pass rate, visual similarity, Lighthouse** — need a browser.
- **Blind human semantic scoring** against each checklist — needs a grader. The rubric and the blinding
  protocol already exist in `bench/phase0/rubric.md` and apply unchanged.

## The reference answers, and what writing them found

`reference/` holds an authored GUML answer for every task. Three point at repository fixtures so their
numbers connect to the published token counts; nine are documents written for this. Each compiles with no
error, formats clean, and its emitted TSX passes `tsc --strict`.

They exist so `report.mjs` measures the dataset it describes rather than a quarter of it. But the more
useful thing turned out to be writing them: **nine documents produced ten compiler defects, and every one
was silent** — the document was accepted and the output was wrong. A fixture is written to exercise a
feature someone already thought of. A whole task answer is written to be correct, and it goes wherever the
task goes: two-step aggregates, a state read into a request URL, a per-row dialog, a numeric field, a
single-object endpoint. Every one of those was broken.

The full list with diagnoses is in `ROADMAP.md` under Phase 6. Four of them became new compiler behaviour
(`GUML0102`, `GUML0103`, free-text `where=`, `js`-declared names in scope) and the rest were fixes.

### What they could not express

Each file's header names it, and this is the vocabulary evidence the escape-hatch rate is supposed to
produce:

| task | what has no spelling |
|---|---|
| `e01-cart` | an aggregate over an *expression* — a subtotal is `Σ unitPrice × quantity`, and aggregates apply to a field |
| `e02-product` | a lookup into a collection by two keys |
| `s01-team-settings` | counting rows by a *value* rather than by truthiness |
| `s02-billing` | the first row of a single-object endpoint |
| `v01`, `v02` | a conjunction of more than one filter — `where=` takes a single enumerated state |
| `v02-cohort` | `select` options projected from the fetched rows |

Six of the nine use exactly one `js` block; three use none. Budgeted at **one per document** in CI as a
ratchet — never raised without naming the construct that forced it. (`--max-escapes` is per document, not a
sum across the corpus; an earlier version of this note had that wrong.)

### The gap this corpus closed

This table had a seventh row: *a repeater over a derived array*. It was the largest finding here, because it
made **more than one client-side filter inexpressible**. `where=` takes one enumerated state; a predicate over
three can only live in a `js` block; and that block's array could not be iterated, because a repeater's source
had to be a declared *resource*. So `v01` and `v02` filtered on the **server** and failed their own "one
fetch, not one per change" criterion, each with a note saying so.

`list matches of=Event` closes it — `of=` names the row type, so a `js`-computed array iterates with its
fields resolving exactly as a resource's rows do. Both files filter client-side now and both criteria pass.
That is what this corpus is *for*: it named a specific missing construct, and the construct turned out to be
small.

Two costs, both recorded rather than hidden. `js` blocks now emit **before** the derived values in React and
Svelte, because `const visibleMatches = matches` above `const matches = …` is a temporal dead zone error that
throws on first render. And the **wc backend refuses** a derived repeater: it emits a class body, so a `js`
block has nowhere to live — the first attempt emitted a read of `#state.matches` that is never assigned, so
the list would have rendered its empty state forever. It reports and points at `raw wc`.

## Arm B4 — TOON, and the objection it exists to answer

The first objection to a UI IR is "why not just emit JSON", and B3 answers it with a real A2UI-shaped
payload the compiler emits itself. The second is sharper: JSON is a *verbose serialisation*, so maybe the
saving is the encoding rather than the language. TOON is the strongest form of that objection, and
`toon.mjs` implements it against **the same payload B3 measures** — a hand-tuned structure for this arm
would measure the tuning, not the format.

Measured over the twelve reference answers:

| | |
|---|---|
| TOON vs the identical JSON | **30% smaller** — the objection is real and worth this much |
| GUML vs that TOON | **63% smaller** — which is the answer: the saving is structural |
| TOON's tabular form reaches | **10% of object rows**, because this IR's arrays are not uniform |

That last row is stated in TOON's favour and matters. The format's headline feature is a uniform array
declared once and emitted as rows; the A2UI-shaped `components` array is not uniform — a `head` node has
`text`, a `form` has `children`, an `input` has `bind` and `properties` — so most of the 30% is dropped
punctuation rather than the feature. **Key folding and alternate delimiters are unimplemented here**, so
these figures are a *lower bound* on how well TOON does. Both facts are printed by `report.mjs` every run.

`toon.mjs` also ships a **decoder**, and `selftest.mjs` asserts encode → decode → deep-equal on all twelve
payloads plus ten hand-written edge cases (a string `"true"` must not come back as a boolean; a value
containing a comma must survive; leading space must not be eaten). Without that, "TOON is 30% smaller" and
"we deleted 30% of the characters" are the same claim. The rival's arm is also the one that most needs a
test, because a bug that makes GUML look better is a bug nobody reports.

## Building it out

To reach the report's spec, in order of what each addition buys:

1. **Paired human-expert React references** (arm B6). The GUML side is done — every task has an authored
   answer in `reference/` and `report.mjs` measures all twelve — but without a hand-written React ceiling a
   reviewer cannot separate "the model did well" from "the task was easy".
2. **The remaining 138 tasks**, 23 per category. Seed realistic structures from Design2Code's 484 curated
   pages, as §8.1 says, so the numbers are comparable to existing literature.
4. **Playwright + axe-core + Lighthouse**, which turns the checklists from a human instrument into a partly
   automatic one.
5. **Pre-register H1–H6** before running any of it. The report says so, and doing it afterwards is how a
   negative result becomes an exploratory one.
