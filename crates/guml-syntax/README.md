# guml-syntax

The lexer.

Indentation-sensitive and line-oriented, so a GUML document's block structure comes from its
indentation rather than from delimiters. This is the bottom of the stack: it depends on nothing else
in the workspace, and everything else depends on it transitively.

It is also the largest single share of compile time (~686 µs of a ~1.3 ms `check` on 200 lines), so
it is the first place to look at a performance question.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 1 of 10 in dependency
order.

MIT.
