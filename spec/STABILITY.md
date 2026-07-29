# Stability policy

A 2004 Markdown document still renders today. That is not luck — it follows from a small syntax and a
refusal to remove anything. This file states what GUML promises so that a document written now still
compiles later, and so that a host embedding GUML knows what it is depending on.

Status: **v0.1, pre-1.0.** Until 1.0, the promises below are intentions with one exception — the
*append-only* rules, which are already enforced by tests and are treated as binding now, because they
are the ones the repair loop and third-party registries key on.

## The three levels of commitment

| | promise | enforced by |
|---|---|---|
| **Frozen** | never changes meaning, never disappears | `stability.rs` |
| **Additive** | may gain members, never lose or renumber them | `stability.rs` |
| **Unstable** | may change in any pre-1.0 release | — |

## Frozen

- **Indentation is structure.** Two spaces per level; children are the following lines with a strictly
  greater indent. A tab is an error.
- **A text tag takes its line remainder as prose, verbatim.** No quoting, no escaping, no reflowing.
  This is the content floor, and anything that drops or rewrites a word of prose is a defect.
- **`>` takes the rest of the line.** An action terminates its line by construction.
- **`//` at the start of a line is a comment.** Comments never affect layout.
- **An unknown tag is a compile error.** Not a passthrough, not a warning. This is the property most of
  the hallucination-resistance claim rests on.
- **Diagnostic ids are stable strings.** `GUML0033` means the same thing forever.

## Additive

- **Diagnostic codes.** Append-only: a new code takes the next number, and no existing number is reused
  or renumbered. The repair loop keys on these, so renumbering one would silently change what a model
  repairs.
- **The tag vocabulary.** A tag may be added. An existing tag's `name`, `kind` and conformance `level`
  may not change, because a document's meaning depends on all three: changing `kind` flips
  prose-versus-structure for every line using that tag, and changing `level` can make a valid core
  document invalid.
- **The modifier vocabulary.** A modifier may be added. Removing one breaks documents; re-pointing one
  at a different meaning is worse, because it changes what a document *looks like* with no diagnostic.
- **Per-tag attributes.** May be added. Removing one turns a valid attribute into `GUML0032`.
- **A `def`'s meaning.** Expansion is by-value substitution into bindings, attributes and prose, with
  positional parameters and exact arity. Slots may be *added* (a call may not take children today, so
  allowing it later breaks nothing). What may not change is what an existing `def` already means: making
  substitution lazy, or making arity flexible, would silently alter documents that compile now.
- **Conformance levels.** `core` and `app` exist. A third level may be added between or above them; the
  two that exist keep their names and their meaning.

## Unstable, pre-1.0

- Emitted code shape. The React backend's output is not a stable interface — it is regenerated, and it
  improves. Pin the compiler version if you have vendored its output.
- The theme rule format and the registry JSON schema. Both are new; the *shapes* may still change, and
  both carry no version field yet. Adding one is a 1.0 task.
- The AST as serialised by `guml ast`. Useful for tooling, not a contract.
- Slots for `def`. Not implemented, so nothing depends on them yet.
- Anything marked `PLANNED` in `spec/GUML-SPEC.md`.

## Deprecation

Nothing is removed before 1.0 without a deprecation path, and after 1.0 nothing is removed at all
within a major version. The path is:

1. The construct keeps working, and gains a warning naming its replacement.
2. The warning stays for at least one minor release.
3. Removal, if ever, only in a major release.

A construct that would be silently reinterpreted rather than deprecated is not eligible for this path —
it has to be a new construct with a new name. Silent reinterpretation is the one change no version
number protects a reader from.

## What a host should pin

- The **compiler version**, if you vendor emitted output.
- The **registry**, if you rely on tags beyond the builtins — publish it with `Registry::to_json` and
  load it explicitly rather than depending on what happens to be builtin.
- The **conformance level**, explicitly: pass `--core` if you mean markup. Defaulting to `app` is
  convenient and is not a security posture.

## How this is checked

`crates/guml-compiler/tests/stability.rs` fails if an append-only rule is broken: a diagnostic code
that changed its string, a tag that changed kind or level, a modifier or global attribute that
disappeared. The lists in that file are the record. Adding to them is normal; changing an existing
entry requires deleting a line, which is the point — it makes a breaking change visible in review
instead of shipping as a one-character diff.
