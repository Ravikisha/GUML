# guml-lsp

The language server.

The compiler's own diagnostics, in an editor. No second implementation of anything: the same parser
and the same analysis, which is the only way the squiggles and `guml check` can be guaranteed to
agree.

`check` runs on a keystroke, which is why the compiler has a latency budget at all.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 9 of 10 in dependency
order.

MIT.
