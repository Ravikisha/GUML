# guml-registry

The component vocabulary.

A closed, validated set of tags — each with its kind, level, attributes, positionals, allowed children
and accessibility contract. This is the vocabulary a model is given in its prompt, which is why it is
data rather than code: a prompt-sized slice of it is generated from these entries, so the tags the
model is told about and the tags the compiler accepts cannot drift apart.

Registry *packages* extend it. A JSON file declaring `element` and `import` maps new tags onto a
host's own React components, which is how `@guml/shadcn` turns shadcn/ui into GUML vocabulary.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 5 of 10 in dependency
order.

MIT.
