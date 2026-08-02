# guml-parser

The parser.

Error-recovering and registry-aware: it does not stop at the first mistake, and it knows which tags
exist while it parses, so an unknown tag is a diagnostic with a suggestion rather than a parse
failure that swallows the rest of the file.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 4 of 10 in dependency
order.

MIT.
