---
name: guml-phase0
description: Use when running or analysing the Phase 0 kill-or-continue spike — the 2-week experiment that decides whether the GUML program proceeds at all. Loads the protocol, the gate criteria, and how to interpret each possible outcome including the negative ones.
---

# Phase 0 — the gate

Full protocol: `spec/PHASE0.md`.

**The one question:** can a model produce valid, semantically correct GUML from a spec in context,
and does the token saving survive contact with real generations?

Everything downstream assumes yes. Two weeks buys the answer; nine months of building does not make
it better. Do not skip this and do not soften the gate afterwards.

## Setup in one paragraph

10 tasks (2 structure-heavy, 2 content-heavy, 6 mixed), each with a prompt, a
functional-requirements checklist, and a hand-written React reference. Prompt =
`spec/GUML-SPEC.md` + a registry slice (`guml registry --tags …`) + N in-context examples. Run
10 × {Haiku 4.5, Sonnet 5, Opus 5} × {0, 3 examples}, both for GUML and for the React baseline.
**No compiler in the loop** — this measures the representation, not the toolchain.

## The gate — all three, or stop

- [ ] ≥80% of Sonnet 5 generations at 3 examples are parseable GUML
- [ ] Median output-token reduction ≥3× vs paired React on structure-heavy tasks
- [ ] Semantic correctness **not worse** than the React baseline on the same tasks

## Measure these, in this order

1. **Escape-hatch rate** first — how many tasks needed a construct the spec cannot express. This is
   the number most likely to sink the idea, so it does not get buried at the bottom.
2. Parse validity, with *what* was invalid recorded, not just whether.
3. Semantic correctness against the checklist, scored blind to arm, by the same person.
4. Output tokens (model's own tokenizer — never `tiktoken`).
5. Input tokens: spec / registry / examples, separately, and as cache-read cost.
6. Wall-clock latency.

## Interpreting outcomes — all of these are legitimate results

| Outcome | Reading | Action |
|---|---|---|
| All three pass | Thesis survives first contact | Proceed to Phase 1; this becomes Paper 1's preliminary section |
| Tokens win, correctness loses | The low-resource-DSL penalty (arXiv:2410.03981) is dominating | Try grammar prompting (arXiv:2305.19234) first. If it still loses, the honest framing is "cost optimisation, not reliability" — a weaker paper |
| Wins on Haiku, vanishes on Opus | Expected per *Capacity, Not Format* (arXiv:2606.09410) | Still publishable, and commercially points at cheap-model + compiler as the product |
| Escape-hatch rate >30% | Vocabulary too small for real work | Fix the vocabulary before anything else |
| Nothing works | The idea does not hold | **Publish the negative result.** It answers an open question in the literature and cost two weeks |

## Discipline

- Write the hypotheses and the analysis plan to a file **before** running anything, and do not edit
  it afterwards.
- Keep every raw generation. Aggregates without raws are not reproducible.
- State model versions, exact tokenizer, and example counts in the results table.
- Deliverable: `spec/phase0-results.md` with negative findings stated first, then the table, the
  raw generations, the escape-hatch list, and a one-paragraph recommendation.
