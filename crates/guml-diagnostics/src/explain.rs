//! Long-form explanations, one per diagnostic code.
//!
//! A diagnostic message has to fit on a line and be useful to a repair loop, which means it says
//! *what* is wrong and not *why the language is like that*. `guml explain GUML0064` is where the
//! second half lives.
//!
//! This also gives the code list one home. It previously existed only inside a uniqueness test,
//! so adding a code meant remembering to edit a test that nothing pointed at — and the compiler
//! could not tell you when you forgot. [`Code::ALL`] now drives the test, the explainer, and the
//! completeness check below.

use crate::Code;

impl Code {
    /// Every code, in numeric order. Append-only, like the ids themselves.
    pub const ALL: &'static [Code] = &[
        Code::TabIndent,
        Code::UnterminatedString,
        Code::UnterminatedBrace,
        Code::UnexpectedChar,
        Code::InconsistentDedent,
        Code::UnexpectedIndent,
        Code::ExpectedTag,
        Code::ExpectedValue,
        Code::TrailingTokensAfterAction,
        Code::BadExpression,
        Code::UnknownTag,
        Code::UnknownModifier,
        Code::UnknownAttr,
        Code::UnknownState,
        Code::DuplicateState,
        Code::MissingPageDirective,
        Code::IconControlWithoutLabel,
        Code::InputWithoutLabel,
        Code::UnknownMutation,
        Code::UnknownTypeName,
        Code::UnknownBodyField,
        Code::AssignToNonState,
        Code::TypeMismatch,
        Code::DuplicateAnchor,
        Code::DanglingAnchor,
        Code::EmptyRepeater,
        Code::MultipleH1,
        Code::UnusedState,
        Code::UnusedResource,
        Code::NotEnumerated,
        Code::BadAttrValue,
        Code::DuplicateAttr,
        Code::BadMethod,
        Code::BadUrl,
        Code::EscapeHatch,
        Code::AppLevelConstruct,
        Code::BadRegistry,
        Code::DuplicateDef,
        Code::DefArity,
        Code::RecursiveDef,
        Code::EmptyDef,
        Code::DefParamUnsupported,
        Code::BadEffect,
        Code::DroppedPositional,
        Code::BadChild,
        Code::RowMutationOutsideRepeater,
        Code::ModifierInProse,
        Code::ResourceNotAList,
        Code::RepeaterNeedsRowType,
    ];

    /// Look a code up by its id, as a human types it.
    pub fn from_id(id: &str) -> Option<Code> {
        let wanted = id.trim().to_ascii_uppercase();
        // Accept `64` and `0064` as well as `GUML0064`: nobody wants to type the prefix.
        let wanted = if wanted.starts_with("GUML") {
            wanted
        } else {
            match wanted.parse::<u32>() {
                Ok(n) => format!("GUML{n:04}"),
                Err(_) => wanted,
            }
        };
        Code::ALL.iter().copied().find(|c| c.id() == wanted)
    }

    /// A one-line title, for listings.
    pub fn title(self) -> &'static str {
        match self {
            Code::TabIndent => "tab used for indentation",
            Code::UnterminatedString => "string is not closed",
            Code::UnterminatedBrace => "`{` group is not closed",
            Code::UnexpectedChar => "character the lexer cannot read",
            Code::InconsistentDedent => "sibling indented differently from its siblings",
            Code::UnexpectedIndent => "indented line with no parent above it",
            Code::ExpectedTag => "line does not start with a tag",
            Code::ExpectedValue => "attribute has no value",
            Code::TrailingTokensAfterAction => "tokens after an action body",
            Code::BadExpression => "expression outside the grammar",
            Code::UnknownTag => "tag is not in the registry",
            Code::UnknownModifier => "modifier is not in the vocabulary",
            Code::UnknownAttr => "attribute is not accepted by this tag",
            Code::UnknownState => "name is not declared",
            Code::DuplicateState => "state declared twice",
            Code::MissingPageDirective => "file has no `page` directive",
            Code::IconControlWithoutLabel => "control has no accessible name",
            Code::InputWithoutLabel => "field is labelled only by its placeholder",
            Code::UnknownMutation => "resource has no such mutation",
            Code::UnknownTypeName => "type is not declared",
            Code::UnknownBodyField => "optimistic body field is not on the type",
            Code::AssignToNonState => "assignment target is not a state",
            Code::TypeMismatch => "assigned value does not fit the state",
            Code::DuplicateAnchor => "id used more than once",
            Code::DanglingAnchor => "link points at an id nothing defines",
            Code::EmptyRepeater => "repeater has no item template",
            Code::MultipleH1 => "more than one `h1`",
            Code::UnusedState => "state declared but never used",
            Code::UnusedResource => "resource fetched but never rendered",
            Code::NotEnumerated => "control needs an enumerated state",
            Code::BadAttrValue => "attribute value is the wrong shape",
            Code::DuplicateAttr => "attribute set twice",
            Code::BadMethod => "not an HTTP method",
            Code::BadUrl => "not a request path",
            Code::EscapeHatch => "escape hatch used",
            Code::AppLevelConstruct => "app-level construct at the core level",
            Code::BadRegistry => "registry could not be loaded",
            Code::DuplicateDef => "component name is already taken",
            Code::DefArity => "wrong number of arguments",
            Code::RecursiveDef => "component expands into itself",
            Code::EmptyDef => "component has no body",
            Code::DefParamUnsupported => "parameter used where it cannot be substituted",
            Code::BadEffect => "`on` effect with no trigger or no action",
            Code::DroppedPositional => "more positional words than the tag reads",
            Code::BadChild => "child this component does not accept",
            Code::RowMutationOutsideRepeater => "row mutation invoked with no row in scope",
            Code::ModifierInProse => "modifier at the start of prose renders as text",
            Code::ResourceNotAList => "resource type is a single object, not a list",
            Code::RepeaterNeedsRowType => "repeater over a derived array with no `of=` row type",
        }
    }

    /// Why the rule exists, and what to do about it.
    pub fn explain(self) -> &'static str {
        match self {
            Code::TabIndent => {
                "\
GUML derives nesting from leading spaces, and a tab has no defined width — the same file would
nest differently in two editors. Two spaces per level, and `guml fmt` will convert a file that
already uses tabs."
            }

            Code::UnterminatedString => {
                "\
A quoted string ran to the end of the line without closing. The lexer recovers and keeps parsing
so the rest of the file is still checked, but the value it read is almost certainly not the one
intended."
            }

            Code::UnterminatedBrace => {
                "\
A `{` group — a binding, a type body or a mutation body — never closed. Braces are matched by
counting, so an extra `{` inside a string is the usual cause."
            }

            Code::UnexpectedChar => {
                "\
A character that cannot begin any token. The lexer skips it and continues, so this is often
accompanied by a second, more useful error on the same line."
            }

            Code::InconsistentDedent => {
                "\
Children of one parent are indented by different amounts. The parser's rule is that children are
the following lines with a *strictly greater* indent, applied recursively — so `4` then `5` is not
two ragged siblings, it is a parent and a child, which is very unlikely to be what was meant.
`guml fmt` makes the intent explicit either way."
            }

            Code::UnexpectedIndent => {
                "\
The first significant line of a file, or a line after a dedent, cannot be indented: there is no
element above it to be a child of."
            }

            Code::ExpectedTag => {
                "\
Every structural line begins with a tag or a directive. This most often means prose leaked into a
document — a model appending an explanation after the last line, which is exactly what the
`sanitize` layer of the repair pipeline strips."
            }

            Code::ExpectedValue => {
                "\
`name=` with nothing after it. Either give it a value, or drop the `=` and use the bare word if
it is a modifier."
            }

            Code::TrailingTokensAfterAction => {
                "\
`>` consumes the rest of its line by construction, so nothing can follow an action. If two things
should happen, separate the statements with `;` inside the action."
            }

            Code::BadExpression => {
                "\
The expression language is deliberately small: paths with aggregates, comparisons, arithmetic,
boolean operators, literals, and prefix `!`/`-`. No calls beyond the fixed aggregates, no
indexing, no lambdas.

This is not a limitation to work around. A GUML document may come from an untrusted agent, and
the fact that a binding cannot call anything is what makes rendering one safe. Anything more
belongs in a `js` block, where the boundary is visible."
            }

            Code::UnknownTag => {
                "\
The tag vocabulary is closed, and an unknown tag is an error rather than a passthrough. That is
the trade the whole design rests on: because the compiler knows every tag, it owns the classes,
the ARIA plumbing and the loading and error states, which is where the token saving comes from.
A tag that is merely misspelled comes with a suggestion `guml fix` can apply."
            }

            Code::UnknownModifier => {
                "\
Modifiers are semantic and closed — `primary`, `sm`, `center` — never utility classes. If the
intent is presentational, the answer is a theme pack rather than a new modifier."
            }

            Code::UnknownAttr => {
                "\
Each tag accepts the global attributes plus a few of its own. `guml registry --tags btn` lists
what a given tag takes."
            }

            Code::UnknownState => {
                "\
A binding or an action referred to a name that no `state` or `data` directive declares. Declare
it, or correct the spelling — the diagnostic names the closest match when there is one."
            }

            Code::DuplicateState => {
                "\
Two `state` directives with the same name. The second would silently win, so it is an error
instead."
            }

            Code::MissingPageDirective => {
                "\
Without `page <Name>` the compiler has no name for the component it emits. It falls back to a
placeholder, which is why this is a warning rather than an error."
            }

            Code::IconControlWithoutLabel => {
                "\
A control with no text has no accessible name, so a screen reader announces it as \"button\" and
nothing else. Add `aria=\"…\"`, or give it a visible label.

This is an error rather than a warning by design: an inaccessible interface is a broken
interface, and the compiler is the last place able to notice."
            }

            Code::InputWithoutLabel => {
                "\
A placeholder is not a label. It disappears the moment someone types, and it is not an accessible
name. This is a warning rather than an error only because the field is still operable — add
`aria=\"…\"` to fix it properly."
            }

            Code::UnknownMutation => {
                "\
An action named a mutation the resource does not declare. Mutations are the indented lines under
a `data` directive; the emitted code calls a function generated from them, so a name that does
not exist would not compile."
            }

            Code::UnknownTypeName => {
                "\
A resource's element type must be declared with `type`. Without it the emitted code is typed as
`unknown[]` and every field access on a row goes unchecked."
            }

            Code::UnknownBodyField => {
                "\
An *optimistic* mutation applies its body to a row locally before the server answers, so every
field in the body has to exist on the row type. A plain, non-optimistic body may carry anything —
a login sends a password that is obviously not a field of a session."
            }

            Code::AssignToNonState => {
                "\
Only a declared state name is assignable. Not a dotted path, not a resource, not a call.

That restriction is what keeps actions non-Turing-complete, and it is the security boundary for
rendering a document an untrusted agent produced. Change a resource through one of its mutations;
put anything else in a `js` block."
            }

            Code::TypeMismatch => {
                "\
The assigned value does not fit the state's type, inferred from its initial value — assigning a
string to a counter, or a value outside an enumerated state's domain. The emitted setter would be
called with the wrong type and `tsc` would reject it."
            }

            Code::DuplicateAnchor => {
                "\
Two elements share an `#id`. A link can only scroll to one of them, and the emitted HTML would be
invalid."
            }

            Code::DanglingAnchor => {
                "\
A link points at an id nothing on the page defines. It looks interactive and does nothing, which
is worse than an obvious omission."
            }

            Code::EmptyRepeater => {
                "\
A `list` or `table` with no indented children renders nothing at all — the fetch happens, the
rows arrive, and the page stays blank. The children are the template for one row."
            }

            Code::MultipleH1 => {
                "\
One `h1` names the page. Extra ones flatten the document outline that assistive technology uses
to navigate; `h2` or `h` are the section headings."
            }

            Code::UnusedState => {
                "\
A `state` nothing reads. It costs tokens in the source, emits a `useState` nothing consumes, and
usually means a binding was renamed and this declaration was left behind."
            }

            Code::UnusedResource => {
                "\
A `data` directive nothing renders. The fetch still happens on mount, so this is a real request
whose result is discarded."
            }

            Code::NotEnumerated => {
                "\
`tabs` and `select` build their options from the bound state's *domain*, declared as
`state name=first|second|third`. A state without a domain gives them nothing to render, so the
control appears empty.

This is also the language's answer to the `option` tag that models reach for: the members live on
the state, not in child elements."
            }

            Code::BadAttrValue => {
                "\
An attribute that takes a number was given something else — `cols=three`. The emitted code would
put the value straight into a class name or a DOM property."
            }

            Code::DuplicateAttr => {
                "\
The same attribute twice on one element. The last value wins, so one of them has no effect and
the intent is ambiguous."
            }

            Code::BadMethod => {
                "\
Not one of GET, POST, PUT, PATCH, DELETE, HEAD. An unrecognised word here was previously skipped
and the request silently became a GET, which is the kind of quiet mis-lowering the compiler is
not allowed to do."
            }

            Code::EscapeHatch => {
                "A `js` or `raw` block. This is not a defect and never fails a build — it is a *measurement*.

Every construct needs a way out, or the expressiveness cliff becomes an adoption wall. But the
rate at which these appear is the early warning that the vocabulary is too small, so the compiler
counts them and says so. If a benchmark task needs one, that task is evidence about the language
rather than about the model.

Two things to know. The code inside is emitted verbatim and is *not* checked, so nothing the rest
of the compiler guarantees — accessible names, rollback, types — applies inside the block. And the
browser runtime does not execute it at all: a document may come from an untrusted agent, so the
live preview renders a placeholder instead. The emitted file is the only place the code runs."
            }
            Code::AppLevelConstruct => {
                "GUML is one language with two conformance levels, the way CommonMark and GFM are.

`core` is markup: containers, text, controls, content blocks. No I/O, no state, no behaviour. A host
can render a core document that arrived from an untrusted agent, because there is nothing in it to
run.

`app` adds the framework layer — `data` resources, actions, mutations, `state`, repeaters that
iterate them — and an `app` document is not safe to render blindly, because it declares network
calls and mutations on the host's behalf.

You are compiling at the core level (`--core`, or a host that loaded a core registry), and this line
needs the app level. Either raise the level for this document, or express the same thing without
behaviour."
            }
            Code::BadRegistry => {
                "A registry document is a JSON object with a `components` array, each entry matching
the `ComponentDef` shape: `name`, `kind`, and optionally `level`, `attrs`, `a11y`, `doc`.

Three things are rejected rather than accepted quietly:

* **Shadowing a builtin.** A registry may add tags; it may not redefine `btn`. Otherwise the same
  document renders differently depending on which registry was loaded, with no diagnostic — exactly
  the failure a closed vocabulary exists to prevent.
* **An unusable name.** The lexer reads a tag as a bare lowercase word, so a name with a space or an
  uppercase letter could be registered and never matched.
* **An app-level entry in a core host.** It is skipped, not merged, so a registry cannot smuggle
  behaviour past a host that asked for markup only."
            }
            Code::DuplicateDef => {
                "A `def` adds a tag to the vocabulary, and the vocabulary is closed: two definitions of
the same name would mean the same document renders differently depending on which one the compiler
reached first.

Redefining a builtin is rejected for the same reason a loaded registry may not shadow one — a document
using `card` has to mean the same thing everywhere."
            }
            Code::DefArity => {
                "A `def` declares its parameters positionally, so a call supplies exactly that many
arguments.

Optional parameters are deliberately absent for now. They would need a defaulting rule, and a rule
about what a missing argument means inside the body — decisions worth making on purpose rather than
inheriting from an implementation detail."
            }
            Code::RecursiveDef => {
                "Expansion happens at compile time, so a `def` that reaches itself has no base case to
stop at: it is not deep recursion, it is unbounded.

The cycle is reported with the path that closes it. If you want a repeating structure, use `list` over a
resource — a repeater is bounded by its data."
            }
            Code::EmptyDef => {
                "A `def` with no body expands to nothing, so every call site silently disappears. That is
the shape of an invariant-3 violation: the author wrote something and the output does not contain it.

An indented body is required. If you want a deliberately empty slot, an `empty` element says so."
            }
            Code::DefParamUnsupported => {
                "A parameter is substituted where the compiler can see it as a value: a binding
positional (`h {label}`), an attribute value (`aria={name}`), and inside prose (`p Total: {value}`).

An action body is not one of those places. Actions are lowered to JavaScript, and substituting a
parameter into one would mean deciding whether the argument is a variable reference or a literal — a
question the call site does not answer. Rather than guess, this is rejected: put the action at the call
site, where its scope is unambiguous."
            }

            Code::BadEffect => {
                "\
`on` takes a trigger and an action: `on mount >tasks.list`, or `on {filter} >tasks.list`.

The trigger *is* the dependency, which is the whole reason this directive exists rather than a `js`
block containing a `useEffect`. A dependency array is a second list that has to agree with the first,
and it is wrong in two directions: a missing entry reads stale values, a spurious one loops forever.
Neither mistake is available here, because there is only one list to get right.

A resource already fetches on mount, so most pages need no `on mount` at all — it is for the cases
where something else has to happen as well. `on {expr}` re-runs whenever that value changes."
            }

            Code::BadUrl => {
                "\
A request needs a path starting with `/`, or an absolute `http` URL. A bare word is not a route
token, so it never reaches the resource and the emitted code would fetch the current page."
            }

            Code::DroppedPositional => {
                "\
A tag reads a fixed number of positional slots — a `btn` reads one label, a `tier` reads name, price and
blurb. Bare words past the last slot have nowhere to go.

    btn Add task primary          # two words, one label slot
    btn \"Add task\" primary        # one label, quoted

This is an error rather than a warning because of what the alternative was. `btn Add task primary`
compiled with **zero diagnostics** and emitted `<button>Add</button>`: the word `task` was deleted from
the output with no trace. That is the same data loss as the older `p Set x=1 to enable` bug, where an
`=` in prose silently discarded four words, and the same rule applies — the content floor is that prose
survives verbatim, and a rule that drops a word from it is a defect rather than compression.

Quoting is unambiguous, so the suggestion is mechanically applicable: `guml fix` repairs this with no
model call, which matters because an unquoted multi-word label is one of the most common things a
language model writes."
            }

            Code::RowMutationOutsideRepeater => {
                "\
A mutation whose path interpolates a field — `retry POST /api/jobs/{id}/retry` — can only run where
there is a row to take `{id}` from. That means inside the `list` or `table` rendering the item:

    table jobs
      text {name}
      btn Retry >jobs.retry        # `{id}` comes from this row

A toolbar button calling the same mutation has no row. The emitted callback was handed an empty object
where the row type was expected, so `tsc --strict` rejected the output, and at runtime the request would
have gone to `/api/jobs/undefined/retry`.

The check exists because the type error is a *consequence*: the document is what is wrong, and a
diagnostic about the document is the one an author or a repair loop can act on. Relying on the emitted
code failing to compile also assumes somebody runs that step, which is a weaker guarantee than the
compiler having an opinion."
            }

            Code::BadChild => {
                "\
A component's registry entry may declare what its children can be, and this document put something else
inside it — or left out a child the entry requires.

    select status                  option, and only option
      option Open
      option Closed

    stepper                        requires at least one `step`
      step Collect

The constraint lives in the registry rather than in the compiler, which is what makes it apply to
components the compiler did not ship: a third-party entry declares `children.allow` and gets the same
checking as `select`. `children.deny: [\"*\"]` marks a leaf that takes no children at all.

Widening a constraint is always safe; narrowing one can invalidate a document that compiles today, so
`spec/STABILITY.md` treats `allow` as extendable and `deny`/`require` as frozen once published."
            }

            Code::ModifierInProse => {
                "\
A text tag takes its whole line remainder as prose, verbatim. So a modifier written at the start of one
is not a modifier — it is the first word of the text:

    note danger Card declined.     renders \"danger Card declined.\"

That rule is frozen and this diagnostic does not change it, because the alternative is worse: if the
compiler quietly reclassified a leading word, `p center the label under the field` would lose a word from
prose, and prose surviving verbatim is a guarantee the whole content floor rests on.

What the warning is for is the case where a modifier is what the author *meant*. That was not
hypothetical: the registry's description of `badge` said \"use `danger`/`primary`/`quiet` for tone\", the
slate theme carried tone rules keyed on exactly those modifiers, and `badge danger Breaking` compiled
with no diagnostic and rendered the string \"danger Breaking\". Two thirds of the compiler documented a
feature the third could not deliver.

`badge` takes positionals now, so it accepts a modifier like every other non-text tag:

    badge Breaking danger
    badge \"Breaking change\" danger

For a genuine text tag, the answer is a different tag. Tone on a paragraph is `alert danger` with the
paragraph inside it; emphasis on a line of prose is not in the vocabulary, and inventing an attribute for
it here would be presentation leaking into the source."
            }

            Code::ResourceNotAList => {
                "A `data` resource is a collection. `data subscription:Subscription GET /api/subscription` — a single
object — parsed and validated, and then emitted `useState<Subscription[]>([])`, so every read of
`{subscription.plan}` was a property access on an array and `tsc --strict` rejected the output.

Everything the resource layer generates assumes rows: the empty state, the optimistic apply and rollback,
`.count`/`.sum`, the keyed `map`. A single object has none of that to do, and pretending otherwise is the
compiler accepting a shape it cannot emit.

Declare the list and take the first row in a `js` block, which keeps the fetch, the cache, the retry and
the error state and costs one counted escape hatch:

    data subscription:Subscription[] GET /api/subscription
    js
      const sub = subscription[0];
    metric {sub.plan}

A first-class single-object resource is a real gap and is tracked in `ROADMAP.md`. Reporting it is not the
fix; it is the guarantee that the gap is visible at compile time rather than in a type error somebody else
reads."
            }

            Code::RepeaterNeedsRowType => {
                "A repeater's source is normally a declared resource, which brings its row type with it. It may also be any
other array in scope — a `js` block's `const` — and then nothing can infer what a row is: the compiler does
not read a `js` body, so `{name}` inside the row template has no field list to resolve against.

`of=` is the document saying:

    js
      const matches = events.filter((e) => e.channel === channel && e.country === country);
    list matches of=Event
      text {name}
      note {country}

This exists because requiring a resource made **more than one client-side filter inexpressible**. `where=`
takes a single enumerated state, and a predicate over three states can only live in a `js` block — which
could compute the right numbers and could not feed the list. Two GUML-Bench reference answers had to filter
on the server and fail their own 'one fetch, not one per change' criterion because of it.

A derived source gets no fetch, no loading state and no error state, because there is no request: those
belong to `data`. It gets the row scope, the `empty` slot, `where=` and the keyed map."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_and_an_explanation() {
        for code in Code::ALL {
            assert!(!code.title().is_empty(), "{} has no title", code.id());
            let explanation = code.explain();
            assert!(
                explanation.len() > 60,
                "{} has a stub explanation ({} chars)",
                code.id(),
                explanation.len()
            );
        }
    }

    #[test]
    fn codes_can_be_looked_up_the_way_people_type_them() {
        assert_eq!(Code::from_id("GUML0064"), Some(Code::AssignToNonState));
        assert_eq!(Code::from_id("guml0064"), Some(Code::AssignToNonState));
        assert_eq!(Code::from_id("0064"), Some(Code::AssignToNonState));
        assert_eq!(Code::from_id("64"), Some(Code::AssignToNonState));
        assert_eq!(Code::from_id("GUML9999"), None);
        assert_eq!(Code::from_id("nonsense"), None);
    }

    #[test]
    fn the_list_holds_every_variant() {
        // A code missing from `ALL` would be invisible to `guml explain` and to the uniqueness
        // check. Comparing against the number of distinct ids catches an omission, because a
        // forgotten variant is simply absent rather than duplicated.
        let ids: std::collections::BTreeSet<&str> = Code::ALL.iter().map(|c| c.id()).collect();
        assert_eq!(ids.len(), Code::ALL.len(), "duplicate id in ALL");
        // 49 codes as of `GUML0104`; the assertion is a reminder to add a title and an explanation
        // when a code is added, not a cap. It has done its job every time it failed.
        assert_eq!(Code::ALL.len(), 49, "a code was added — give it a title and an explanation");
    }
}
