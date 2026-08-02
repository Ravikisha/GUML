# guml-diagnostics

Diagnostic codes, spans and rendering.

Every error carries a `GUML0000`-style code, a span, and a suggested fix where one exists.

**Codes are append-only.** The LLM repair loop keys on them to decide what to change, so renumbering
one is a breaking change for every consumer at once. Add a new code; never reuse or renumber an old
one.

The parser also collects *every* error in one pass rather than stopping at the first. Each round of
the repair loop is a full model generation, so reporting one error at a time would multiply the cost
of fixing a document by the number of mistakes in it.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 2 of 10 in dependency
order.

MIT.
