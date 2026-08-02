# @guml/fmt

The [GUML](https://github.com/guml-lang/guml) formatter and syntax classifier, as WebAssembly.
**178 KB**, and unlike `@guml/core` it **runs in Node**.

```sh
pnpm add @guml/fmt
```

```ts
import { format, canonical, isFormatted } from "@guml/fmt";

await format(source);       // idempotent; preserves comments and blank lines
await canonical(source);    // strips them — a normaliser, not a formatter
await isFormatted(source);  // the `--check` predicate
```

This is the compiler's own `guml-fmt` crate — the same code `guml fmt` runs — not a re-implementation.
Formatting here and formatting on the command line produce identical bytes.

## Why it is separate from `@guml/core`

`guml-fmt` sits **below the parser**. It needs the lexer, the registry and the diagnostic codes, and
nothing else: no parser, no semantic analysis, no code generation, no backends. Building only that is
not a marginal saving.

| build | size |
|---|---|
| `@guml/core` — the whole compiler | 787 KB |
| `@guml/fmt` — formatter and classifier | **178 KB** |

A pre-commit hook that formats GUML has no reason to download the code generator for seven backends.

`@guml/core` still exposes `format` and `highlight`, from these same Rust functions — this is a smaller
door to the same room, not a fork. Use core when you also need to **compile**.

## It works in Node

`@guml/core` does not, and for a formatter that matters more: pre-commit hooks, CI checks and editor
tooling are all Node.

The reason core cannot is that `wasm-pack --target web` generates a loader that fetches the `.wasm`
beside itself, and `fetch` has no `file://` support in Node. The failure surfaces as an undici stack
trace that never mentions WebAssembly, so it reads as a broken install rather than a wrong environment.

This package reads the bytes itself when it detects Node, so `init()` needs no argument anywhere:

```ts
// works unchanged in Node, in the browser, and through any bundler
import { format } from "@guml/fmt";
await format(source);
```

Supply your own module if you want control over when it loads:

```ts
import { init } from "@guml/fmt";
await init(await fetch("/guml_fmt_bg.wasm"));
```

## `format` vs `canonical`

They are separate functions rather than one function with an option, because they answer different
questions and one of them **deletes your comments on purpose**.

- **`format`** — what you want in an editor or a hook. Normalises indentation and spacing, preserves
  comments and blank lines. Idempotent, which is what makes `isFormatted` a sound `--check`.
- **`canonical`** — strips comments and blank lines, hoists and sorts directives, prefers the shortest
  spelling of every value. Two documents that *mean* the same thing become byte-identical. That is what
  makes two independent generations of one interface comparable, and it is why it is not a formatter.

```ts
const a = `page "X"\ncol\n  p Hello\n`;
const b = `// notes\npage   "X"\n\n\ncol\n     p Hello\n`;
(await canonical(a)) === (await canonical(b)); // true
```

## Highlighting

`highlight()` returns spans carrying the compiler's own class names:

```ts
const spans = await highlight(`page "Counter"`);
// [{ line: 1, start: 0, end: 4, class: "directive" }, …]
```

For a page that renders on a **server**, [`@guml/highlight`](https://www.npmjs.com/package/@guml/highlight)
does the same job in ~15 KB of TypeScript with no wasm and no `await`, held to this implementation by a
parity test over every fixture. Prefer it unless you need exactness by construction rather than by test.

## Related

- [`@guml/core`](https://www.npmjs.com/package/@guml/core) — the full compiler, plus a React runtime
- [`@guml/highlight`](https://www.npmjs.com/package/@guml/highlight) — highlighting without wasm
- [`@guml/widgets`](https://www.npmjs.com/package/@guml/widgets), [`@guml/shadcn`](https://www.npmjs.com/package/@guml/shadcn) — component vocabularies

---

MIT.
