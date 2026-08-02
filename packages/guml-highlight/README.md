# @guml/highlight

Syntax highlighting for [GUML](https://github.com/guml-lang/guml). **No WebAssembly**, ~15 KB, zero
runtime dependencies, and it runs synchronously — in a browser, in Node, and during server rendering.

```sh
pnpm add @guml/highlight
```

```ts
import { highlight, CLASS_STYLE } from "@guml/highlight";

const lines = highlight(`page "Counter"\nstate n: 0`, "guml");
// [
//   [ {text:"page", cls:"directive"}, {text:" ", cls:"plain"}, {text:"\"Counter\"", cls:"string"} ],
//   [ {text:"state", cls:"directive"}, …, {text:"0", cls:"number"} ],
// ]
```

**One array per line**, not a flat list — so you can render a line at a time without re-splitting, and
line numbers come for free.

Each token carries `cls`, the **compiler's own class name** (`tag`, `directive`, `modifier`, `string`,
`number`, `binding`, `punct`, `prose`, `plain`, …), not a CSS class. Map it to colour yourself, or use
the `CLASS_STYLE` table this package ships:

```tsx
{lines.map((toks, i) => (
  <div key={i}>
    {toks.map((t, j) => (
      <span key={j} className={CLASS_STYLE[t.cls]}>{t.text}</span>
    ))}
  </div>
))}
```

## Why this exists separately from `@guml/core`

The compiler has its own classifier — `guml_fmt::highlight`, reachable as `highlight()` from
`@guml/core` — and it is the authoritative one. Using it costs 787 KB of compiler WebAssembly, loaded
asynchronously, in a browser.

Highlighting a code block needs none of that. It has to run **synchronously during server rendering**,
and it has to work in **Node**, where the wasm build cannot load at all — it is compiled for the web
target and loads itself with `fetch()`.

So the trade is explicit:

| | `@guml/highlight` | `@guml/core` |
|---|---|---|
| size | ~15 KB | 787 KB wasm |
| runtime | synchronous | async, after wasm init |
| works in Node / SSR | yes | no |
| authoritative | by parity test | by construction |

## How it stays honest

A hand-written highlighter for a language whose vocabulary lives somewhere else is a drift machine.
Two things stop that, and both are mechanical:

**The vocabulary is generated.** `src/vocabulary.generated.ts` comes from `guml registry` — the
compiler's own tag, modifier and directive lists. A tag added in Rust reaches this package with no
second edit. It is never retyped.

**Tokenising is parity-tested.** `pnpm check:highlight` runs this package and the compiler's Rust
classifier over every fixture and fails on any disagreement. **936 spans across 10 documents currently
agree.**

Both halves earn their keep. The hand-maintained version of this file had already drifted before the
checks existed — it listed `h3`, which the registry does not define — and the generator itself was
emitting `content-children:` as a tag, because a section header in `guml registry` output matched the
component pattern.

## Other languages

`tsx`, `bash`, `json` and `text` are also supported, as ordinary regex grammars. Nothing in the
compiler describes them, so there is nothing for them to drift from and no parity test to run.

```ts
highlight(source, "tsx");
```

## Related

- [`@guml/core`](https://www.npmjs.com/package/@guml/core) — the full compiler as wasm, plus a React runtime
- [`@guml/fmt`](https://www.npmjs.com/package/@guml/fmt) — the formatter, 178 KB rather than 787 KB
- [`@guml/widgets`](https://www.npmjs.com/package/@guml/widgets), [`@guml/shadcn`](https://www.npmjs.com/package/@guml/shadcn) — component vocabularies

---

MIT.
