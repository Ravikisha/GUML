# guml-ast

The abstract syntax tree.

Elements, states, derived values, resources, types, actions and escape blocks — the shape the parser
produces and every backend consumes.

Also the home of the whole-program queries the backends share, such as which fields a repeater's rows
actually use and what a `js` block declares at its top level.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 3 of 10 in dependency
order.

MIT.
