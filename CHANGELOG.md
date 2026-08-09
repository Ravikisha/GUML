# Changelog

Notable changes to GUML. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**GUML is `0.x`.** Under semver that means the language surface, the emitted output
and the crate APIs may change in a minor release. Diagnostic *codes* are the
exception: they are append-only and never renumbered, because the repair loop keys
on them.

## [Unreleased]

## [0.2.0] — unreleased

A minor rather than a patch: two diagnostics changed severity and the default theme changed, both of
which alter what an existing document does.

### Fixed — a document could crash the process

`card {😀` — an unterminated `{` before any multibyte character — panicked the lexer. The code
trimming the (absent) closing brace subtracted one *byte*, landing inside the character. With
`panic = "abort"` that was not an exception anyone could catch: it killed the CLI, and it would kill a
Flask worker or trap a wasm module in a browser tab.

- The slicing bug is fixed, and a byte-level fuzzer guards the class. The previous fuzzer recombined a
  fixed corpus of valid ASCII lines, so in a million documents it could not once produce a multibyte
  character.
- **`panic = "unwind"`** — a compiler that gets embedded should hand its host a catchable failure
  rather than a corpse.

**Every 0.1.x npm package contains this crash.** Upgrade.

### Changed — the compiler stopped refusing what it already knew

These were the two most common reasons model-generated GUML failed to compile, measured in
`bench/gen`. Both were cases where the answer was already in the document.

- **`GUML0080`** now accepts `option` children as a source of options, not only a state `domain`.
  `guml_codegen::select_options` had reconciled both spellings for a while, so codegen accepted a form
  validation rejected — two halves of one compiler disagreeing about one document.
- **`GUML0051`** is a **warning** where a field binds a state: `select colour` is named from `colour`,
  and every backend emits that name. It was an error while the usable name sat in the same line.
  Deriving is not inventing — a field with no binding is still an error.

On the same six generations, with no regeneration: **1 of 6 compiled → 3 of 6**.

- **The default theme is now stock Tailwind**, not shadcn. shadcn emits `bg-primary` and
  `text-foreground`, which resolve only where its CSS variables are defined — so `pnpm add tailwindcss`
  plus a compile produced an unstyled page with no error. `--theme shadcn` or `"theme": "shadcn"` keeps
  the old behaviour, and `@guml/shadcn` now ships the tokens with the components.

### Added

- **`guml mcp`** — the compiler as a Model Context Protocol server. A model asks for the tags it needs
  (175 characters rather than 3,808) and checks its own output against the compiler that will build it.
  Removes the ~3,000-token prompt tax that using GUML previously required.
- **Plugins.** `"plugins": ["@guml/shadcn"]` in `guml.json` loads a package's vocabulary *and* its
  theme from one entry, resolved through `node_modules`. Naming them separately was two chances to
  install one and forget the other.
- **`html-fragment` and `html-bare` backends.** A fragment carries no doctype, no `<head>` and **no
  `<main>`** — a document may hold exactly one landmark, so a fragment carrying its own would create a
  second the moment it were embedded.
- **CDN builds.** `<script src="https://cdn.jsdelivr.net/npm/@guml/core">` defines `window.guml`;
  the ESM entry works from any CDN.
- **`guml.stylesheet()`** in the Python package, and `check --format json` now emits `[]` rather than
  nothing for a clean document.

### Security

- `DEMO_COOKIE_SECRET` fails closed rather than falling back to a constant published in a public
  repository.
- pyo3 0.27 → 0.29, closing RUSTSEC-2026-0177 (a missing `Sync` bound allowing data races under
  free-threaded Python). Found by `cargo deny`, now a CI gate.
- Seven transitive npm advisories pinned to patched versions.

## [0.1.0] — unreleased

First public release.

### Language

- 49 tags, with modifiers, directives and content children. Levels: `core`, `app`.
- Bindings, derived values, enumerated state domains, resources and repeaters.
- Actions, deliberately not Turing-complete. `js` and `raw` are the escape hatch and
  the security boundary at once, and both are reported by `guml capabilities`.

### Compiler

- Seven backends from one document: `react`, `svelte`, `html`, `wc`, `json`, `a2ui`,
  `mcp-ui`. One element table, one class table and one expression lowering shared
  across all of them, each pinned by a cross-backend agreement test.
