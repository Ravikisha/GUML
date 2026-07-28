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
use std::collections::{HashMap, HashSet};

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];

/// Attributes whose value must be a whole number.
const NUMERIC_ATTRS: &[&str] = &["cols", "open", "gap", "min", "max"];

pub fn validate(program: &Program, diags: &mut Diagnostics) {
    let types: HashMap<&str, HashSet<&str>> = program
        .types
        .iter()
        .map(|t| (t.name.as_str(), t.fields.iter().map(|f| f.name.as_str()).collect()))
        .collect();

    resources(program, &types, diags);
    let used = walk_tree(program, &types, diags);
    unused(program, &used, diags);
}

/* ------------------------------------------------------------------ resources */

fn resources(
    program: &Program,
    types: &HashMap<&str, HashSet<&str>>,
    diags: &mut Diagnostics,
) {
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
            if let Some(near) = nearest(item, &types.keys().map(|k| k.to_string()).collect::<Vec<_>>()) {
                d = d.with_suggestion(near);
            }
            diags.push(d);
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
                        if let Some(near) = nearest(field, &names) {
                            d = d.with_suggestion(near);
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
    if !url.starts_with('/') && !url.starts_with("http") {
        diags.push(
            Diagnostic::error(
                Code::BadUrl,
                format!("`{url}` is not a request path"),
                span,
            )
            .with_help("start it with `/` for a same-origin path, or `http` for an absolute URL"),
        );
    }
}

/* --------------------------------------------------------------------- tree */

#[derive(Default)]
struct Used {
    states: HashSet<String>,
    resources: HashSet<String>,
    anchors: Vec<(String, Span)>,
}

fn walk_tree(
    program: &Program,
    types: &HashMap<&str, HashSet<&str>>,
    diags: &mut Diagnostics,
) -> Used {
    let mut used = Used::default();
    let mut defined_anchors: HashMap<String, Span> = HashMap::new();
    let mut h1s: Vec<Span> = Vec::new();

    for el in &program.tree {
        element(el, program, types, None, &mut used, &mut defined_anchors, &mut h1s, diags);
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
            .with_help("add `#{anchor}` to the section it should scroll to".replace("{anchor}", anchor));
            let names: Vec<String> = defined_anchors.keys().cloned().collect();
            if let Some(near) = nearest(anchor, &names) {
                d = d.with_suggestion(format!("#{near}"));
            }
            diags.push(d);
        }
    }

    if h1s.len() > 1 {
        for span in h1s.iter().skip(1) {
            diags.push(
                Diagnostic::warning(
                    Code::MultipleH1,
                    "more than one `h1` on the page",
                    *span,
                )
                .with_help("one `h1` names the page; use `h2` or `h` for sections"),
            );
        }
    }

    used
}

