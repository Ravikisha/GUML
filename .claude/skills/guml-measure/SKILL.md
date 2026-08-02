---
name: guml-measure
description: Use before asserting any token, cost, latency, or compression number about GUML — in a README, a paper, a commit message, or a conversation. Encodes the measurement protocol and the reporting requirements that keep the claims defensible.
---

# Measuring GUML honestly

The project's credibility is a function of these numbers. Bias toward understating the win.

## Protocol

1. **Right tokenizer.** Use the target model's own tokenizer or `count_tokens`. `tiktoken` is an
   **OpenAI** tokenizer and undercounts Claude by ~15–20% on text, more on code. `guml tokens` is a
   dev-loop estimate (~3.6 chars/token) and must be labelled as one every time it is quoted.
2. **Split input from output.** Output tokens cost ~5× input on current frontier models (Opus 5
   $5/$25 per MTok; Sonnet 5 $3/$15; Haiku 4.5 $1/$5). Report spec / registry / examples / prompt /
   generated separately, plus cached vs uncached.
3. **Report the prompt tax and the break-even size.** Spec + registry + examples is real cost.
   State the artifact size below which raw React is simply cheaper. Omitting this is the documented
   failure mode that sank TOON's headline claims (arXiv:2603.03306).
4. **Per-category, never one average.** Structure-heavy artifacts approach 8×; content-heavy
   asymptote at 2–3×, because prose is irreducible. Refuse to publish a blended average.
5. **Report the escape-hatch rate.** Tasks needing `raw`/`js`. A benchmark of only expressible
   tasks is rigged.
6. **Compare against the real baseline.** Editing claims go against *diff-based* React editing, not
   full regeneration — agents patch files, they do not rewrite them.
7. **Disclose authorship bias.** If one person wrote both sides, say so in the result line.
8. **Distinguish authored from generated.** Fixture measurements are what a human wrote. They say
   nothing about what a model can produce; that needs Phase 0 data.

## Established baseline (authored fixtures, `cl100k_base`, report §1.5)

| Fixture | React+TS+Tailwind | GUML | Reduction | Ratio |
|---|---:|---:|---:|---:|
| `a` counter card | 368 | 64 | 82.6% | 5.75× |
| `b` task CRUD | 1,441 | 178 | 87.6% | 8.10× |
| `c` landing page | 1,648 | 376 | 77.2% | 4.38× |
| total | 3,457 | 618 | 82.1% | 5.59× |

- GUML vs **minified JSON UI IR** (fixture `b`): 178 vs 324 → 45% fewer tokens.
- **Content floor**: 232 of `c`'s 376 GUML tokens are irreducible prose; structural overhead is
  144 vs React's ~1,416.
- **Amortisation**: a 3,000-token spec read from cache (~$0.0015/request at Opus 5 rates) against
  ~$0.0315/generation saved on fixture `b` → roughly 20:1, positive from the first request.
- **Latency**: output tokens decode sequentially, so an 8× output cut is roughly an 8× cut in
  generation time. This is the strongest practical argument — stronger than cost.

## Quick local check

```sh
cargo run -q -p guml-cli -- tokens fixtures/*.guml fixtures/*.tsx   # estimates only
cargo run -q -p guml-cli -- build fixtures/a.guml -o /tmp/out       # prints expansion ratio
```

## Reporting template

> Measured with `<tokenizer>` on `<n>` `<authored|model-generated>` artifacts across
> `<categories>`. Output tokens: X → Y (Z% reduction). Input overhead: spec S + registry R
> tokens, cached. Break-even artifact size: ~N tokens. Escape-hatch rate: E%. Caveat: `<bias>`.

If a number does not support the thesis, report it in the same breath as the ones that do.
