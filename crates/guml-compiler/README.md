# guml-compiler

The driver. **Start here.**

Source in, diagnostics and emitted files out. This is the crate to depend on if you want to compile
GUML from Rust; the others are its internals, published so that it can be.

```rust
let (program, diagnostics) = guml_compiler::check(source);
```

Semantic analysis, desugaring of the loading/empty/error conventions, and the whole-program checks
live here.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 8 of 10 in dependency
order.

MIT.
