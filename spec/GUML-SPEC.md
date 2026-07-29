# GUML v0.1 — language specification

Written for a model: terse, complete, example-led. Everything here is implemented and tested
unless marked `PLANNED`. Budget ≤3,000 tokens assembled (spec + registry slice + examples);
`bench/phase0/preflight.mjs` enforces it, and rationale lives in the docs site, not here.

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
route /app -> Dashboard                       // PLANNED
auth clerk                                    // PLANNED

def stat label value                          // a user-defined component
  card sm center
    h {label}
    metric {value}

stat "Revenue" {total}                        // a call: arguments are positional
```

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

**Bindings** — `{expr}`: paths (`title`, `tasks.open.count`), comparison, boolean, arithmetic,
aggregates (`.count`, `.sum`, `.where`). Bindings are read-only.

## Tag vocabulary

Closed set. An unknown tag is a compile error with a `did you mean` suggestion.

| Kind | Tags | Notes |
|---|---|---|
| Container | `card` `row` `col` `section` `nav` `hero` `footer` `form` `tabs` `tier` `faq` | children are elements |
| Text | `h` `h1` `h2` `p` `text` `metric` `head` `empty` | line remainder is prose |
| Control | `btn` `link` `check` `toggle` | `>` gives the behaviour |
| Field | `input` `select` | first positional is the state it binds |
| Repeater | `list` `table` | children are the item template |

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

Per-tag extras: `btn` → `busy` `type`; `input` → `placeholder` `kind` `min` `max`;
`list`/`table` → `where` `sort` `of`; `tier` → `cta`; `faq` → `open`; `text` → `strike`.

## User-defined components

`def <name> <params…>` + indented body, expanded at compile time. In the body `{param}` substitutes
into a binding, an attribute value, or prose; any other `{name}` is the document's own. A literal
argument becomes text, a binding stays a binding.

Exact arity. No redefining an existing tag, no recursion, no parameter inside an action, no children
at a call site (slots are not implemented).

## Conformance levels

**core** is markup: containers, text, controls, `tier`, `faq`, `raw` — no I/O, no state, safe to
render from an untrusted agent. **app** adds `state`, `store`, `data`, actions, `js`, and the
repeaters that iterate a resource. An app construct at the core level is `GUML0091`, an error.

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

## Worked example

Three complete documents are appended below under **Examples of valid GUML**, including a CRUD page
with optimistic updates. Read those rather than a fourth one here.

## Rules a generator should follow

1. Start with `page <Name>`.
2. Declare `type`, `data`, `state` before the tree.
3. Never write class names, colours, spacing, or ARIA plumbing — use modifiers and let the
   compiler decide.
4. Never hand-write loading, empty, error, or rollback logic — declare the resource and the
   `empty` message.
5. Put `>action` last on its line.
6. Prose goes in text tags or after `|`, and needs no quoting: `p Set x=1 to enable` is prose,
   because `=` only starts an attribute when the name is one the tag accepts. Quote it if it
   contains `|`.
7. If something cannot be expressed, say so rather than inventing a tag.
