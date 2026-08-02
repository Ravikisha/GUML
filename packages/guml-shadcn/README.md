# @guml/shadcn

Every [shadcn/ui](https://ui.shadcn.com/docs/components) component, plus the GUML registry that turns them
into a vocabulary a model can write.

```sh
pnpm add @guml/shadcn
guml build app.guml --registry node_modules/@guml/shadcn/guml.registry.json
```

```guml
textarea note "What happened?" rows=4
radio size
slider volume min=0 max=100
datepicker when
```

```tsx
import { RadioGroup, Slider, Textarea, DatePicker } from "@guml/shadcn";

<Textarea placeholder="What happened?" rows={4} aria-label="note" value={note} onChange={setNote} />
<RadioGroup aria-label="Size" value={size} onChange={setSize} options={["small", "medium", "large"]} />
```

## What is in here

**The real components.** `src/components/ui/` is 61 files written by `pnpm dlx shadcn@latest add --all` —
Radix, Base UI, cmdk, embla, recharts, react-day-picker, sonner, vaul. Not a reimplementation, not a fork.
`pnpm dlx shadcn@latest add <name>` still updates one in place, which is shadcn's own model, and nothing in
this package edits them.

**The theme is already the default.** GUML's builtin theme *is* shadcn — `crates/guml-codegen/themes/shadcn.json`,
with shadcn's own token names (`--background`/`--foreground` pairs, `--primary`, `--muted`, `--border`,
`--ring`, `--radius`) in `oklch`. So a document that never installs this package already looks like shadcn.
What the package adds is the components a class table cannot express.

**26 tags.** Only the components GUML has no builtin for. `card`, `btn`, `badge`, `input`, `select`, `table`,
`tabs`, `modal`, `drawer`, `sidebar`, `skeleton`, `progress`, `toast`, `avatar`, `breadcrumb` and
`pagination` are already in the compiler's vocabulary and already wear these classes; a package may not
shadow a builtin (`GUML0092`), and a second spelling of each would split every document's vocabulary in two
for nothing.

| | |
|---|---|
| overlays | `confirm` `popover` `hovercard` `tooltip` `dropdown` `contextmenu` `menubar` `navmenu` |
| layout | `collapsible` `carousel` `resizable` `scrollarea` `ratio` `btngroup` |
| fields | `textarea` `radio` `slider` `otp` `combobox` `datepicker` `togglegroup` |
| content | `attachment` `kbd` `spinner` `bubble` `label` |

`chart`, `calendar`, `date`, `upload` and `command` are in `@guml/widgets`, the worked example in
`spec/REGISTRY.md`. Two packages declaring one name cannot both load, so they stay there.

Roughly **600 estimated prompt tokens** for the whole slice, by `guml tokens` — a ~3.6 chars/token estimate,
not a tokenizer count.

## Consuming this package

**It ships TypeScript source, not a build.** `exports` points at `.tsx` directly — no `dist`, no
`main`, no build step — so your bundler has to compile it.

- **Next.js** — add `"@guml/shadcn"` to `transpilePackages` in `next.config.ts`.
- **Vite** — works as-is.
- **Plain `tsc`** — set `jsx` and do not exclude this package from `node_modules`, or you will get
  "unexpected token" on the first `<`.

This is deliberate and it matches shadcn's own model: the premise of shadcn/ui is that **the component
source is yours to edit**, and a prebuilt `dist` would defeat that entirely. `pnpm dlx shadcn@latest
add <name>` still updates a file in place here exactly as it would in your own app.

## Setup

Tailwind v4, CSS-first — no `tailwind.config.js`.

```css
@import "tailwindcss";
@import "@guml/shadcn/styles.css";
```

`styles.css` carries the `:root` and `.dark` token blocks and the `@theme inline` mappings. **The variables
are the interface**: a host already running shadcn deletes the `:root` block and its own palette applies
throughout, which is the entire reason to theme in tokens rather than in colour literals.

## Adding your own

`components.json` is a normal shadcn config, so the usual command works and new components land in
`src/components/ui/` beside the rest:

```sh
pnpm dlx shadcn@latest add <name>
```

To make one a GUML tag, add an entry to `guml.registry.json` and re-run `guml registry --validate .`.

## `src/guml/` — the adapters, and why they exist

The compiler emits **one shape** for every `field`-kind tag, whoever wrote it:

```tsx
<Slider value={volume} onChange={setVolume} min={0} max={100} aria-label="Volume" />
```

`onChange` takes the *value*, not an event. That uniformity is what lets one lowering serve every field in
the vocabulary. shadcn's components each carry their upstream primitive's API instead — Radix's Slider is
`number[]` and `onValueChange`, a raw `<textarea>` is a React `ChangeEvent`, Base UI's Combobox is a compound
of six elements. All three are right for their own library and none is the shape above.

The reconciliation lives here, in the package, in the language the components are written in — not as a table
of prop spellings inside the compiler (a copy of shadcn's API that goes stale the day shadcn changes, and
would need a branch per package anyone ever writes) and not as a mapping language in JSON (which cannot
express `number[]`, let alone a compound). That is why registry `element`/`import` point at a *component*
rather than a DOM tag: the host owns the glue.

Eight adapters — `Textarea`, `RadioGroup`, `Slider`, `InputOTP`, `Combobox`, `DatePicker`, `ToggleGroup`,
`Collapsible` — are re-exported explicitly from `src/index.ts` *after* the wildcards, so they shadow the raw
components of the same name. An explicit re-export beats an `export *` for the same binding; that is the
module spec, not a bundler quirk. The untouched Radix originals stay reachable at `@guml/shadcn/ui/slider`.

`DatePicker` is the one that is a real component rather than a prop rename: shadcn ships no `date-picker.tsx`
because its date picker is a *recipe* composing Popover, Button and Calendar. The registry declared it before
it existed, and the typecheck gate is what caught that.

## Checks

```sh
just shadcn-test
```

- `guml registry --validate .` — the package audits
- `pnpm typecheck:example` — compiles `example.guml` and typechecks the **emitted** TSX against the real
  components
- `cargo test -p guml-compiler --test package_shadcn` — the same from Rust, needing no Node

The second is the one that matters, because the components are someone else's and the emitted props are being
checked against an API this repo does not control. Its first run found four things:

1. `DatePicker` declared but absent upstream
2. `onChange` emitted as a value callback where a raw `<textarea>` wants a React event
3. `value={n}` where Radix wants `number[]`
4. a `radio` emitted with **no options at all** — bound correctly to a state and offering the reader no way
   to change it

The fourth was a compiler bug, and neither the audit nor the typecheck could see it: an empty `<RadioGroup>`
is valid TypeScript. A `field`-kind host component now receives the alternatives it offers as `options`,
reconciled from `option` children or the bound state's domain by the same function `select` uses, so the two
spellings cannot disagree about one element.
