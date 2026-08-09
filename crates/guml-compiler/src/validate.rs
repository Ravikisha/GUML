//! Static validation: everything a parse cannot tell you.
//!
//! The parser answers "is this GUML". This answers "does this GUML mean anything" —
//! whether the names resolve, whether the types line up, whether the structure could
//! actually render. Both matter, but for different readers: a human wants the first, and
//! the repair loop needs the second, because a document that parses and then renders an
//! empty list is a failure the model cannot see.
//!
//! # Rules
//!
//! Everything here reports through a stable code, in one pass with the parser's own
//! diagnostics (invariant 1), and every code is append-only (invariant 2).
//!
//! Severity is chosen by what the compiler can recover from:
//!
//! - **error** — the emitted code would be wrong or would not compile. An unknown mutation
//!   generates a call to a function that does not exist.
//! - **warning** — the output is valid but the author's intent is not achieved. An unused
//!   state is dead weight; an empty container renders nothing.
//!
//! The line between them is deliberately conservative. A false error blocks a working
//! document, and in a generation loop that costs a round for nothing.

use crate::sema::nearest;
use guml_ast::{Element, Positional, Program, Value};
use guml_diagnostics::{Code, Diagnostic, Diagnostics, Span};
use guml_registry::Registry;
use std::collections::{HashMap, HashSet};

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];

/// Attributes whose value must be a whole number.
const NUMERIC_ATTRS: &[&str] = &["cols", "open", "gap", "min", "max"];

/// Whether `attr` is numeric *on this tag*.
///
/// One exception, and it earns the extra function: `cols` is a grid column *count* everywhere except a
/// repeater, where it lists the column *headers* — `table invoices cols="Client, Amount, Due"`. A grid's
/// columns are a number the compiler generates; a table's are names only the author knows.
///
/// Per-tag rather than a second attribute name, because the surface stays smaller and the meaning is the
/// same question ("what are this thing's columns") answered in the two ways the two things allow.
fn numeric_on(tag: &str, attr: &str) -> bool {
    if attr == "cols" && matches!(tag, "list" | "table") {
        return false;
    }
    NUMERIC_ATTRS.contains(&attr)
}

pub fn validate(program: &Program, reg: &Registry, diags: &mut Diagnostics) {
    let types: HashMap<&str, HashSet<&str>> = program
        .types
        .iter()
        .map(|t| (t.name.as_str(), t.fields.iter().map(|f| f.name.as_str()).collect()))
        .collect();

    resources(program, &types, diags);
    walk_tree(program, reg, &types, diags);
    effects(program, diags);
    unused(program, diags);
}

/// Declared effects, through the same action rules an element's `>` goes through.
///
/// `on mount >tasks.remove` has to be the same error as `btn Go >tasks.remove`. Two sets of rules for
/// one action language would mean the stricter one is the real one and the other is a hole.
fn effects(program: &Program, diags: &mut Diagnostics) {
    for e in &program.effects {
        actions_in(&e.actions, e.span, program, diags);
    }
}

/* ------------------------------------------------------------------ resources */

