# guml-fmt

Formatter, canonicaliser and syntax classifier.

Sits below the parser and works on source text, so it can format a document that does not yet compile.

The canonical form strips comments, blank lines and declaration order, which is what makes two
generations of the same interface comparable in the benchmark.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 7 of 10 in dependency
order.

MIT.
