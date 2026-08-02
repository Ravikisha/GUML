# guml-codegen

The backends, and the design system.

Seven of them from one document: `react`, `svelte`, `html`, `wc`, `json`, `a2ui`, `mcp-ui`.

**One element table, one class table, one expression lowering, shared by all seven.** Not a
convention — a cross-backend agreement test pins it, because all three have drifted before. Three
copies of the element mapping once meant `nav`/`hero`/`footer` were `<div>` in the static-HTML backend
where React emitted landmarks, so the no-JavaScript build shipped a page with no landmarks at all.

Themes are data: `(tag, modifiers) -> class string`, with a contract that refuses a theme below WCAG
AA contrast or with no focus treatment. shadcn/ui is the default.

---

Part of [GUML](https://github.com/guml-lang/guml) — an intermediate representation and
compiler for LLM-generated user interfaces. Crate 6 of 10 in dependency
order.

MIT.
