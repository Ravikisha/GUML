# GUML developer tooling — formatter, highlighting, and what else is required

Design and current state. The formatter and the classifier are built; the rest is specified
so the order of work is a decision already made rather than one taken under pressure.

---

## 1. The formatter (`crates/guml-fmt`, `guml fmt`) — built

### Who it is for

An LLM canonicaliser first, a `gofmt` second. That ordering decides the architecture: the
primary caller formats output that does *not* parse yet. A model that indented with a tab or
three spaces has produced a whitespace problem, and whitespace problems must never cost a
repair round.

### Why it works on the line stream, not the AST

`parse → print` is the obvious design and it fails twice here:

1. **The lexer discards comments.** `guml-syntax` skips `//` lines before they become
   tokens. An AST round-trip would silently delete every note the author wrote.
2. **It requires a clean parse**, which is exactly what the caller does not have.

So the formatter consumes `lex()` output plus the raw source. It sits *below* the parser,
which also keeps it clear of the `guml-codegen`/`guml-parser` cycle (invariant 7).

```
guml-syntax ──► guml-fmt ──► text
                  ├── trivia.rs   comments + blank lines, recovered from source
                  ├── layout.rs   nesting, mirroring the parser's indent rule exactly
                  └── print.rs    one line → one canonical line
```

### What is discretionary, and what is not

The formatter may change indentation, inter-token spacing, attribute punctuation and column
alignment. It may not change anything the AST records:

| Never touched | Why |
|---|---|
| Prose after a text tag | `p Two  spaces` is content. Respacing it edits the product's copy. |
| `tier`/`faq` content lines | Stored verbatim in `text_lines`. Even normalising `\|` spacing changes the tree. |
| Action bodies | The AST holds the string. Normalising `;` spacing looked safe and was caught by the preservation test. |
| Element order | The document. Only *declarations* may be reordered, and only in canonical mode. |

### Rules

| Rule | Form |
|---|---|
| Indent | 2 spaces per nesting level, computed from the stack — never the author's count |
| Tabs, CRLF, trailing space | Removed |
| Token gap | One space |
| Attributes | `name=value`, tight |
| `state` domains | `all\|open\|done`, tight — those bars are enum separators |
| Content bar | ` \| `, one space each side |
| `type` bodies | `{id, title, done:bool}` |
| Mutations under `data` | Aligned as a five-column table: name, method, url, body, flags |
| File | Ends in exactly one newline |

### Two modes, one engine

- **Default** — keeps comments and single blank lines; collapses runs; inserts one blank
  line at the seam between declarations and the tree.
- **`--canonical`** — strips comments and blank lines, hoists and sorts declarations, drops
  needless quotes, collapses alignment. Two documents that mean the same thing produce the
  same bytes, which is what dedup, caching and inter-run consistency measurement need. A
  benchmark cannot compare generations that differ only in blank lines.

### The invariant, and how it is enforced

`ast(fmt(x)) == ast(x)` for every input that parses.
`crates/guml-compiler/tests/fmt_preserves.rs` checks 17 ugly-but-valid inputs plus all three
fixtures, in both modes, comparing span-stripped ASTs. Canonical mode compares the element
tree exactly and declarations as a set, because reordering them is the point.

It has already earned its place: it caught the action-body normalisation, which was
plausible, useful-looking, and an AST change.

Also tested: idempotence, comment survival, no new diagnostics, and that the committed
fixtures are already in formatted form — if the formatter and the fixtures disagree, one of
them is wrong and the published token counts stop meaning anything.

### CLI

```sh
guml fmt file.guml              # print
guml fmt --write a.guml b.guml  # rewrite in place
guml fmt --check .              # exit 1 if unformatted; for CI
guml fmt --canonical file.guml  # the benchmark's form
cat x.guml | guml fmt           # stdin → stdout, what editors want
```

### Known limitation

Whitespace *inside* an action body is preserved, so canonical form is not canonical for
`>a; b` vs `>a ;  b`. Actions are strings until the expression parser lands (Phase 2); at
that point they become structure and can be formatted properly.

---

## 2. Syntax highlighting (`guml_fmt::highlight`, `guml highlight`) — built

### Why a regex grammar cannot do this

