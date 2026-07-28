# Phase 0 scoring rubric

Two things are scored. One is mechanical and `score.mjs` does it. The other is human
and is the part most easily corrupted, so the procedure matters more than the scale.

## Mechanical (automated)

| Measure | How | Recorded as |
|---|---|---|
| Parse validity | `guml check --format json` on the extracted output | pass/fail plus the error codes |
| Markdown fence | the output was wrapped in ```` ``` ```` | counted separately from validity |
| Escape-hatch rate | a `// UNSUPPORTED:` line, or a `js`/`raw` block | list of what could not be expressed |
| Output tokens | `usage.output_tokens` from the API response | per run, medians per category |
| Prompt tax | `usage.input_tokens` and `cache_read_input_tokens` | reported separately, never netted off |
| Latency | wall clock around the streamed call | median per arm |
| Truncation | `stop_reason == "max_tokens"` | flagged; excluded from token medians |

Notes that keep these honest:

- **Never `tiktoken`.** It is an OpenAI tokenizer and undercounts Claude by roughly
  15–20% on prose and more on code. `guml tokens` is a dev-loop estimate and says so.
- **A fence is not a parse failure**, but it is a rule violation. Stripping it and
  saying nothing would hide a real usability finding: the model would not stop
  formatting for humans.
- **The `// UNSUPPORTED:` marker is a GUML comment**, so it compiles away and an honest
  admission is never also counted as a parse error. The marker was originally `#`, which
  parsed as an unknown tag and conflated the two measurements.
- **An unparseable generation is not put up for human scoring.** It already failed in
  the parse column; scoring it as zero correctness would count one failure twice.

## Semantic correctness (human)

### The scale

Per checklist item, one of:

| Score | Meaning |
|---|---|
| **1** | Met. A user could rely on it. |
| **0.5** | Attempted but incomplete or subtly wrong — the optimistic update applies but does not roll back; the label exists but is a placeholder. |
| **0** | Absent, or present and broken. |

A generation's score is the mean across its checklist. Report the median across
generations, per arm and per category.

### The procedure

1. Run `node score.mjs`. It writes `results/scoresheet.csv` — every gradable
   generation, rows shuffled, arm and model stripped to a `blindId`.
2. **Do not open `results/keymap.json`.** It is the de-blinding key. Reading it before
   scoring invalidates the correctness gate, which is the whole reason for the gate.
3. Score the GUML arm from its **compiled output**, not its source: `guml build`
   the generation and grade the emitted React the same way you grade the React arm.
   Grading GUML source directly rewards the compiler's conventions for free — the
   `data` directive *promises* rollback, and the question is whether the model asked
   for the right thing, not whether the compiler kept its promise.
4. Score every checklist item for one generation before moving to the next, and take
   a break between tasks rather than between arms.
5. Re-run `node score.mjs`. It reads the filled sheet and will not overwrite it.

### Where a scorer will be tempted to cheat, without meaning to

- **Recognising the style.** Compiled GUML has a house style. If you can tell the arm,
  you cannot un-know it — so score the *checklist*, item by item, and refuse to form
  an overall impression first.
- **Rewarding verbosity.** A 200-line React component that does eight of twelve things
  scores 8/12, exactly like a 30-line one that does eight of twelve.
- **Rewarding intent.** A comment saying `// TODO: roll back on failure` is a 0.
- **Grading the prompt.** If a checklist item was ambiguous in the task prompt, fix
  the prompt and re-run that task. Do not average over your own confusion.

### Second scorer

If a second person is available, have them score a 20% overlap sample and report
Cohen's κ. A correctness claim from a single unblinded scorer is the weakest link in
this study, and a reviewer will say so first.

## The gate

From `spec/PHASE0.md`, all three must hold:

- [ ] ≥80% of Sonnet 5 generations at 3 examples are parseable GUML
- [ ] Median output-token reduction ≥3× versus paired React on structure-heavy tasks
- [ ] Semantic correctness not worse than the React baseline

`score.mjs` prints the state of each. Two of the three can be computed without a
human; the third cannot, and until it is filled in the answer to Phase 0 is "unknown",
not "yes".
