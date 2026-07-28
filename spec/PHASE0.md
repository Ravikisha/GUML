# Phase 0 — the kill-or-continue spike

**Two weeks. No compiler in the loop. One question:**

> Can a model produce valid, semantically correct GUML from a spec in context, and does the
> token saving survive contact with real generations?

Everything downstream assumes yes. The research report's primary risk (§12.1) is that the
out-of-distribution penalty documented for low-resource DSLs (arXiv:2410.03981) swamps the
token win — GUML has zero training data by construction. Anka (arXiv:2512.23214) is the
counter-example (+40pp on multi-step tasks, unseen DSL), but it is n=1, one narrow domain, and
reports no token accounting.

Two weeks answers it. Nine months of building does not make the answer better.

## The harness

`bench/phase0/` implements everything below. `just phase0-verify` runs the parts that
need no API key: harness preflight (task set, examples, registry slices, prompt budget,
references typecheck), a scoring self-test over synthetic generations, and prompt
assembly. `just phase0-run` needs `ANTHROPIC_API_KEY` and is resumable.

## Setup

- 10 tasks: 2 structure-heavy (CRUD, dashboard), 2 content-heavy (landing, docs), 6 mixed
  (settings screen, checkout step, data-viz filter panel, team management, pricing page, form
  wizard). Two of them reuse `fixtures/` (`b` → CRUD, `c` → landing) so the results connect
  to the report's numbers. `fixtures/a` is *not* a task: it serves as an in-context example,
  and a document cannot be both the example and the answer.
- Per task: a natural-language prompt, a functional-requirements checklist, and a hand-written
  React+TS+Tailwind reference implementation.
- Prompt = `spec/GUML-SPEC.md` + a registry slice (`guml registry --tags …`) + N examples.

## Conditions

| Variable | Levels |
|---|---|
| Model | Haiku 4.5, Sonnet 5, Opus 5 |
| In-context examples | 0, 3 |
| Target representation | GUML, React (baseline) |

30 GUML runs + 30 React runs. Both directions from the same prompt, same model, same session
settings.

## Measurements

1. **Output tokens** — per generation, using the model's own tokenizer or `count_tokens`.
   **Do not use `tiktoken`**: it is an OpenAI tokenizer and undercounts Claude by ~15–20% on
   text, more on code. `guml tokens` is a rough dev-loop estimate only, and says so.
2. **Input tokens** — spec + registry + examples, reported *separately*, and again as
   cache-read cost. This is the "prompt tax" (report §7.3); omitting it invalidates the study.
3. **Parse validity** — check by hand against `spec/grammar.ebnf`, or with `guml check`
   once the front end covers the constructs used. Record *what* was invalid, not just whether.
4. **Semantic correctness** — score against the task checklist. Score the React baseline the
   same way, by the same person, blind to which arm produced it.
5. **Escape-hatch rate** — how many tasks needed a construct the spec cannot express. This is
   the number most likely to sink the whole idea, so it gets recorded first, not last.
6. **Latency** — wall clock per generation.

## The gate

Continue to Phase 1+ only if **all three** hold:

- [ ] ≥80% of Sonnet 5 generations at 3 examples are parseable GUML
- [ ] Median output-token reduction ≥3× vs the paired React on structure-heavy tasks
- [ ] Semantic correctness is **not worse** than the React baseline on the same tasks

## Interpreting the result honestly

- **All three pass** → proceed, and the numbers become Paper 1's preliminary section.
- **Tokens win, correctness loses** → that is the low-resource penalty. Try grammar prompting
  (arXiv:2305.19234) before concluding; if it still loses, the honest framing is "GUML is a
  cost optimisation, not a reliability one," which is a much weaker paper.
- **Wins on Haiku, vanishes on Opus** → expected per *Capacity, Not Format* (arXiv:2606.09410).
  Still publishable, and commercially it points at cheap-model + compiler as the product.
- **Escape-hatch rate >30%** → the vocabulary is too small for real work. Fix the vocabulary
  before anything else; a benchmark of only expressible tasks is a rigged benchmark.
- **Nothing works** → publish the negative result. It answers an open question in the
  literature and costs two weeks.

## Deliverable

`spec/phase0-results.md`: the table, the raw generations, the escape-hatch list, and a
one-paragraph recommendation. Negative findings stated first, not buried.
