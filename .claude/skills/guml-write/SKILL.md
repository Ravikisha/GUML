---
name: guml-write
description: Use when writing, reading, reviewing, or debugging GUML source (.guml files) — authoring a fixture, hand-writing a spec for the Phase 0 spike, fixing a parse error, or judging whether generated GUML is correct. Loads the language rules and the common mistakes.
---

# Writing GUML

Full spec: `spec/GUML-SPEC.md`. Grammar: `spec/grammar.ebnf`. Live vocabulary:
`cargo run -q -p guml-cli -- registry`.

## The shape

```
page Counter
state count=0

card sm center
  h Clicks
  p Press the buttons to change the value.
  metric {count}
  row center
    btn Decrement ghost disabled={!count} >count--
    btn Increment primary >count++
```

Directives first (`page`, `type`, `data`, `state`), then the element tree. 2-space indent = nesting.
No closing tags, no imports, no class names.

## The seven rules that prevent most errors

1. `page <Name>` first.
2. `>action` **last on its line** — it swallows the rest of the line. `btn Go >count++ primary`
   silently puts `primary` inside the action; the compiler catches this specific case, but do not
   rely on it.
3. Never write presentation. Use modifiers (`primary`, `sm`, `center`); the compiler owns colours,
   spacing, and classes.
4. Never hand-write loading / empty / error / rollback logic. Declare the resource with
   `optimistic:` and give `empty` a message; the compiler generates the rest.
5. Prose is unquoted and unescaped. `p Some text.` or `card "Title" | body text`. Quote only when
   the text contains `|` or `=`.
6. Bindings are `{expr}` and are derived — never assigned. `{tasks.open.count}` not a state.
7. If something is not expressible, say so. Do not invent a tag; an unknown tag is a compile error.

## Checking your work

```sh
cargo run -q -p guml-cli -- check  path.guml            # human diagnostics
cargo run -q -p guml-cli -- check  path.guml --format json   # machine-readable
cargo run -q -p guml-cli -- build  path.guml            # emitted React to stdout
cargo run -q -p guml-cli -- lex    path.guml            # token stream, for parse mysteries
cargo run -q -p guml-cli -- ast    path.guml            # AST as JSON
```

A diagnostic with a `suggestion` field is mechanically applicable — take it literally.

## Common mistakes

| Symptom | Cause |
|---|---|
| `unknown tag` | Not in the registry. Check `guml registry`; take the suggestion. |
| Modifier ignored | It was written after `>action` and got swallowed. |
| Prose parsed as modifiers | Tag is not `TagKind::Text`. Use `p`/`text`, or put the prose after `|`. |
| `unexpected indentation` | Indented line with no parent element above it. |
| Tab error | Spaces only, 2 per level. |
| `does not accept the attribute` | Wrong tag for that attribute — check the per-tag list in the spec. |

## v0.1 limits — real, and worth stating rather than working around

The React backend lowers containers, text, and controls end to end. Resources, repeaters, forms,
tabs, faq, and tier are **parsed** but not yet lowered; they emit a warning and a TODO in the
output. That is deliberate (ROADMAP Phase 3) — an honest partial compiler beats a quietly wrong one.
