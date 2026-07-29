# Generation test: what six applications actually produced

Run twice: `meta/llama-3.1-8b-instruct` (the product default) and
`meta/llama-3.3-70b-instruct`, both hosted on build.nvidia.com. Shipping prompt (spec + full
registry + 3 examples, 11,064 chars ≈ 3,073 est. tokens), `temperature 0.2`, n=1 per app.
Reproduce with `node bench/gen/run.mjs`.

The table below is the 8B run; the 70B comparison is further down.

**n=1 per app.** These are observations that point at specific defects, not rates. Anything
phrased as a percentage below is arithmetic over six samples and should be read as such.

## Result — `meta/llama-3.1-8b-instruct`

| app | parses | requirements met | first error |
|---|---|---|---|
| todo | **yes** | **8/8** | — |
| expenses | no | 6/6 | `GUML0030` unknown tag `option` |
| bmi | no | 5/6 | `GUML0020` expected a tag name (trailing prose) |
| dashboard | no | 5/6 | `GUML0033` action refers to `order`, not declared |
| signup | no | 5/6 | `GUML0030` unknown tag `option` |
| tip | no | 3/5 | `GUML0030` unknown tag `---` |

1 of 6 parse · 32 of 37 requirements met.

The gap between those two numbers is the finding. The model largely **understood the
applications** — it produced the right resources, mutations, aggregates, enumerated filters
and bound `disabled` attributes — and then failed on a handful of surface rules. `expenses`
met every functional requirement and still does not compile.

## The one that worked, end to end

`out/todo.guml` — 24 lines, ~182 est. tokens — parses, validates clean, compiles, and the
emitted TSX **typechecks under `tsc --strict`**. It independently reproduced the shape of
`fixtures/b.guml`, including `optimistic:prepend` on the create and a `where={filter}` on the
repeater, from a prompt that never mentioned either.

## A bigger model does not fix it

The same six prompts through `meta/llama-3.3-70b-instruct` — 9× the parameters, and 107–262
seconds per generation instead of ~1.

| | 8B | 70B |
|---|---|---|
| generated | 6 | 5 (`todo` returned 504) |
| parses | 1 | 1 (`dashboard`) |
| requirements met | 32/37 | 27/29 |
| `option` under `select` | 2 apps | **2 apps** |
| trailing prose | 2 apps | 1 app |

The 70B understands the applications slightly better — 27/29 requirements, and it was the only
model to get the dashboard to compile — and it makes **the same `option` mistake at the same
rate**. That is the finding worth carrying forward: the gap survives a 9× increase in
capability, so it is not something a stronger model or a firmer prompt will fix. It is the
vocabulary.

> **Comparison hygiene.** The 70B run wrote into the same output directory as the 8B run
> before `--out` existed, so its first scoring table showed an 8B `todo` as a 70B success.
> The table above excludes it. Recording this because it is exactly the class of error that
> turns a comparison into decoration, and the fix (`--out`) is now in the runner.

## Failure classes, in order of what they cost

### 1. `option` children under `select` — the HTML prior (2 of 6)

```
select country aria="Country" required
  option GB "United Kingdom"     ← GUML0030: unknown tag
```

GUML puts a dropdown's domain on the *state* (`state country=GB|US|DE`); the model reaches
for `<option>` because every dropdown it has ever seen has them. Everything else on those
lines was right, including the `aria`.

This is a **language-surface finding, not a model failure.** The vocabulary is missing the
shape models reach for, and the fix belongs to whoever owns the spec — either `select` accepts
content lines as an alternative domain declaration, or it does not and the cost is accepted.
It is exactly the signal Phase 0 is built to measure, arriving early.

**Tried and rejected:** adding two explicit rules to the prompt (`select` takes no children;
stop after the last line). It did not fix it — the 8B still emitted `option`, and one app
regressed into prose. Cost ~65 prompt tokens for no measured gain, so it was reverted.

**And the 70B makes the same mistake**, with no prompt change, at the same rate. Two models an
order of magnitude apart in size converge on `<option>`, which is what a wrong abstraction
looks like from the outside: everyone independently writes the thing the language does not
accept.

