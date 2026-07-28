# Phase 0 harness

The kill-or-continue spike from `spec/PHASE0.md`, mechanised. No compiler in the
generation loop: this measures whether a model can produce valid, correct GUML from a
spec in context, and whether the token saving survives real generations.

Everything here runs today. The generations do not exist yet — they need an API key and
about 90 calls.

## What is in here

| Path | What it is |
|---|---|
| `tasks/index.mjs` | 10 tasks: prompt, category, registry tags, and a 12–14 item checklist that *is* the scoring instrument |
| `references/*.tsx` | A hand-written React + TS + Tailwind implementation per task. All 10 typecheck under `--strict` |
| `examples/*.guml` | The three in-context examples. None of them is a task's answer |
| `lib/prompt.mjs` | Prompt assembly: spec + registry slice + examples, laid out so the stable prefix is one cache hit |
| `preflight.mjs` | Validates the harness without an API key |
| `run.mjs` | The sweep. Resumable, one file per run |
| `score.mjs` | Mechanical scoring, gate check, and a **blind** human scoresheet |
| `selftest.mjs` | Scores synthetic generations with known properties, so scoring bugs surface before the API bill |
| `rubric.md` | The human scoring scale, the blinding procedure, and the ways a scorer cheats without meaning to |

## Running it

```sh
node preflight.mjs        # harness integrity — no key needed
node selftest.mjs         # scoring correctness — no key needed
node run.mjs --dry-run    # assemble all 30 distinct prompts to results/prompts

export ANTHROPIC_API_KEY=…
node run.mjs              # 90 generations, resumable
node score.mjs            # report + blind scoresheet
# fill the score column in results/scoresheet.csv, see rubric.md
node score.mjs            # re-read; the sheet is never overwritten
```

Narrow it while iterating:

```sh
node run.mjs --tasks t01-crud,t02-dashboard --models sonnet --examples 3 --arms guml
node run.mjs --repeats 3   # variance across identical prompts
```

## The design decisions that matter

**Both arms get the same prompt.** The task text never mentions GUML or React; only the
output rules differ. Asking the baseline for less is the easiest way to fake this
result, and `preflight.mjs` fails if a task prompt mentions GUML.

**90 runs, not 120.** The React arm has no spec and no examples, so the example-count
variable does not apply to it: 10 tasks × 3 models × 2 example counts for GUML, plus
10 × 3 for React.

**Thinking is off.** Extended-thinking tokens land in the same output counter as the
artifact, which would make the headline token number mean something other than what it
says. If a later phase wants thinking, it is a separate arm with its own column.

**Temperature 0.** Reproducibility first; `--repeats` is how variance gets measured,
not sampling noise mixed into a single figure.

**Three examples, none of them an answer.** The three examples cover state and actions
(counter), a form with validation (sign-in), and a data resource with a repeater
(invoices). `preflight.mjs` fails if an example ever becomes byte-identical to a task's
fixture answer.

**The registry slice is a variable.** A per-task slice that contains exactly the tags
the answer needs is a hint the full registry does not give. `--full-registry` runs the
harder condition, and the flag is recorded in every result file.

**The prompt budget is checked.** `spec/PHASE0.md` commits to a spec that fits in
≤3,000 context tokens. The largest assembled prompt is ~2,831 estimated tokens
(`t03-landing`, 3 examples); preflight fails if any prompt exceeds the budget.

## Known confounds, stated up front

- **Content-heavy tasks get no content-heavy example.** All three examples are
  app-shaped. If the landing and docs tasks underperform, that is a candidate
  explanation and not evidence about the language.
- **The React arm has nothing to cache.** Its input is a short system prompt; GUML's is
  ~2.8k cached tokens. The asymmetry is real, is what a user would actually pay, and is
  reported as two separate columns rather than a single net figure.
- **The references and the tasks share an author.** Same limitation as the report's
  token measurements. A second author writing the references would be a real
  improvement and is out of scope for two weeks.
- **n=1 per cell at the default `--repeats`.** Ninety runs answer "does this work at
  all", not "how reliably". Phase 6 is where n gets large enough for statistics.

## What this harness cannot do

It cannot call the models, and it cannot score correctness. Both need something outside
this repo: an API key with budget, and a person willing to grade 60-odd generations
blind against a checklist. Everything up to and after those two steps is automated and
tested.

The deliverable is `spec/phase0-results.md`: the tables, the raw generations, the
escape-hatch list, and a one-paragraph recommendation — negative findings first.
