# guml-fmt-wasm

WebAssembly bindings for the GUML formatter and syntax classifier.

A build input for the [`@guml/fmt`](https://www.npmjs.com/package/@guml/fmt) npm package, not a library
to depend on. Not published to crates.io.

Separate from `guml-wasm` because `guml-fmt` depends only on the lexer, the registry and diagnostics —
no parser, no codegen, no backends. That is 178 KB instead of 787 KB.

---

Part of [GUML](https://github.com/guml-lang/guml).

MIT.
