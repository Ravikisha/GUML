# GUML conformance suite

Each `.txt` file here is a sequence of cases in the shape CommonMark's `spec.txt` uses: an input, and
what a conforming implementation must produce from it.

```
:::: name of the case
level: core            (optional; `core` or `app`, default `app`)
--- guml
page Counter
metric {count}
--- ast
page=Counter
metric bind={count}
--- diagnostics
GUML0033 3
::::
```

Three sections, all optional after `guml`:

- **`ast`** — a structural fingerprint, one line per declaration and element, indented by depth. Not
  the full serialised AST: a fingerprint stays readable in a diff and does not churn every time a
  field is added, which is what makes these files reviewable by a human.
- **`diagnostics`** — `CODE line` pairs, in any order. An expected diagnostic that does not appear is
  a failure, and so is a diagnostic that appears but is not expected. The second half matters more:
  it is what stops a change from quietly adding noise to every document.
- **`html`** — substrings that must appear in the `html` backend's output. Used sparingly, for cases
  where the emitted document *is* the specified behaviour (`lang`, `<details>`, escaping).

  Never assert a **class string** here. Classes come from the theme, and a theme is replaceable by
  design — a case that pins `class="mt-1 text-sm text-slate-500"` is testing the shipped theme, not the
  language, and breaks the moment the theme gains a dark variant. Assert the element and its content.

## Why this exists

Before it, the Rust implementation *was* the specification. Two implementations have already disagreed
in this project — the TypeScript highlighter against `guml highlight`, and an expression grammar that
existed twice — and each time the only reason it surfaced was that someone thought to compare them.

A conformance suite inverts the relationship: the files here are the authority, `cargo test -p
guml-compiler --test conformance` checks the Rust against them, and a second implementation in another
language can be checked against exactly the same files. That is the difference between a program and a
language.

## Adding a case

Write the case before the code. If a case here disagrees with the compiler, one of them is wrong and
the discussion is about which — that conversation is the point.

Cases are grouped by what they pin down, not by which crate implements them:

| file | what it pins |
|---|---|
| `syntax.txt` | lines, indentation, prose vs structure, comments |
| `directives.txt` | `page`, `type`, `state`, `data`, metadata |
| `elements.txt` | tags, positionals, modifiers, attributes, content blocks |
| `levels.txt` | what belongs to `core` and what needs `app` |
| `escapes.txt` | `js` / `raw`, and what each backend does with them |
| `diagnostics.txt` | the errors a conforming implementation must report |