Whether a line's remainder is structure or prose depends on the tag, resolved against the
component registry — the same ambiguity that forced the lexer to be line-oriented. A
TextMate grammar cannot consult a registry, so it will always colour

```
p Press the center button
```

as a tag followed by three modifiers, because `center` is a real modifier. Same for `full`,
`start`, `end`, `loading`. In English prose those words are common.

So classification runs the real lexer and the real registry and everything else consumes the
result. Fourteen classes, each with a stable machine name and a mapping to the LSP
`SemanticTokenType` an editor already themes:

`tag` `directive` `modifier` `binding` `string` `number` `attr` `action` `prose` `comment`
`route` `anchor` `punct` `text`

Two rules that took a bug each to get right:

- **A binding inside prose is still a binding.** `head Tasks — {tasks.open.count} open`
  renders a live number; flattening it to prose hides the only executable part of the line.
- **Content after `\|` is prose**, not structure — the same rule the formatter follows.

### Consumers

| Consumer | Path |
|---|---|
| CLI | `guml highlight file.guml [--format human]` |
| Docs site, playground | wasm `highlight()`, and a TypeScript tokeniser for server rendering |
| LSP | `semanticTokens`, via `Class::lsp_type()` |
| TextMate grammar | Generated from the registry (planned) |

### The docs site keeps a second implementation, deliberately

Server rendering has to be synchronous, and loading wasm at build time for every inline
snippet is a lot of machinery. So `docs/lib/highlight.ts` stays — but:

- its **vocabulary is generated** from `guml registry` and from probing `guml highlight`
  (`lib/vocabulary.generated.ts`). The hand-maintained copy had already drifted: it listed
  `h3`, which the registry does not define.
- its **rules are checked mechanically**. `pnpm check:highlight` runs both implementations
  over every `.guml` file in the repository and fails on any disagreement. Currently 385
  spans across 6 documents.

A second implementation is acceptable when a machine proves it agrees. It is not acceptable
on the strength of a code review.

> **Byte offsets.** The compiler reports byte offsets; JavaScript strings are UTF-16. `—` is
> one JS character and three bytes. Slicing spans with those offsets shreds any line
> containing an em dash — which is how this was found. Consumers should slice bytes, or use
> the row-oriented output.

---

## 3. What else is required

### Now — built

| Tool | State |
|---|---|
| `guml fmt` with `--write`, `--check`, `--stdin`, `--canonical` | done |
| `guml highlight`, `guml_fmt::highlight` | done |
| wasm `format()` and `highlight()`, exposed from the `guml` npm package | done |
| Playground format button | done |
| Generated docs vocabulary + parity check | done |

### Next

**Repair-loop hook.** Format before parse, so `GUML0001` (tab), `GUML0010` (inconsistent
dedent) and `GUML0011` (unexpected indent) are fixed with no model call. The saving is
measurable and should be reported as a separate line in the Phase 5 telemetry, not folded
into the model's success rate.

**Language server** (`tower-lsp`). Most of it is plumbing over things that already exist:

| Feature | Reuses |
|---|---|
| Diagnostics | `check()` — humans and models get identical errors |
| Semantic tokens | `Class::lsp_type()` |
| Formatting, format-on-save | `guml fmt --stdin` |
| Completion | The registry: tags, modifiers, per-tag attributes, declared state names |
| Hover | Registry docs plus the per-tag token cost |
| Code actions | The `suggestion` field diagnostics already carry |
| Document symbols | `page`, `state`, `data`, sections |

**VS Code extension.** Language configuration (`//` comments, indent rules), a generated
TextMate grammar for colour before the server attaches, the LSP client, and a preview pane
driven by the same wasm build the playground uses.

**`guml explain GUML0030`.** Codes are append-only and machine-keyed; humans need the prose.

### Phase 7

- **tree-sitter grammar** — GitHub, Neovim, Zed. Deliberately redundant with the
  hand-written parser, per `TECH-STACK.md`.
- **`prettier-plugin-guml`** wrapping the wasm formatter, so format-on-save works everywhere
  without an extension.

### Cut, deliberately

`guml new` scaffolding, range formatting, rename, a debugger. None of them serve a v1 whose
users are mostly models.
