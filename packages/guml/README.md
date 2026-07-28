# guml

Compile [GUML](https://github.com/guml-lang/guml) in the browser and render it in React.

This package ships the **actual Rust compiler** built to WebAssembly — not a re-implementation —
so diagnostics and generated classes are identical to what the `guml` CLI produces. 298 KB of
wasm, loaded lazily on first use.

```sh
pnpm add guml
```

## Render GUML

```tsx
"use client";
import { Guml } from "guml/react";

const source = `page Counter
state count=0

card sm center
  h Clicks
  metric {count}
  row center
    btn Decrement ghost disabled={!count} >count--
    btn Increment primary >count++
`;

export default function Page() {
  return <Guml source={source} />;
}
```

The preview is rendered from the compiler's own UI tree, so its markup and Tailwind classes match
the code `guml build` would have written. Requires Tailwind in the host app, or your own
`components` overrides (below).

## Compile without rendering

```ts
import { check, compile, tree, applyAllSuggestions } from "guml";

const { ok, diagnostics } = await check(source);

// Every unambiguous fix, applied with no model call — the mechanical half of a
// repair loop.
const repaired = applyAllSuggestions(source, diagnostics);

const { files } = await compile(source, "react"); // → [{ path: "Counter.tsx", contents }]
const { tree: ui } = await tree(source);          // → render tree for a custom renderer
```

| Export | Purpose |
|---|---|
| `check(source)` | Parse + analyse. Every diagnostic in one pass. |
| `compile(source, "react" \| "json")` | Framework source, or a render tree. |
| `tree(source)` | The render tree the React runtime consumes. |
| `registry(tags?)` | The component vocabulary, or a prompt-sized slice of it. |
| `applySuggestion` / `applyAllSuggestions` | Apply mechanical fixes from diagnostics. |
| `formatDiagnostic` | CLI-style rendering with a caret. |
| `evaluate` / `runAction` | The binding evaluator and action lowering, if you build your own renderer. |
| `init(url?)` | Warm the wasm module early. Optional. |
| `<Guml>` (`guml/react`) | Compile and render. |
| `useGumlTree` / `useGumlRuntime` | The pieces, for a custom renderer. |

## Bring your own components

Map any tag to your own component and keep GUML's semantics:

```tsx
<Guml
  source={source}
  components={{
    btn: (node, children) => <MyButton variant={node.class}>{children}</MyButton>,
  }}
/>
```

## Data

`data` resources fetch from their declared URL. Seed them instead — for previews, tests or
Storybook — with `data`:

```tsx
<Guml source={source} data={{ tasks: [{ id: "1", title: "Ship it", done: false }] }} />
```

Optimistic mutations apply immediately and roll back if the request fails, which is what
`optimistic:prepend` in the source means.

## Security

Bindings are evaluated by a small recursive-descent parser. **No `eval`, no `new Function`.** GUML
actions are deliberately not Turing-complete, and that boundary is what makes it defensible to
render a document produced by an untrusted agent. Unknown tags never reach the DOM — they are a
compile error.

## Scope, honestly

Runtime v0 covers containers, text, controls, fields, state, actions, bindings, and `list` with
optimistic mutations. `form`, `tabs`, `faq`, `tier`, `route`, `auth` and the `js`/`raw` escape
hatches parse but are not lowered yet: they render as a labelled gap rather than as approximate
markup. `useGumlTree` gives you the diagnostics if you want to handle that differently.

## Rebuilding the wasm

```sh
pnpm build:wasm   # needs a Rust toolchain + wasm-pack
```

Apache-2.0.