fn resources(program: &Program, types: &HashMap<&str, HashSet<&str>>, diags: &mut Diagnostics) {
    for r in &program.resources {
        let item = r.ty.trim_end_matches("[]");
        // A resource whose type is undeclared emits `unknown[]`, and every field access on
        // its rows is then unchecked — so this is where the type story starts.
        if !item.is_empty() && !types.contains_key(item) {
            let mut d = Diagnostic::error(
                Code::UnknownTypeName,
                format!("`{}` is not a declared type", item),
                r.span,
            )
            .with_help("declare it with `type Name {field, other:bool}`");
            let names: Vec<String> = types.keys().map(|k| k.to_string()).collect();
            if let Some(hint) = did_you_mean(item, &names) {
                d = d.with_help(hint);
            }
            diags.push(d);
        }

        // A resource is a collection, and every part of what `data` generates assumes so: the empty
        // state, the optimistic apply and rollback, `.count`/`.sum`, the keyed `map`. A single-object
        // type was accepted anyway and emitted `useState<Type[]>([])`, so `{subscription.plan}` read a
        // property off an array and only `tsc --strict` over the output objected. Reported at compile
        // time instead, in one place, for every backend.
        if !item.is_empty() && !r.ty.ends_with("[]") {
            diags.push(
                Diagnostic::error(
                    Code::ResourceNotAList,
                    format!("`{}` is a single `{item}`, and a resource is a collection", r.name),
                    r.span,
                )
                .with_suggestion(format!("{}:{item}[]", r.name))
                .with_help(
                    "declare the list and take the first row in a `js` block if the endpoint really \
                     returns one object — that keeps the fetch, the cache and the error state",
                ),
            );
        }

        check_method(&r.method, r.span, diags);
        check_url(&r.url, r.span, diags);

        for m in &r.mutations {
            check_method(&m.method, m.span, diags);
            check_url(&m.url, m.span, diags);

            // Only checked for *optimistic* mutations. A plain request body legitimately
            // carries fields the resource does not have — a login sends a password that is
            // obviously not a field of `Session`. But an optimistic mutation is different:
            // the compiler splices the body into a row locally, so a field the row does not
            // have produces a row that renders wrong until the refetch corrects it.
            //
            // The first version of this rule flagged `e2-signin.guml`, one of the examples
            // the model learns from. The example was fine; the rule was too broad.
            if let (Some(fields), true) = (types.get(item), m.optimistic.is_some()) {
                for field in &m.body {
                    if !fields.contains(field.as_str()) {
                        let mut d = Diagnostic::error(
                            Code::UnknownBodyField,
                            format!("`{}` is not a field of `{}`", field, item),
                            m.span,
                        )
                        .with_help(
                            "an optimistic mutation applies its body to a row locally, so every                              field must exist on the row type",
                        );
                        let names: Vec<String> = fields.iter().map(|f| f.to_string()).collect();
                        if let Some(hint) = did_you_mean(field, &names) {
                            d = d.with_help(hint);
                        }
                        diags.push(d);
                    }
                }
            }
        }
    }
}

fn check_method(method: &str, span: Span, diags: &mut Diagnostics) {
    if !method.is_empty() && !METHODS.contains(&method) {
        diags.push(
            Diagnostic::error(Code::BadMethod, format!("`{method}` is not an HTTP method"), span)
                .with_help(format!("one of: {}", METHODS.join(", "))),
        );
    }
}

fn check_url(url: &str, span: Span, diags: &mut Diagnostics) {
    // Empty is not "absent": the lexer only produces a route token for something starting
    // with `/`, so `data rows:T[] GET api/rows` leaves the URL empty and the emitted code
    // would fetch the current page. Silence here would be a silent mis-lowering.
    if url.is_empty() {
        diags.push(
            Diagnostic::error(Code::BadUrl, "this request has no path", span)
                .with_help("add a path starting with `/`, for example `GET /api/rows`"),
        );
        return;
    }
    // A protocol-relative URL. `//evil.com/x` *looks* like a path and is not one: it inherits the page's
    // scheme and goes to another origin entirely.
    //
    // This was reachable and silent. Before the lexer learned `https://`, any `scheme://host/path` lost
    // its scheme and arrived here as `//host/path` — so `javascript://x/y` compiled clean and emitted a
    // cross-origin fetch. It is now two separate guards: the lexer only tokenises `http`/`https`, and this
    // refuses what is left, because a document that means "a path" should not be one character away from
    // meaning "somebody else's server".
    if url.starts_with("//") {
        diags.push(
            Diagnostic::error(
                Code::BadUrl,
                format!("`{url}` is protocol-relative, so it points at another origin"),
                span,
            )
            .with_help(
                "write `/path` for a same-origin request, or `https://host/path` to be explicit about the origin",
            ),
        );
        return;
    }
    if !url.starts_with('/') && !url.starts_with("http") {
        diags.push(
            Diagnostic::error(Code::BadUrl, format!("`{url}` is not a request path"), span)
                .with_help(
                    "start it with `/` for a same-origin path, or `http` for an absolute URL",
                ),
        );
    }
}

/* --------------------------------------------------------------------- tree */

