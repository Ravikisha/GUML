# @guml/widgets

Five components GUML has no builtin for, as a GUML vocabulary: `chart`, `calendar`, `date`, `upload`,
`command`.

This is the **worked example** that `spec/REGISTRY.md` describes — the smallest complete registry
package, kept small on purpose so it can be read end to end. If you are writing your own package, read
this one first.

```sh
pnpm add @guml/widgets
guml build app.guml --registry node_modules/@guml/widgets/guml.registry.json
```

```guml
chart revenue of=revenue kind=line
date starts
upload avatar
```

## Why these five are not builtins

They need a charting library, a date library and a file-upload surface — dependencies the compiler
cannot assume and should not carry. A builtin has to lower in **all seven backends**, including the
no-JavaScript static-HTML build; a chart cannot. Making them a package is what lets the vocabulary grow
without the compiler growing a dependency tree.

## Consuming this package

**It ships TypeScript source, not a build.** `exports` points at `.tsx` directly. There is no `dist`,
no `main`, and no build step.

That is a deliberate choice and it has a cost, so it is worth stating rather than leaving to be
discovered: your bundler has to compile it.

- **Next.js** — add it to `transpilePackages` in `next.config.ts`.
- **Vite** — works, since Vite transforms linked/ESM sources.
- **Plain `tsc`** — set `"allowJs"`/`"jsx"` appropriately and do not exclude `node_modules` for this
  package, or you will get "unexpected token" on the first `<`.

The reason is that a registry package's components are meant to be **read and edited**. A prebuilt
`dist` hides the one thing you most need to see when a tag lowers to something you did not expect. The
same reasoning applies to `@guml/shadcn`, whose upstream model is explicitly "the source is yours".

## What a registry package is

A JSON file plus the components it names. Each entry declares a tag's kind, its attributes and
positionals, its accessibility contract, and — the part that makes it a *package* — an `element` and an
`import`:

```json
{ "name": "chart", "kind": "container", "element": "Chart", "import": "@guml/widgets" }
```

The compiler then emits `<Chart … />` and generates the import. `element` pointing at a *component*
rather than a DOM tag is what lets the host own any glue between GUML's calling convention and the
component's own API.

A package may not shadow a builtin (`GUML0092`), and every tag it declares must lower somewhere.

## Checks

```sh
just widgets-test
```

- `guml registry --validate .` — the package audits
- `pnpm typecheck` — the components themselves
- `pnpm typecheck:example` — compiles `example.guml` and typechecks the **emitted** TSX against those
  components

The third is the one that matters, and it found three compiler bugs the first time it ran: `of=` and
`kind=` were silently dropped because the React backend's attribute loop encoded what those names mean
*for a builtin*, the title never reached `aria-label`, and a `field`-kind component got its state name
as children instead of a two-way binding.

---

MIT.
