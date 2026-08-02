# Registry schema

A registry document declares components a host makes available to GUML documents. `guml add ./mine.json`
installs one into `guml.json`; `guml check --registry mine.json` loads it for one run; `Registry::from_json`
does the same in-process.

This file is the contract for someone writing one from outside the repository. The authority for the
*shapes* is `crates/guml-registry/src/lib.rs`; the authority for what the compiler does with each field is
the conformance suite in `spec/tests/`. **The builtin vocabulary is itself a registry document** —
`crates/guml-registry/components.json` — which is the strongest available test of this schema: if the load
path regresses, every builtin tag disappears and four hundred tests say so.

## Shape

```json
{
  "name": "@acme/design-system",
  "version": "1.4.0",
  "components": [ … ]
}
```

A bare array is also accepted, which is what a hand-written registry usually starts as:

```json
[ … ]
```

`name` and `version` are optional and are read by `guml add` and `guml registry --validate`. Nothing pins a
version yet — see **Not in the schema**.

## An entry

```json
{
  "name": "callout",
  "kind": "container",
  "level": "core",
  "since": "1.0.0",
  "doc": "Highlighted aside. Optional title as first positional.",
  "positionals": ["title"],
  "attrs": ["tone"],
  "element": "Callout",
  "import": "@acme/design-system",
  "children": { "deny": ["callout"] },
  "slots": ["footer"],
  "capabilities": { "needs_runtime": false, "backends": ["react", "svelte"] },
  "a11y": { "requires_label": false, "role": null, "focusable": false, "announces_state": false }
}
```

| field | required | default | meaning |
|---|---|---|---|
| `name` | yes | — | the tag as written in a document |
| `kind` | yes | — | see **Kinds**; this decides *parsing*, not just rendering |
| `doc` | yes | — | one line, written for a model. An empty one is a load error |
| `level` | no | `core` | `core` or `app` — see `spec/GUML-SPEC.md` |
| `since` | no | — | the version that introduced the entry, so two registries can be diffed |
| `positionals` | no | `[]` | the positional slots this entry reads, **in order**. See below |
| `attrs` | no | `[]` | attribute names accepted beyond the global set |
| `element` | no | — | what it lowers to. See **Lowering** — without it the entry validates and emits nothing |
| `import` | no | — | module to import `element` from. Required when `element` is PascalCase |
| `children` | no | unconstrained | `allow` / `deny` / `require`. See **Children** |
| `slots` | no | `[]` | named slots the body accepts beyond the unnamed one |
| `capabilities` | no | all false | what the entry needs from its host. See **Capabilities** |
| `a11y` | no | all false / null | what the compiler must guarantee. See **Accessibility** |

## Kinds

`kind` is the load-bearing field, because it decides whether a line's remainder is **prose** or
**structure** — and no tokeniser can work that out alone. `p Press the buttons` is prose;
`btn Decrement ghost` is a label plus a modifier. Same shape, different parse, and the registry is what
distinguishes them.

| `kind` | children | line remainder | examples |
|---|---|---|---|
| `container` | elements | a quoted title, then modifiers | `card` `row` `section` `nav` |
| `text` | none | **prose, verbatim** | `p` `h` `h1` `metric` `head` |
| `control` | none | a label, then modifiers; `>` gives behaviour | `btn` `link` `check` |
| `field` | none | first positional is the state it binds | `input` `select` |
| `repeater` | the item template | the resource name | `list` `table` |

Because `kind` changes how existing documents parse, it is **append-only**: a shipped tag may never change
kind. See `spec/STABILITY.md`.

One thing that follows from `text` and catches people: **modifiers do not work on a text tag**, because its
remainder is prose taken verbatim. `badge danger Breaking` rendered the string "danger Breaking" for a
release, while `badge`'s own doc line said to use those modifiers for tone. `badge` is a `container` with a
`label` positional now, and `GUML0102` warns for the rest of the kind. If your component needs a modifier,
it is not `text`.

## Positionals

`["label"]` for a `btn`; `["name", "price", "blurb"]` for a `tier`. Empty means unspecified and nothing is
checked.

Declare it. Without it, `btn Add task primary` parsed as two text positionals plus a modifier, codegen read
only the first, and the emitted button said `Add` — the word `task` deleted with no diagnostic and no trace.
The count is what makes the extra words *countable*, so `GUML0099` can report them and suggest the quoting
that fixes it. Names rather than a number, because a diagnostic and a docs page both need to say which slot.

## Lowering

Without `element`, a loaded component is half-usable: `guml check` accepts a document using `callout` and
`guml build` warns "does not yet lower tag `callout`" and emits a `TODO`. The compiler is right to refuse —
nothing told it what a `callout` is — so a package that wants output has to say.

Two spellings, distinguished by **case**, because they mean genuinely different things:

| `element` | emits | when |
|---|---|---|
| `aside`, `figure` (lowercase) | that HTML element, with the theme's classes, exactly as a builtin | the component is a styled wrapper |
| `Callout` (PascalCase) | `<Callout …>` plus an import from `import` | **a design system** |

The second is the right answer for a design system, and the reason is worth stating: a compiler that tried
to reimplement someone's component would get it subtly wrong, and the whole point of a registry package is
that the host already has the implementation. A PascalCase `element` with no `import` is a **load error** —
emitting `<Callout>` into a file that does not import it is a silent mis-lowering with extra steps.

An entry with no `element` at all is legitimate and warned about: it closes the vocabulary for validation
without providing output, which a host that only wants `guml check` may genuinely want.

## Children

```json
"children": { "allow": ["option"], "deny": [], "require": ["option"] }
```

| field | meaning |
|---|---|
| `allow` | only these tags may be direct children. Empty means no restriction |
| `deny` | these may never be, even when `allow` is empty. `deny: ["*"]` means **no children at all** |
| `require` | at least one direct child of each of these is required |

`deny` wins over `allow`, so "everything except one thing" is expressible without enumerating the
complement. Violations are `GUML0100`.

The point of this being registry data rather than a `match` arm in the compiler: `select` accepting only
`option`, and `stepper` requiring at least one `step`, are the same mechanism a host's own `combobox` gets
for free.

`allow` is extendable; `deny` and `require` are **frozen** once published, because narrowing either
invalidates a document that compiles today. See `spec/STABILITY.md`.

## Capabilities

```json
"capabilities": { "needs_runtime": true, "network": false, "storage": false, "backends": ["react"] }
```

| field | effect |
|---|---|
| `needs_runtime` | needs client-side JavaScript. The no-JavaScript `html` backend reports it and emits an inert `<template>` rather than markup that silently does nothing |
| `network` | issues requests of its own. Surfaces in `guml capabilities` and in the generated CSP |
| `storage` | reads or writes host storage. Reported as `false` rather than omitted, so a consumer can tell "no" from "unknown" |
| `backends` | backends known to lower this entry. **Empty means every backend**, which is the honest default for a component whose author has not tested one |

This is the registry half of the security posture the `core`/`app` split starts. `level` answers "may an
untrusted document contain this at all"; these answer the narrower question a *backend* has to ask. Declaring
it as data is what lets each backend report the gap instead of re-deriving the list — a hardcoded list inside
`theme.rs` was exactly the bug this replaced.

An `app`-level entry that declares none of the three gets a warning: a core host will refuse it with no
stated reason.

## Accessibility

The reason this is data rather than compiler code: without it, the accessibility guarantee would stop at the
tags we shipped. A third-party component has to be able to *declare* what the compiler must enforce.

| field | effect |
|---|---|
| `requires_label` | a control with no text label and no `aria` is `GUML0050`, a **hard error** |
| `role` | ARIA role the compiler emits when the chosen HTML element does not imply it |
| `focusable` | must be reachable and operable from the keyboard; picks up the theme's focus treatment |
| `announces_state` | assistive technology has to hear the state change (`aria-pressed`, `aria-checked`) |

```json
{ "name": "avatar", "kind": "control", "doc": "Round user image.",
  "a11y": { "requires_label": true } }
```

That entry alone makes `avatar` with no `aria` a compile error. Nothing in the compiler knows what an avatar
is.

## Auditing one before you install it

```sh
guml registry --validate ./vendor/design-system.json
```

```text
@acme/design-system 1.4.0: 7 component(s)
  callout btn My Tag orphan badcomp needy picky
  ~100 est. prompt tokens for the whole package
  warning: `needy` is app-level but declares no capability that justifies it …
  error:   `btn` is a builtin tag; a package may add tags but not redefine them
  error:   `badcomp` lowers to the host component `BadComp` but declares no `import` …
```

`Registry::audit_package` reports **everything wrong at once**, without loading it — the same one-pass rule
the parser follows, for the same reason: a tool that stops at the first problem turns one fix into N rounds.
`guml add` runs the audit first and refuses on any error. Errors:

| error | why it is fatal |
|---|---|
| a builtin name | a document using `btn` must mean the same thing everywhere |
| not valid JSON | reported as one error rather than a panic |
| an unusable name | the lexer reads a tag as a bare lowercase word, so `My Tag` or `Card` could be registered and never matched. Lowercase letters, digits and `-`, starting with a letter |
| a duplicate within the package | which entry wins would be load-order dependent |
| an empty `doc` | the doc line *is* the entry's prompt representation, so the component would be present in the vocabulary and impossible for a model to be told about |
| a PascalCase `element` with no `import` | the emitted file would reference an undefined name |

Warnings:

- an `app`-level entry with no capability justifying it
- `children.allow`/`require` naming a tag no builtin or package component provides
- no `element` at all — validates, lowers nowhere
- a `container` with `requires_label`, whose accessible name comes from a title positional that is easy to
  omit

The audit also reports **`approx_prompt_tokens`**: the total prompt cost of every entry at ~3.6 chars/token.
Labelled `approx` because a published figure comes from the target model's own tokenizer, and it answers the
question a host actually asks — "will adding this package blow my prompt budget". The real answer is
per-slice, because `guml registry --tags` sends only the tags a task needs.

## Pinning

```json
{
  "registries": [
    "./design-system.registry.json",
    { "path": "./vendor/widgets", "version": "0.1.0" }
  ]
}
```

A bare path or `{ "path", "version" }`. With a version, loading **fails** — not warns — when the package
declares a different one:

```text
./vendor/widgets/guml.registry.json declares version 0.2.0, but guml.json pins it to 0.1.0
a registry decides which tags a document may use, so a version change is a change in what its documents
mean — update the pin deliberately
```

Refusing rather than warning, because a document compiled against the wrong vocabulary is not a degraded
build; it is a different document, and the failure would otherwise surface somewhere unrelated. The check runs
*before* the vocabulary is extended, so a mismatched package never contributes a tag.

**Exact equality, not a range.** A range needs a resolver, a lockfile, and a policy for what "compatible"
means for a vocabulary — and semver's answer ("additive is a minor bump") is the one this project has evidence
against: adding a tag is not purely additive, because a `def` may not shadow one. Growing the builtin
vocabulary from 28 entries to 49 broke exactly that in three places here. Exact is the version that needs no
design decision.

`guml add` writes the pin for you from the version it just audited. A package with no `version` gets a bare
path, because inventing one would be a claim the package did not make.

## Installing one

```sh
guml add ./vendor/design-system.json     # audits, then writes it into guml.json
guml add ./vendor/design-system --dry-run # a directory containing guml.registry.json
```

`guml add` takes a **path and never a URL**, and that is a deliberate constraint rather than an unimplemented
feature. A registry decides which tags a document may use and which classes the compiler emits, so resolving
one over the network at build time would make compiler output depend on a remote server — the wrong trade for
a project whose claim is reliability. It is also the mitigation for package tampering that needs no signing
scheme; see **Not in the schema**.

Installed registries are listed in `guml.json`, which `guml` discovers by walking up from the file being
compiled.

## Reference docs for a loaded vocabulary

```sh
guml registry --docs > VOCABULARY.md
```

Generated rather than written, for the same reason the docs site's vocabulary block is: a hand-written table
drifts from the registry silently, and a component page listing an attribute the compiler rejects is worse
than no page. A package author gets documentation for their own components from the same command.

## What is rejected at load, and why

Loading fails rather than merging quietly. All three surface as `GUML0092`; `guml explain GUML0092` restates
them.

| rejection | reason |
|---|---|
| **shadowing a builtin** | silently replacing `btn` means the same file renders differently depending on which registry was loaded, with no diagnostic — exactly the failure a closed vocabulary exists to prevent |
| **an unusable name** | see above |
| **an app-level entry in a core host** | *skipped*, not merged. A host that asked for markup only gets markup only, even if the registry says otherwise — the host's decision wins |

## Publishing one

`Registry::to_json` serialises the vocabulary a host accepts, **including builtins**. That is how a host
publishes its contract instead of describing it in prose, and it is the input a prompt-slice or a third-party
tool should read rather than assuming the builtin set.

Note the asymmetry: `to_json` emits everything and `from_json` refuses builtins. So a published registry is a
*description*, not something to feed back in unchanged.

## Not in the schema

Named honestly, because a registry author will look for them.

- ~~Version pinning.~~ **Done.** A `guml.json` entry may be `{ "path": …, "version": … }`, and loading fails
  if the package declares a different version. `guml add` records the pin automatically, since it has just
  audited that exact file. See **Pinning** below.
- **Signing.** `guml add` takes a path and never a URL, so a registry cannot be fetched from a remote server
  at build time — which is the mitigation that needs no design decision. Signed packages need a signing
  scheme and a key-distribution story, and picking either without input would be inventing policy.
- **Exact per-entry token cost.** `approx_prompt_tokens` is an estimate. The exact figure needs the target
  model's tokenizer, which needs an API key.
- **A theme binding.** Presentation lives in a separate theme document (`crates/guml-codegen/themes/`), so a
  component cannot ship its own styling. A registry entry says what a tag *is*; a theme says what it looks
  like. A PascalCase `element` sidesteps this entirely — the host's component brings its own styling.
- **Per-entry modifiers.** The modifier vocabulary is global and closed. A component cannot declare one of
  its own, so `attrs` is the extension point.