- 50 diagnostic codes, all collected in a single pass — the repair loop gets every
  error per round, not the first one.
- Unsupported constructs produce a warning and a `TODO` in the output. Never
  silently-wrong code.
- `guml capabilities` reports what a document will do and emits a matching
  Content-Security-Policy.
- Source maps from emitted output back to GUML spans.

### Design system

- **shadcn/ui is the default theme** (`themes/shadcn.json`), covering all 51 styled
  tags and every modifier, using shadcn's own token names in `oklch`. The variables
  are the interface: a host running shadcn already deletes the `:root` block and its
  palette applies throughout.
- `slate` is also shipped, selectable with `--theme`.
- A theme below WCAG AA contrast, or with no focus treatment, is refused by
  `Theme::validate`.

### Tooling

- `guml-cli`: `build`, `check`, `fmt`, `validate`, `capabilities`, `registry`,
  `tokens`, `ast`, `lex`. Installs as `guml`.
- Language server (`guml-lsp`), formatter (`guml-fmt`), tree-sitter grammar and a
  VS Code extension.

### npm packages

Published under the `@guml` scope. The unscoped name `guml` is unavailable — npm's
similarity check rejects it against `gulp`, `gm`, `xml`, `toml`, `yaml` and others.

| package | for | size |
|---|---|---|
| `@guml/core` | Compiling, rendering, diagnostics, repair | 787 KB wasm |
| `@guml/fmt` | Formatting, canonical form, classification | 178 KB wasm |
| `@guml/highlight` | Highlighting, synchronously and in Node | ~15 KB, no wasm |
| `@guml/widgets` | `chart`, `calendar`, `date`, `upload`, `command` | 22 KB |
| `@guml/shadcn` | 26 tags over all 61 shadcn/ui components | 257 KB |

The split follows the compiler's own shape rather than being a packaging convenience.
`guml-fmt` sits *below the parser* — lexer, registry and diagnostics, no codegen — so
building only that is 178 KB rather than 787. `@guml/highlight` has no wasm at all and is
held to the compiler's classifier by a parity test over every fixture.

`@guml/fmt` and `@guml/highlight` **run in Node**; `@guml/core` does not, because its wasm
is built for the web target and loads itself with `fetch`. Use the CLI to compile from a
shell.

### Python

`pip install guml` — the compiler as a native extension, plus a `guml` command, with no Rust
toolchain. One wheel per platform via `abi3-py39`, covering Python 3.9 and later.

Two audiences, both first-class: driving a model (`SPEC`, `registry()` slices, `check`, `repair`) and
serving pages (`render()` to HTML with no JavaScript, from Flask, FastAPI or Django). A Jinja2
extension is shipped; everything else is a three-line snippet in the README rather than a framework
adapter to maintain.

**`render()` defaults to `level="core"`** — markup only, no `state`, `data`, actions or `js`. This
deliberately differs from `guml build`, which defaults to `app`: `js` and `raw` compile through
unchanged, and a server usually renders documents it did not write.

Held to the CLI byte-for-byte. `test_agreement.py` compiles every fixture through both and compares
the output across five backends, plus formatting, canonical form and diagnostic codes — three
bindings over one compiler is three chances to disagree, and this repository has been bitten by
exactly that three times.

### HTML backend

- `html-bare` — classes, no stylesheet, for a host that already runs Tailwind. `Style::None` existed
  in the backend with no way to reach it.
- `html-fragment` — content only: no doctype, no `<head>`, and **no `<main>`**. For a Jinja include or
  an htmx swap target. The missing landmark is the point: a document may hold exactly one `main`, so a
  fragment carrying its own would create a second the instant it were embedded.

### Registry packages

- `@guml/widgets` — `chart`, `calendar`, `date`, `upload`, `command`. The worked
  example that `spec/REGISTRY.md` describes.
- `@guml/shadcn` — 26 tags over all 61 real shadcn/ui components, for the components
  GUML has no builtin for. Includes the adapter layer that reconciles GUML's uniform
  field contract with each component's own API.

### Known limitations

See the Status page in the documentation. In short: v1 is client-only — server,
database and authentication code generation are not implemented, and `route` and
`auth` do not lower.
