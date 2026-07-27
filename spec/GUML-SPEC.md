# GUML v0.1 — language specification

This file is the artifact that goes **into the model's context**, so it is written for a model:
terse, complete, example-led. Target budget ≤3,000 tokens including the registry slice and
examples. If it grows past that, the amortisation math weakens and in-context learnability
degrades — cut features, not explanation.

Everything described here is implemented and covered by tests unless marked `PLANNED`.

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
page Counter                                  // page name -> component name
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
```

A `data` block's indented children are its mutations: `name METHOD /url {body fields}
[optimistic[:strategy]]`. Strategies: `prepend`, `append`, `replace` (default).

**What `data` gives you for free** — none of this is written by hand: fetch on mount, request
cancellation, retry with backoff, loading state, error state, empty state, optimistic apply,
snapshot rollback on failure, and derived aggregates (`tasks.open.count`).

## Elements

```
tag [positionals…] [name=value…] [>action]
tag …                | content text
```

Order is free except `>action`, which **must be last** — it consumes the rest of the line.

**Positionals** — the first bare word or quoted string is the label/title. Others:

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
aggregates (`.count`, `.sum`, `.where`). Bindings are derived, never assigned — there is no
memoisation to get wrong.

## Tag vocabulary

Closed set. An unknown tag is a compile error with a `did you mean` suggestion.

| Kind | Tags | Notes |
|---|---|---|
| Container | `card` `row` `col` `section` `nav` `hero` `footer` `form` `tabs` `tier` `faq` | children are elements |
| Text | `h` `h1` `h2` `p` `text` `metric` `head` `empty` | line remainder is prose |
| Control | `btn` `link` `check` `toggle` | `>` gives the behaviour |
| Field | `input` `select` | first positional is the state it binds |
| Repeater | `list` `table` | children are the item template |

`tier` and `faq` take **content lines** as children rather than elements:

```
tier Pro $24/mo "For working developers" cta="Go Pro" /signup featured
  Unlimited projects
  Custom domains

faq open=1
  Can I export the code? | Yes. Every build is plain source.
```

Run `guml registry` for the live list, or `guml registry --tags btn,card,list` for a
prompt-sized slice.

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

`id` `class` `aria` `title` `hidden` `cols` `gap` `w` `if` `disabled` `loading` `readonly`
`required`. Per-tag extras: `btn` → `busy` `type`; `input` → `placeholder` `kind` `min` `max`;
`list`/`table` → `where` `sort` `of`; `tier` → `cta`; `faq` → `open`; `text` → `strike`.

## Escape hatches `PLANNED`

Every construct must have a way out, or the expressiveness cliff becomes an adoption wall:

```
js
  // arbitrary expression / handler code
raw react
  <SomeThirdPartyChart data={rows} />
```

The compiler tracks how often these appear — a rising escape-hatch rate is the early warning
that the vocabulary is too small.

## Worked example — CRUD with optimistic updates

```
page Tasks

type Task {id, title, done:bool, createdAt:date}
data tasks:Task[] GET /api/tasks
  add  POST   /api/tasks         {title}  optimistic:prepend
  save PATCH  /api/tasks/{id}    {done}   optimistic
  drop DELETE /api/tasks/{id}             optimistic

state draft=""
state filter=all|open|done

head Tasks — {tasks.open.count} open

form >tasks.add{title:draft}; draft=""
  input draft placeholder="Add a task…"
  btn Add primary disabled={!draft.trim()} busy="Adding…"

tabs filter

list tasks where={filter}
  check {done} >tasks.save
  text {title} strike={done}
  btn Delete quiet aria="Delete {title}" >tasks.drop
  empty Nothing here yet.
```

175 tokens. The equivalent hand-written React+TS+Tailwind is 1,434.

## Rules a generator should follow

1. Start with `page <Name>`.
2. Declare `type`, `data`, `state` before the tree.
3. Never write class names, colours, spacing, or ARIA plumbing — use modifiers and let the
   compiler decide.
4. Never hand-write loading, empty, error, or rollback logic — declare the resource and the
   `empty` message.
5. Put `>action` last on its line.
6. Prose goes in text tags or after `|`; never quote prose unless it contains `|` or `=`.
7. If something cannot be expressed, say so rather than inventing a tag.