#[derive(Default)]
struct Used {
    /// Anchor references, kept here rather than in the shared walker because a duplicate-anchor
    /// diagnostic needs the span of each *occurrence*, not just the set of names.
    anchors: Vec<(String, Span)>,
}

fn walk_tree(
    program: &Program,
    reg: &Registry,
    types: &HashMap<&str, HashSet<&str>>,
    diags: &mut Diagnostics,
) {
    let mut used = Used::default();
    let mut defined_anchors: HashMap<String, Span> = HashMap::new();
    let mut h1s: Vec<Span> = Vec::new();

    for el in &program.tree {
        element(el, program, reg, types, None, &mut used, &mut defined_anchors, &mut h1s, diags);
    }

    // `link Pricing #pricing` pointing at nothing is a dead control: it looks interactive
    // and does nothing, which is worse than an obvious omission.
    for (anchor, span) in &used.anchors {
        if !defined_anchors.contains_key(anchor) {
            let mut d = Diagnostic::error(
                Code::DanglingAnchor,
                format!("nothing on this page has the id `{anchor}`"),
                *span,
            )
            .with_help(
                "add `#{anchor}` to the section it should scroll to".replace("{anchor}", anchor),
            );
            let names: Vec<String> = defined_anchors.keys().cloned().collect();
            if let Some(near) = nearest(anchor, &names) {
                d = d.with_help(format!("did you mean `#{near}`?"));
            }
            diags.push(d);
        }
    }

    if h1s.len() > 1 {
        for span in h1s.iter().skip(1) {
            diags.push(
                Diagnostic::warning(Code::MultipleH1, "more than one `h1` on the page", *span)
                    .with_help("one `h1` names the page; use `h2` or `h` for sections"),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn element(
    el: &Element,
    program: &Program,
    reg: &Registry,
    types: &HashMap<&str, HashSet<&str>>,
    // Fields of the row type when inside a repeater; `None` outside one.
    item_fields: Option<&HashSet<&str>>,
    used: &mut Used,
    anchors: &mut HashMap<String, Span>,
    h1s: &mut Vec<Span>,
    diags: &mut Diagnostics,
) {
    if el.tag == "h1" {
        h1s.push(el.span);
    }

    // An `#id` on a control is a *reference*; on anything else it is a *definition*. The
    // first version inserted it as a definition and then removed it for links, which meant
    // `link Work #work` deleted the definition that `section #work` had just made — so a
    // correct page reported both a duplicate and a dangling anchor. Deciding by tag once is
    // the whole fix.
    let references_anchor = matches!(el.tag.as_str(), "link" | "btn");
    if let Some(a) = el.anchor() {
        if references_anchor {
            used.anchors.push((a.to_string(), el.span));
        } else if anchors.insert(a.to_string(), el.span).is_some() {
            diags.push(
                Diagnostic::error(
                    Code::DuplicateAnchor,
                    format!("`#{a}` is used more than once"),
                    el.span,
                )
                .with_help("ids must be unique: a link can only scroll to one of them"),
            );
        }
    }
    // `/#features` as a route is also a reference.
    for p in &el.positionals {
        if let Positional::Route(r) = p {
            if let Some(rest) = r.strip_prefix('#') {
                used.anchors.push((rest.to_string(), el.span));
            }
        }
    }

    attributes(el, diags);
    expressions(el, diags);
    actions(el, program, item_fields.is_some(), diags);
    enumerated(el, program, diags);

    let child_fields: Option<&HashSet<&str>> = if matches!(el.tag.as_str(), "list" | "table") {
        // A repeater over something that is not a resource has to say what its rows are, because nothing
        // else can: a `js` block's array has no declared element type and the compiler does not read the
        // block. Without `of=` the old behaviour was a backend warning and an empty list.
        let named = el.positionals.iter().find_map(|p| match p {
            Positional::Text(t) => Some(t.as_str()),
            _ => None,
        });
        // Only for a tag the active registry *has*. At the `core` level `list` is not in the vocabulary at
        // all, so the parser already reported `GUML0030`; adding "give it an `of=` row type" on top would
        // send a repair loop to fix the row type of a tag it cannot use. Enforcement by absence, and one
        // diagnostic for one problem.
        if let (Some(name), true) = (named, reg.get(&el.tag).is_some()) {
            let is_resource = program.resources.iter().any(|r| r.name == name);
            if !is_resource && el.attr("of").is_none() {
                diags.push(
                    Diagnostic::error(
                        Code::RepeaterNeedsRowType,
                        format!(
                            "`{}` iterates `{name}`, which is not a resource, so its row type is unknown",
                            el.tag
                        ),
                        el.span,
                    )
                    .with_help(
                        "add `of=Type` naming the declared type of one row — that is how a `js`-computed \
                         array becomes iterable",
                    ),
                );
            } else if !is_resource {
                // `of=` present but naming nothing declared: the row scope would be empty and every field
                // read inside would be `GUML0033`, which points at the wrong line.
                let ty = el.attr("of").and_then(|v| v.as_text()).unwrap_or_default();
                if !types.contains_key(ty) {
                    let mut d = Diagnostic::error(
                        Code::UnknownTypeName,
                        format!("`of={ty}` is not a declared type"),
                        el.span,
                    )
                    .with_help("declare it with `type Name {field, other:bool}`");
                    let names: Vec<String> = types.keys().map(|k| k.to_string()).collect();
                    if let Some(hint) = did_you_mean(ty, &names) {
                        d = d.with_help(hint);
                    }
                    diags.push(d);
                }
            }
        }

        if el.children.is_empty() && el.text_lines.is_empty() {
            diags.push(
                Diagnostic::warning(
                    Code::EmptyRepeater,
                    format!("`{}` has no item template, so it renders nothing", el.tag),
                    el.span,
                )
                .with_help("indent the children that should render once per row"),
            );
        }

        // A derived array carries its row type in `of=`, so the field set comes from there when the source
        // is not a resource. Same shared answer as the resolver and the type checker.
        program.repeater_rows(el).and_then(|rows| types.get(rows.ty.as_str()))
    } else {
        item_fields
    };

    for child in &el.children {
        element(child, program, reg, types, child_fields, used, anchors, h1s, diags);
    }
}

/// Every binding on this element goes through the real expression parser.
///
/// Before this, syntax the grammar does not cover — `{a ? b : c}`, `{fetch(url)}` — was passed
/// through into emitted JavaScript, where it either failed to compile or, worse, ran. Reporting
/// it is also the security boundary: actions and bindings are deliberately not
/// Turing-complete, and "not covered by the grammar" has to mean "rejected", not "forwarded".
fn expressions(el: &Element, diags: &mut Diagnostics) {
    if el.is_escape() {
        return;
    }
    // Already parsed, so this only reports. Parsing again here is what the `Binding` type was
    // introduced to stop.
    for p in &el.positionals {
        if let Positional::Binding(b) = p {
            guml_syntax::expr::report_unknown(&b.expr, el.span, diags);
        }
    }
    for a in &el.attrs {
        if let Value::Binding(b) = &a.value {
            guml_syntax::expr::report_unknown(&b.expr, a.span, diags);
        }
    }
    // Prose interpolations are expressions too: `head Total {a ? b : c}` reaches the same
    // lowering as an attribute binding.
    for text in el.content.iter().chain(el.text_lines.iter()) {
        for b in interpolations(text) {
            guml_syntax::expr::parse_reported(b, el.span, diags);
        }
    }
}

fn attributes(el: &Element, diags: &mut Diagnostics) {
    let mut seen: HashMap<&str, Span> = HashMap::new();
    for a in &el.attrs {
        if seen.insert(a.name.as_str(), a.span).is_some() {
            diags.push(
                Diagnostic::warning(
                    Code::DuplicateAttr,
                    format!("`{}` is set twice; the last value wins", a.name),
                    a.span,
                )
                .with_help("remove one of them so the intent is unambiguous"),
            );
        }

        if numeric_on(&el.tag, &a.name) {
            let numeric = match &a.value {
                Value::Num(_) | Value::Binding(_) => true,
                Value::Word(w) | Value::Str(w) => w.parse::<f64>().is_ok(),
                _ => false,
            };
            if !numeric {
                diags.push(
                    Diagnostic::error(
                        Code::BadAttrValue,
                        format!("`{}` takes a number", a.name),
                        a.span,
                    )
                    .with_help("for example `cols=3`"),
                );
            }
        }
    }
}

/// Actions: assignment targets must be assignable, and a mutation must exist.
///
/// The two shapes are told apart by the assignment operators, not by the dot. `>n.length=3`
/// has a dot *and* an `=`; reading the dot first sent it down the mutation path, where it was
/// silently ignored. So `=`, `++` and `--` decide, and everything else is a call.
fn actions(el: &Element, program: &Program, in_row: bool, diags: &mut Diagnostics) {
    actions_in_row(&el.actions, el.span, program, in_row, diags);
}

/// The action rules, over a span rather than an element — so a declared effect uses exactly these.
///
/// A declared effect is never inside a repeater row, so it passes `in_row: false`.
fn actions_in(actions: &[String], span: Span, program: &Program, diags: &mut Diagnostics) {
    actions_in_row(actions, span, program, false, diags);
}

fn actions_in_row(
    actions: &[String],
    span: Span,
    program: &Program,
    in_row: bool,
    diags: &mut Diagnostics,
) {
    for action in actions {
        for stmt in action.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            let assigns = stmt.ends_with("++")
                || stmt.ends_with("--")
                || (stmt.contains('=') && !stmt.contains("==") && !stmt.contains("!="));

            if assigns {
                let target = stmt
                    .split_once('=')
                    .map(|(lhs, _)| lhs.trim())
                    .unwrap_or_else(|| stmt.trim_end_matches(['+', '-']))
                    .trim();

                // Only a declared state name is assignable. That boundary is what keeps
                // actions non-Turing-complete, which is also the security boundary for
                // rendering a document an untrusted agent produced.
                if target.contains('.') || target.contains('(') {
                    diags.push(
                        Diagnostic::error(
                            Code::AssignToNonState,
                            format!("`{target}` is not assignable"),
                            span,
                        )
                        .with_help(
                            "only a declared state name can be assigned; anything else belongs in a `js` block",
                        ),
                    );
                } else if program.resources.iter().any(|r| r.name == target) {
                    diags.push(
                        Diagnostic::error(
                            Code::AssignToNonState,
                            format!("`{target}` is a resource, not a state"),
                            span,
                        )
                        .with_help("change a resource through one of its mutations"),
                    );
                } else if let Some(state) = program.state(target) {
                    check_assignment_type(state, stmt, span, diags);
                }
                continue;
            }

            // `tasks.add{...}` or `tasks.drop`
            let call = stmt.split('{').next().unwrap_or(stmt).trim();
            if let Some((head, rest)) = call.split_once('.') {
                if let Some(resource) = program.resources.iter().find(|r| r.name == head) {
                    let name = rest.trim();
                    // `list` is the resource's own GET, which every `data` declaration has and none
                    // declares. Without it there is no way to say "fetch this again" — needed by a
                    // Reload button and by `on {filter} >tasks.list`, and the JSON IR has always
                    // called the GET `list`, so the name is not new vocabulary.
                    let implicit = name == "list";
                    if !name.is_empty()
                        && !implicit
                        && !resource.mutations.iter().any(|m| m.name == name)
                    {
                        let mut d = Diagnostic::error(
                            Code::UnknownMutation,
                            format!("`{head}` has no mutation `{name}`"),
                            span,
                        )
                        .with_help("declare it as an indented line under the `data` directive");
                        let names: Vec<String> =
                            resource.mutations.iter().map(|m| m.name.clone()).collect();
                        if let Some(hint) = did_you_mean(name, &names) {
                            d = d.with_help(hint);
                        }
                        diags.push(d);
                    }

                    // A mutation whose URL interpolates a row field needs a row.
                    //
                    // `retry POST /api/jobs/{id}/retry` called from a toolbar button emitted
                    // `jobsRetry({})` — an empty object where the row type was expected. `tsc` rejects
                    // it, which means the *only* thing standing between this and a shipped build was a
                    // check nobody runs on a Windows machine. At runtime it would have requested
                    // `/api/jobs/undefined/retry`.
                    //
                    // Reported here rather than left to the type checker because the emitted code is a
                    // consequence, not the contract: the document is what is wrong.
                    if let Some(m) = resource.mutations.iter().find(|m| m.name == name)
                        && m.url.contains('{')
                        && !in_row
                    {
                        diags.push(
                            Diagnostic::error(
                                Code::RowMutationOutsideRepeater,
                                format!(
                                    "`{head}.{name}` needs a row: its path `{}` interpolates a field of the item",
                                    m.url
                                ),
                                span,
                            )
                            .with_help(
                                "move the control inside the `list` or `table` that renders the item, where the row is in scope",
                            ),
                        );
                    }
                }
            }
        }
    }
}

/// `state count=0` then `>count=""` would emit `setCount("")` into a number.
fn check_assignment_type(
    state: &guml_ast::StateDecl,
    stmt: &str,
    span: Span,
    diags: &mut Diagnostics,
) {
    let Some((_, rhs)) = stmt.split_once('=') else { return };
    let rhs = rhs.trim();
    if rhs.is_empty() || rhs.contains(['{', '(']) {
        return;
    }

    let assigned_string = rhs.starts_with('"');
    let assigned_number = rhs.parse::<f64>().is_ok();
    let assigned_bool = matches!(rhs, "true" | "false" | "!true" | "!false");

    // Only a *literal* right-hand side can be judged by its spelling, which is all this check does.
    //
    // A bare name or a path is a value the check cannot see the type of, and treating "does not look
    // like a string literal" as "is not a string" produced a false error on the one construct that needs
    // it: `>editing = id` copies a row's id into a string state, which is how a per-row dialog remembers
    // which row it belongs to. `GUML0065` fired three times on a correct line and there was no other way
    // to write it, because the row's field is not a literal and never can be.
    //
    // An enumerated state is exempt from the exemption: its domain is a closed set of words, `>filter=opne`
    // is a bare word, and catching that typo is the whole point of the arm below.
    let assigned_literal = assigned_string || assigned_number || assigned_bool;
    if !assigned_literal && !rhs.starts_with('!') && state.domain.is_empty() {
        return;
    }

    let (expected, ok) = match &state.init {
        Value::Num(_) => ("a number", assigned_number),
        Value::Str(_) => ("a string", assigned_string),
        Value::Bool(_) => ("a boolean", assigned_bool || rhs.starts_with('!')),
        // An enumerated state may only take a member of its domain.
        Value::Word(_) if !state.domain.is_empty() => {
            let bare = rhs.trim_matches('"');
            ("a member of its domain", state.domain.iter().any(|d| d == bare))
        }
        _ => ("", true),
    };

    if !ok {
        let mut d = Diagnostic::error(
            Code::TypeMismatch,
            format!("`{}` holds {expected}", state.name),
            span,
        );
        if !state.domain.is_empty() {
            d = d.with_help(format!("one of: {}", state.domain.join(", ")));
        }
        diags.push(d);
    }
}

/// `tabs`, `select` and `where=` all need somewhere to get their choices from; with none, the
/// control renders empty.
///
/// # Two spellings, and this used to accept only one
///
/// The choices may be written in either of two places, and both are ordinary GUML:
///
/// ```text
/// state c: a          state c: a
///   domain: a, b      select c
/// select c              option a
///                       option b
/// ```
///
/// This checked *only* the state's domain and never looked at the `option` children — while
/// `guml_codegen::select_options` reconciles exactly those two sources and has done for a while. So
/// two halves of one compiler disagreed about where a `select`'s options come from: codegen accepted
/// either, validation accepted one, and the second form was rejected with "has no domain" while the
/// options sat directly beneath it.
///
/// That is the invariant-8 bug class — two copies of one rule drifting — in a seam invariant 8 does
/// not cover, because it is sema against codegen rather than backend against backend. It was also the
/// **single most common reason model-generated GUML failed to compile**: `bench/gen` recorded it as
/// the first error in two of six applications, unchanged between an 8B and a 70B model.
///
/// The fix is to ask the same function codegen asks, so there is one answer rather than two.
fn enumerated(el: &Element, program: &Program, diags: &mut Diagnostics) {
    let bound = match el.tag.as_str() {
        "tabs" | "select" => el
            .positionals
            .iter()
            .find_map(|p| match p {
                Positional::Text(t) => Some(t.clone()),
                Positional::Binding(b) => b.head_ident().map(str::to_string),
                _ => None,
            })
            .or_else(|| el.attr("bind").and_then(|v| v.as_text()).map(str::to_string)),
        _ => None,
    };

    if let Some(name) = bound {
        // The same reconciliation codegen performs: `option` children first, then the bound state's
        // domain. Asking the one function means the two can no longer disagree.
        let has_options = !guml_codegen::select_options(program, el).is_empty();

        match program.state(&name) {
            Some(state) if state.domain.is_empty() && !has_options => diags.push(
                Diagnostic::error(
                    Code::NotEnumerated,
                    format!("`{}` has no options; `{name}` has no domain", el.tag),
                    el.span,
                )
                // Both spellings, because both work and an author who reached for one should not be
                // told to use the other.
                .with_help(
                    "give the state a domain (`state name=first|second|third`), \
                     or write `option` children under this element",
                ),
            ),
            _ => {}
        }
    }

    if let Some(Value::Binding(b)) = el.attr("where") {
        // From the parsed tree rather than a substring scan.
        let head = b.head_ident().unwrap_or("");
        if let Some(state) = program.state(head) {
            // A domain-less *string* state is a free-text search, not a missing domain — `input query`
            // plus `table contacts where={query}` is a searchable table, and the backends lower it to a
            // substring match over the row's string fields. `guml_codegen::search_fields` is the one
            // place that decides, so this warning and that lowering cannot disagree: if it finds fields,
            // the filter is real and there is nothing to warn about.
            let searchable = el
                .positionals
                .iter()
                .find_map(|p| match p {
                    Positional::Text(t) => Some(t.as_str()),
                    _ => None,
                })
                .map(|source| !guml_codegen::search_fields(program, source, head).is_empty())
                .unwrap_or(false);
            if state.domain.is_empty() && !searchable {
                diags.push(
                    Diagnostic::warning(
                        Code::NotEnumerated,
                        format!("`where={{{head}}}` filters by a state with no domain"),
                        el.span,
                    )
                    .with_help(
                        "an enumerated state tells the compiler which filters exist; a string state \
                         searches the row's text fields, but only if the row type declares some",
                    ),
                );
            }
        }
    }
}

fn unused(program: &Program, diags: &mut Diagnostics) {
    // One shared answer for "what does this document refer to", so codegen cannot elide a
    // declaration this pass considers live. See `guml_ast::referenced_names`.
    let referenced = guml_ast::referenced_names(program);
    for state in &program.states {
        if !referenced.contains(&state.name) {
            diags.push(
                Diagnostic::warning(
                    Code::UnusedState,
                    format!("`{}` is declared but never used", state.name),
                    state.span,
                )
                .with_help("remove it, or bind it to a control — unused state costs tokens and reads as an oversight"),
            );
        }
    }
    for r in &program.resources {
        if !referenced.contains(&r.name) {
            diags.push(
                Diagnostic::warning(
                    Code::UnusedResource,
                    format!("`{}` is fetched but never rendered", r.name),
                    r.span,
                )
                .with_help("render it with `list` or `table`, or remove the `data` directive"),
            );
        }
    }
}

/* ------------------------------------------------------------------ helpers */

use guml_syntax::expr::interpolations;

/// A near-miss name, phrased for the `help` line.
///
/// Deliberately *not* `with_suggestion`. That field is a machine-applicable replacement for
/// the diagnostic's span, and every span in this module covers a whole line — the resource
/// declaration, the mutation, the element. Attaching a bare name to a line span means
/// `guml fix` replaces `check {done} aria="done" >rows.sve` with the single word `save`,
/// which is how an autofix destroys a document. Until the AST carries token spans inside
/// actions and directives, the name goes in prose where a human applies it.
fn did_you_mean(unknown: &str, candidates: &[String]) -> Option<String> {
    nearest(unknown, candidates).map(|near| format!("did you mean `{near}`?"))
}
