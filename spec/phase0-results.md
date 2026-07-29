# Phase 0 — results

**Status: not yet run. The gate is undecided.**

This document exists so that the answer has somewhere to go and so the parts that *are* settled
are written down. Everything below is either measured with a stated method, or marked as
outstanding. Nothing is estimated into a conclusion.

The protocol is `spec/PHASE0.md`; the harness is `bench/phase0/`.

---

## The gate

From `spec/PHASE0.md`, all three must hold to continue:

| # | Criterion | Status |
|---|---|---|
| 1 | ≥80% of Sonnet 5 generations at 3 examples are parseable GUML | **no data** |
| 2 | Median output-token reduction ≥3× vs paired React on structure-heavy tasks | **no data** |
| 3 | Semantic correctness not worse than the React baseline | **no data** |

Criteria 1 and 2 need 90 generations against an Anthropic key. Criterion 3 additionally needs a
human grading blind against `bench/phase0/rubric.md`. Neither has happened.

**Do not soften these after seeing results.** That instruction is in the protocol because the
same people wrote the language, the benchmark and the compiler.

---

## What is settled

### The prompt fits the budget it promised

The spec commits to ≤3,000 tokens for spec + registry slice + examples. Measured by
`bench/phase0/preflight.mjs`, the largest assembled prompt is the landing task at three
examples: **~2,831 estimated tokens** (~3.6 chars/token heuristic — an estimate, and labelled
as one everywhere it appears). Preflight fails the build if any prompt exceeds the budget, so it
cannot drift silently as the spec grows.

### Artifact token counts

Measured with `node scripts/count-tokens.mjs`. `cl100k_base` and `o200k_base` are **OpenAI**
tokenizers, reported for continuity with the figures already published; they undercount Claude.
The authoritative column requires `ANTHROPIC_API_KEY` and is outstanding.

| fixture | React+TS+Tailwind | GUML | reduction | ratio |
|---|---:|---:|---:|---:|
| `a` counter card | 368 | 64 | 82.6% | 5.75× |
| `b` task CRUD | 1,434 | 173 | 87.9% | 8.29× |
| `c` landing page | 1,648 | 376 | 77.2% | 4.38× |
| total | 3,450 | 613 | 82.2% | 5.63× |

`o200k_base` agrees within about a point (83.2% / 88.1% / 77.9%), so this is not a tokenizer
artifact. GUML vs the same app as a minified JSON UI IR: **173 vs 315 — 45% smaller**.

**These are authored artifacts, not generations.** They say what the representation costs, not
what a model can produce. That distinction is the entire reason Phase 0 exists.

### The harness is built and self-tested

| stage | state |
|---|---|
| 10 task specs across the six categories, with checklists | done |
| 10 paired React references, all typechecking under `--strict` | done |
| Prompt assembly with the stable prefix cached | done |
| Preflight: budget, example leakage, registry slices, references | done |
| Mechanical scoring: parse validity, escape hatches, tokens, latency, prompt tax | done |
| Blind scoresheet + written rubric | done |
| Self-test over synthetic generations of known shape | done |
| **90 generations** | **needs an API key** |
| **Blind correctness grading** | **needs a grader** |

`just phase0-verify` runs everything that needs no key.

---

## Adjacent evidence — not Phase 0

A separate experiment (`bench/gen/`) generated six applications through **NVIDIA-hosted Llama
models**, not the Claude tiers this study specifies. It does not satisfy any gate criterion and
must not be reported as if it did. It is recorded here because it makes two concrete predictions
about what Phase 0 will find.

Full detail in `bench/gen/FINDINGS.md`. In summary:

- **Requirements were mostly met; parsing mostly failed.** 32 of 37 functional requirements
  across six apps, but only 1 of 6 documents compiled. The models understood the applications
  and failed on surface rules.
- **One failure class is the language's fault.** Both an 8B and a 70B model write `option`
  children under `select`, unprompted, at the same rate — and an explicit prompt rule did not
  stop it. GUML puts a dropdown's domain on the state. Expect this to show up in Phase 0's
  escape-hatch rate.
- **Free deterministic repair does real work.** Sanitise → `guml fmt` → `guml fix` fixed one
  document outright and took another from 8 errors to 2, with no model call.
- **A repair round is not reliable at this tier.** Of nine attempts, seven failed to improve and
  two made the document *worse*. Phase 5's "≥95% valid within ≤1 repair round" target should be
  treated as unproven until measured on the actual model tiers.

If Phase 0 finds `option`-style vocabulary failures dominating the escape-hatch rate, that is a
**language** result, not a model result, and the fix is the registry rather than the prompt.

---

## To produce this document for real

```sh
export ANTHROPIC_API_KEY=…
just phase0-verify                 # harness integrity, no key needed
cd bench/phase0
node run.mjs                       # 90 generations, resumable
node score.mjs                     # mechanical report + blind scoresheet
# fill the score column in results/scoresheet.csv — see rubric.md, do not open keymap.json
node score.mjs                     # re-read; the sheet is never overwritten
node ../../scripts/count-tokens.mjs   # authoritative token column
```

Then replace the "no data" rows above with the measurements, state the verdict, and — per the
protocol — **write the negative findings first**.

## How to read a failure

From `spec/PHASE0.md`, each outcome has a predetermined reading, decided before any data existed:

| outcome | reading | action |
|---|---|---|
| All three pass | The thesis survives first contact | Proceed; this becomes Paper 1's preliminary section |
| Tokens win, correctness loses | The low-resource-DSL penalty dominates | Try grammar prompting; if it still loses, GUML is a cost optimisation, not a reliability one |
| Wins on Haiku, vanishes on Opus | Expected per *Capacity, Not Format* | Still publishable; commercially it points at cheap-model-plus-compiler |
| Escape-hatch rate >30% | The vocabulary is too small for real work | Fix the vocabulary before anything else |
| Nothing works | The idea does not hold | Publish the negative result — it answers an open question and cost two weeks |
