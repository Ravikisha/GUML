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

  **Adding a tag is not free, and this is the one place that says so.** A `def` may not shadow a tag
  (`GUML0093`), so a document that defined its own `stat` component stops compiling the release `stat`
  becomes builtin. Growing the vocabulary from 28 to 49 entries in 0.2 broke exactly that, in two
  places in this repo, both renamed to `kpi`. The failure mode is the acceptable one — compile time,
  loud, the name in the message, a one-word fix — but it is a breakage, not an addition, for any
  document that already used the name. The mitigations are that the collision is *detected* rather
  than silently reinterpreted, and that a host can pin a registry (below) rather than inherit
  whatever became builtin.

- **Per-entry registry metadata.** `children`, `slots`, `capabilities`, `positionals` and `since` may be
  added to an entry and may be widened. They may not be *narrowed*: adding a name to `children.deny`, a
  tag to `children.require`, or removing a `positionals` slot turns a document that compiles into one
  that does not. Widening — a longer `allow` list, a dropped `require`, an extra positional slot — is
  additive and always permitted.

  `positionals` is the one whose *absence* was a defect rather than a gap. Without it,
  `btn Add task primary` compiled with no diagnostic and emitted `<button>Add</button>`: the word `task`
  was deleted from the output. Four instances existed in this repo's own `portfolio.guml`, one of them
  truncating the author's name to `Ravi`. It is now `GUML0099` with an applicable quoting suggestion.
- **The modifier vocabulary.** A modifier may be added. Removing one breaks documents; re-pointing one
  at a different meaning is worse, because it changes what a document *looks like* with no diagnostic.
- **Per-tag attributes.** May be added. Removing one turns a valid attribute into `GUML0032`.
- **A `def`'s meaning.** Expansion is by-value substitution into bindings, attributes and prose, with
  positional parameters and exact arity, and a single `slot` receiving the call's children in the
  *caller's* scope. What may not change is what an existing `def` already means: making substitution
  lazy, making arity flexible, or resolving slot children in the def's scope would silently alter
  documents that compile now. Named slots may be added; the unnamed one keeps its meaning.
- **Conformance levels.** `core` and `app` exist. A third level may be added between or above them; the
  two that exist keep their names and their meaning.

## Unstable, pre-1.0

- Emitted code shape. The React backend's output is not a stable interface — it is regenerated, and it
  improves. Pin the compiler version if you have vendored its output.
- The theme rule format and the registry JSON schema. Both are new and the *shapes* may still change.
  The registry document now carries a top-level `version` (`Registry::builtin_version`) and each entry
  an optional `since`, so a host can diff two vocabularies and see what a bump added; the theme format
  still has neither, and giving it one is a 1.0 task.
- The AST as serialised by `guml ast`. Useful for tooling, not a contract.
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