### 2. Markdown and commentary leaking into the document (2 of 6)

`---` separators and a trailing "This page…" paragraph, despite the prompt's first rule being
"no prose before or after, no markdown code fence". Instruction-following, not comprehension.
Two cheap mitigations exist and neither is implemented yet: strip a trailing prose block in the
harness (the chatbot already strips fences), or constrain decoding.

### 3. Invented references (2 of 6)

`>order.ship` where the row variable is not declared; `>tip.update` on a resource with no such
mutation; a `data` directive with no path. All three are caught by the validator with a
suggestion attached, and all three are the kind of thing one repair round should fix — which
is the next thing worth measuring.

## The repair round, measured

`node repair.mjs --trials 3`. Four layers in increasing cost; each only sees what the cheaper
ones could not fix.

| app | raw errors | after the free layers | repair trials that fixed it | final | survived |
|---|---|---|---|---|---|
| todo | 0 | 0 | — | 0 | — |
| dashboard | 0 | 0 | — | 0 | — |
| tip | 1 | **0** | — | 0 | — |
| bmi | 8 | 2 | **1/2** | 0 | — |
| expenses | 5 | 5 | 0/3 | 5 | `GUML0030`, `GUML0051`, `GUML0080` |
| signup | 4 | 4 | 0/3 | 4 | `GUML0030`, `GUML0080` |

- **The free layers cost nothing and did most of the work.** `tip` went from broken to
  compiling with no model call at all, and `bmi` from 8 errors to 2. Five trailing lines of
  commentary were dropped across the set, found with the compiler rather than a prose regex.
- **One repair round fixed the reference and type errors**, as predicted — `bmi`'s remaining
  `GUML0065` fell on the second trial.
- **`option` survived everything.** Zero of six repair trials across `expenses` and `signup`
  removed `GUML0030`/`GUML0080`. Both predictions in this document held, so the vocabulary
  conclusion stands rather than being withdrawn.
- **Seven of nine repair attempts did not improve on the free layers, and two made things
  worse** (`expenses` 5 → 8, `signup` 4 → 6). The pipeline discards any attempt that raises
  the error count; without that guard a repair loop degrades documents it was asked to fix.
  That is worth knowing before Phase 5 commits to "≤1 repair round".

**Variance is real.** Two consecutive single-trial runs disagreed about `bmi` — fixed in one,
not in the next, at `temperature 0.1`. The table above reports trials-that-succeeded for that
reason; a single lucky round reported as a capability would have been wrong.

## What this says about the validator

Every failure above was caught statically, with a line number and, where possible, a fix.
Three of the codes involved are new in this change: `GUML0061` (unknown mutation), `GUML0080`
(`select` needs an enumerated state), `GUML0084` (request has no path). Without them, `tip`
and `signup` would have compiled to code that fetched the wrong URL or rendered an empty
dropdown — valid GUML, wrong program.

Two compiler gaps were found by pointing the validator at real generations rather than at
fixtures:

- `data rows:T[] FETCH /api/rows` **silently became a GET.** An unknown method was skipped by
  the parser. That is a silent mis-lowering, which invariant 3 forbids. Now `GUML0083`.
- A path that is not a route token (`GET api/rows`) left the URL **empty**, and the emitted
  code would have fetched the current page. Now `GUML0084`.

## What to do next, in order

1. **Decide the `select` question** at the language level. It is 2 of 6 failures on both
   models tested, unmoved by prompt or by scale, and the only one that is arguably the
   language's fault. Two options: `select` accepts content lines as an alternative way to
   declare its domain, or it does not and the escape-hatch rate carries the cost. This needs
   the spec owner, not a patch.
2. **Measure one repair round.** Feed the diagnostics back and re-score. Class 3 should
   disappear, and class 1 will not — which is precisely the experiment that separates "the
   model needs a hint" from "the vocabulary is wrong".
3. **Strip trailing prose in the harness.** Cheap, and it converts class 2 into nothing.
4. **Raise n.** Six samples at n=1 identify defects; they do not measure rates. `--repeat`
   exists for this.
