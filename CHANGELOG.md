# Changelog

Notable changes to GUML. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**GUML is `0.x`.** Under semver that means the language surface, the emitted output
and the crate APIs may change in a minor release. Diagnostic *codes* are the
exception: they are append-only and never renumbered, because the repair loop keys
on them.

## [Unreleased]

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
