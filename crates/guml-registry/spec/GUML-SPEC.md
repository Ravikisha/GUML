<!--
  Maintainer notes. An HTML comment, so it costs a model nothing: every byte of this file is paid for
  once per generation, and text addressed to a human is pure overhead in a prompt.

  Budget: ≤3,000 est. tokens for the *assembled* prompt (this file + registry slice + examples),
  enforced by `bench/phase0/preflight.mjs`. Invariant 5 in CLAUDE.md states why.

  The vocabulary is deliberately **not** enumerated here. The assembled prompt appends an
  `Available tags` block generated from the compiler, one line per tag and only the tags a task needs,
  so a list here would be a duplicate that can drift — and growing the vocabulary from 28 to 49 tags in
  0.2 is exactly what made the difference between fitting the budget and not.
-->

# GUML v0.1 — language specification

Written for a model: terse, complete, example-led. Everything here is implemented and tested.

## Shape of a file

One page per file. Lines are significant. Indentation (2 spaces per level) is nesting. No
closing tags, no imports, no braces for blocks.

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
    btn Reset quiet >count=0
```

- Blank lines and `//` comment lines are ignored and never affect nesting.
- Tabs are an error. Spaces only.

## Directives (top level, any order)

```
page Counter title="Clicks" lang=en dir=ltr   // name -> component; metadata optional
type Task {id, title, done:bool, createdAt:date}   // fields default to `string`
state count=0                                 // local state; type inferred
state draft=""
state filter=all|open|done                    // enumerated domain; first value is initial
data tasks:Task[] GET /api/tasks              // a resource
  add  POST   /api/tasks      {title}  optimistic:prepend
  save PATCH  /api/tasks/{id} {done}   optimistic
  drop DELETE /api/tasks/{id}          optimistic
on mount >tasks.list                          // an effect; the trigger is the dependency
on {filter} >tasks.list                       // re-runs when `filter` changes
```

`def` is a directive too; see **User-defined components** below.

A `data` block's indented children are its mutations: `name METHOD /url {body fields}
[optimistic[:strategy]]`. Strategies: `prepend`, `append`, `replace` (default).

**What `data` gives you for free** — none of it written by hand: fetch on mount, request
cancellation, loading/error/empty state, optimistic apply, rollback on failure, and aggregates
(`tasks.open.count`).

## Elements

```
tag [positionals…] [name=value…] [>action]
tag …                | content text
```

Order is free except `>action`, which **must be last**: it consumes the rest of the line.

**Positionals** — first bare word or quoted string is the label/title. Others:

| Form | Meaning |
|---|---|
| `Word` / `"Two words"` | label / title text |
| `primary` `ghost` `sm` `center` … | a modifier from the closed vocabulary |
| `{expr}` | a binding |
| `/signup` | a route (link target, CTA target) |
| `#features` | an anchor id |

**Attributes** — `name=value`, where value is a string, number, bare word, or `{binding}`.

**Actions** — `>` then statements separated by `;`:

```
>count++            >count--          >count=0
>draft=""           >tasks.add{title:draft}; draft=""
```

**Content** — for text tags the whole line remainder is prose, taken verbatim (no quoting, no
escaping). For other tags, `|` starts content:

```
p Press the buttons to change the value.
card "Ship in minutes" | Describe the page, get a deployable build.
```

**Bindings** — `{expr}`: paths (`title`, `tasks.open.count`), comparison, boolean, arithmetic.
Aggregates: `.count` `.sum` `.open` `.done` `.trim` `.lower` `.upper`. After a field they narrow the
rows first — `invoices.paid.amount.sum`. Read-only.

## Tag vocabulary

Closed set. An unknown tag is a compile error with a `did you mean` suggestion.

**The `Available tags` block is the authoritative list**, generated from the compiler. Kinds:

| Kind | Means |
|---|---|
| Container | children are elements; first positional is its title |
| Text | the whole line remainder is prose, verbatim |
| Control | interactive leaf; `>` gives the behaviour |
| Field | first positional is the state it binds |
| Repeater | children are the item template |

- **A bare word past the last positional slot is an error.** `btn Add task` is one slot, two words:
  quote it — `btn "Add task"`. Nothing is dropped silently.
- **Modifiers do not work on a Text tag**, whose remainder is prose: `badge danger X` renders "danger X".
- **`if={expr}`** renders an element only while true — how `modal`/`drawer`/`toast` show and hide.
- `select` options come from the bound state's domain, or from `option` children.
- A repeater takes a `data` resource, or any in-scope array with `of=Type`. `table` needs `cols="A, B"`.

`tier` and `faq` take **content lines**, not elements:

```
tier Pro $24/mo "For working developers" cta="Go Pro" /signup featured
  Unlimited projects
  Custom domains

faq open=1
  Can I export the code? | Yes. Every build is plain source.
```

Live list: `guml registry`, or `--tags btn,card,list` for a slice.

## Modifiers

Semantic, never utility classes. The compiler owns all presentation.

```
intent   primary secondary outline ghost quiet danger featured
size     xs sm md lg xl
layout   center start end between wrap tight loose full
state    disabled loading readonly required
```

`disabled` as a bare word is static; `disabled={expr}` is bound.

## Global attributes

`id` `aria` `title` `hidden` `cols` `gap` `w` `if` `disabled` `loading` `readonly`
`required`. No `class`: presentation is the theme's.

Per-tag extras: `btn` → `busy` `type`; `input`/`select` → `placeholder` `kind` `min` `max`;
`list`/`table` → `where` `sort` `of` `cols` (a `table`'s column headers, comma-separated); `tier` → `cta`; `faq` → `open`; `text` → `strike`;
`img` → `src` `alt`; `progress` → `value` `max`; `stat` → `delta`; `step` → `done` `current`.

## User-defined components

`def <name> <params…>` + indented body, expanded at compile time. In the body `{param}` substitutes
into a binding, an attribute value, or prose; any other `{name}` is the document's own. A literal
argument becomes text, a binding stays a binding.

A `slot` in the body is where a call's children go, so a `def` can wrap content; at most one, and the
children keep the caller's scope. Exact arity. No redefining an existing tag, no recursion, and no
parameter inside an action.

## Conformance levels

**core** is markup — no I/O, no state, safe to render from an untrusted agent. **app** adds `state`,
`store`, `data`, actions, `js`, the repeaters, and `modal`/`drawer`/`toast`. An app construct at the
core level is `GUML0091`, an error.

## Escape hatches

For anything the vocabulary cannot express:

```
js
  // arbitrary expression / handler code
raw react
  <SomeThirdPartyChart data={rows} />
```

Bodies are verbatim and never checked; indentation is preserved and `//` is the host language's
comment. `raw <target>` is skipped by other backends. Each block reports `GUML0090`, so the
escape-hatch rate stays countable.

## Rules a generator should follow

1. Start with `page <Name>`; declare `type`, `data`, `state` before the tree.
2. Never write class names, colours, spacing or ARIA plumbing — use modifiers.
3. Never hand-write loading, empty, error or rollback logic — declare the resource and the `empty`
   message.
4. Put `>action` last on its line.
5. Prose needs no quoting (`p Set x=1 to enable` is prose); quote it if it contains `|`.
6. If something cannot be expressed, say so rather than inventing a tag.
