# GUML for VS Code

Language support for `.guml` files, backed by the compiler itself.

## What it gives you

| Feature | Where it comes from |
|---|---|
| Diagnostics as you type | `guml_compiler::check` — the same call the repair loop makes, so a human and a model see identical errors |
| Highlighting | The compiler's classifier, via semantic tokens. Prose is prose, because the server consults the registry |
| Format on save | `guml fmt`. An AST-preservation test enforces that formatting never changes meaning |
| Completion | The registry: tags at the start of a line, modifiers and that tag's attributes after it, declared state inside `{…}` |
| Hover | Tag documentation, and the enumerated domain of a state |
| Outline | Declarations and anchored sections |

The TextMate grammar in `syntaxes/` is generated from `guml registry` and exists only to colour a
file in the moment before the server attaches. It is the weaker highlighter by design: a regex
cannot know that the remainder of a `p` line is prose rather than four modifiers, which is why
`p Press the center button` is coloured correctly only once semantic tokens arrive.

## Building it

```sh
cargo build -p guml-lsp --release   # the server
npm install                          # the client's dependencies
npm run gen:grammar                  # regenerate the grammar from the registry
npm run compile                      # build the extension
```

Then press F5 in VS Code to launch an Extension Development Host.

The extension finds the server on `PATH`, or in `target/release` / `target/debug` of the open
workspace, or wherever `guml.serverPath` points.