#[allow(clippy::too_many_arguments)]
fn element(
    el: &Element,
    program: &Program,
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
    record_uses(el, used);
    actions(el, program, diags);
    enumerated(el, program, diags);

    let child_fields: Option<&HashSet<&str>> = if matches!(el.tag.as_str(), "list" | "table") {
        let source = el
            .positionals
            .iter()
            .find_map(|p| match p {
                Positional::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .or_else(|| el.attr("of").and_then(|v| v.as_text()));

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

        source
            .and_then(|name| program.resources.iter().find(|r| r.name == name))
            .and_then(|r| types.get(r.ty.trim_end_matches("[]")))
    } else {
        item_fields
    };

    for child in &el.children {
        element(child, program, types, child_fields, used, anchors, h1s, diags);
    }
}

/// Which declarations this element refers to, so unused ones can be reported later.
fn record_uses(el: &Element, used: &mut Used) {
    let mut note = |expr: &str| {
        let head = head_ident(expr);
        if !head.is_empty() {
            used.states.insert(head.to_string());
            used.resources.insert(head.to_string());
        }
    };

    for p in &el.positionals {
        match p {
            Positional::Binding(b) => note(b),
            // `input draft` and `list tasks` name a declaration in a positional slot.
            Positional::Text(t) => note(t),
            _ => {}
        }
    }
    for a in &el.attrs {
        if let Value::Binding(b) = &a.value {
            note(b);
        }
    }
    for text in el.content.iter().chain(el.text_lines.iter()) {
        for b in interpolations(text) {
            note(&b);
        }
    }
    for action in &el.actions {
        for part in action.split(';') {
            note(part.trim());
            // `>tasks.add{title:draft}` uses `draft` as well as `tasks`.
            if let Some(open) = part.find('{') {
                for pair in part[open + 1..].trim_end_matches('}').split(',') {
                    if let Some((_, v)) = pair.split_once(':') {
                        note(v.trim());
                    }
                }
            }
            if let Some((_, rhs)) = part.split_once('=') {
                note(rhs.trim());
            }
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

        if NUMERIC_ATTRS.contains(&a.name.as_str()) {
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
fn actions(el: &Element, program: &Program, diags: &mut Diagnostics) {
    for action in &el.actions {
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
                            el.span,
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
                            el.span,
                        )
                        .with_help("change a resource through one of its mutations"),
                    );
                } else if let Some(state) = program.state(target) {
                    check_assignment_type(state, stmt, el.span, diags);
                }
                continue;
            }

            // `tasks.add{...}` or `tasks.drop`
            let call = stmt.split('{').next().unwrap_or(stmt).trim();
            if let Some((head, rest)) = call.split_once('.') {
                if let Some(resource) = program.resources.iter().find(|r| r.name == head) {
                    let name = rest.trim();
                    if !name.is_empty() && !resource.mutations.iter().any(|m| m.name == name) {
                        let mut d = Diagnostic::error(
                            Code::UnknownMutation,
                            format!("`{head}` has no mutation `{name}`"),
                            el.span,
                        )
                        .with_help("declare it as an indented line under the `data` directive");
                        let names: Vec<String> =
                            resource.mutations.iter().map(|m| m.name.clone()).collect();
                        if let Some(near) = nearest(name, &names) {
                            d = d.with_suggestion(near);
                        }
                        diags.push(d);
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

/// `tabs`, `select` and `where=` all need an enumerated state; without a domain there is
/// nothing to build options from and the control renders empty.
fn enumerated(el: &Element, program: &Program, diags: &mut Diagnostics) {
    let bound = match el.tag.as_str() {
        "tabs" | "select" => el
            .positionals
            .iter()
            .find_map(|p| match p {
                Positional::Text(t) => Some(t.clone()),
                Positional::Binding(b) => Some(head_ident(b).to_string()),
                _ => None,
            })
            .or_else(|| el.attr("bind").and_then(|v| v.as_text()).map(str::to_string)),
        _ => None,
    };

    if let Some(name) = bound {
        match program.state(&name) {
            Some(state) if state.domain.is_empty() => diags.push(
                Diagnostic::error(
                    Code::NotEnumerated,
                    format!("`{}` needs an enumerated state; `{name}` has no domain", el.tag),
                    el.span,
                )
                .with_help("declare it as `state name=first|second|third`"),
            ),
            _ => {}
        }
    }

    if let Some(Value::Binding(b)) = el.attr("where") {
        let head = head_ident(b);
        if let Some(state) = program.state(head) {
            if state.domain.is_empty() {
                diags.push(
                    Diagnostic::warning(
                        Code::NotEnumerated,
                        format!("`where={{{head}}}` filters by a state with no domain"),
                        el.span,
                    )
                    .with_help("an enumerated state tells the compiler which filters exist"),
                );
            }
        }
    }
}

fn unused(program: &Program, used: &Used, diags: &mut Diagnostics) {
    for state in &program.states {
        if !used.states.contains(&state.name) {
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
        if !used.resources.contains(&r.name) {
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

/// `tasks.open.count` → `tasks`; `!draft.trim()` → `draft`.
fn head_ident(expr: &str) -> &str {
    let expr = expr.trim().trim_start_matches(['!', '(']);
    let end = expr
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(expr.len());
    &expr[..end]
}

fn interpolations(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                out.push(after[..close].to_string());
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    out
}
