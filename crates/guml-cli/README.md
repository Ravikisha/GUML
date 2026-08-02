# guml-cli

The `guml` command.

```sh
cargo install guml-cli
guml build app.guml
guml check app.guml --format json
```

`build`, `check`, `fmt`, `validate`, `capabilities`, `registry`, `tokens`, `ast`, `lex`.

`capabilities` is the one worth knowing about: it reports what a document will actually do — network,
storage, script evaluation — and emits a matching Content-Security-Policy.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 10 of 10 in dependency
order.

MIT.
