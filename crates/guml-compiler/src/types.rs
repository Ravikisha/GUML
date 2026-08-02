//! Type inference over expressions.
//!
//! # What this can and cannot know
//!
//! GUML declares no types for state — they are inferred from the initial value, which is what
//! makes `state count=0` one line instead of three. So inference here is *local and partial*: it
//! knows a state's type, a type declaration's field types, and the result of each operator. It
//! does not know the type of a field on an undeclared type, or of anything reached through a
//! resource without a `type` directive.
//!
//! [`Type::Unknown`] is therefore a first-class answer and never an error. A checker that guessed
//! instead would report type errors on correct documents, and in a generation loop a false error
//! costs a whole model round to "fix" something that was already right.
//!
//! # What it catches
//!
//! Mismatches the emitted code would carry into `tsc`, or worse, past it:
//!
//! - `{count + "x"}` — arithmetic on a string
//! - `{done + 1}` — arithmetic on a boolean
//! - `{name > 5}` — ordering a string against a number
//! - `disabled={draft}` where `draft` is a string — a truthiness bug rather than a type error in
//!   JavaScript, which is exactly why the compiler should say something

use guml_ast::{Element, Positional, Program, Value};
use guml_diagnostics::{Code, Diagnostic, Diagnostics, Span};
use guml_syntax::expr::{Aggregate, BinOp, Expr, Step};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Num,
    Str,
    Bool,
    /// An array of rows. Element fields are looked up through the resource's declared type.
    List,
    /// Not knowable from what the document declares. Never an error.
    Unknown,
}

impl Type {
    fn name(self) -> &'static str {
        match self {
            Type::Num => "a number",
            Type::Str => "a string",
            Type::Bool => "a boolean",
            Type::List => "a list",
            Type::Unknown => "an unknown type",
        }
    }

    fn known(self) -> bool {
        self != Type::Unknown
    }
}

/// Everything the checker knows about names in scope.
pub struct Scope<'a> {
    states: HashMap<&'a str, Type>,
    /// Resource name → its element type's field types.
    resources: HashMap<&'a str, HashMap<&'a str, Type>>,
    /// Fields of the row type, when inside a repeater.
    item: Option<HashMap<&'a str, Type>>,
}

fn field_type(declared: &str) -> Type {
    // `type Task {id, title, done:bool, n:number}` — a field with no annotation is a string,
    // which is the same default the code generator uses.
    match declared {
        "bool" | "boolean" => Type::Bool,
        "number" | "int" | "float" => Type::Num,
        _ => Type::Str,
    }
}

fn state_type(init: &Value, domain: &[String]) -> Type {
    if !domain.is_empty() {
        // An enumerated state holds one of its members, all of which are strings.
        return Type::Str;
    }
    match init {
        Value::Num(_) => Type::Num,
        Value::Str(_) => Type::Str,
        Value::Bool(_) | Value::Flag => Type::Bool,
        Value::Word(_) => Type::Str,
        // `state x={y}` — whatever `y` is, and this is not the place to chase it.
        Value::Binding(_) => Type::Unknown,
    }
}

impl<'a> Scope<'a> {
    pub fn new(program: &'a Program) -> Self {
        let types: HashMap<&str, HashMap<&str, Type>> = program
            .types
            .iter()
            .map(|t| {
                let fields =
                    t.fields.iter().map(|f| (f.name.as_str(), field_type(&f.ty))).collect();
                (t.name.as_str(), fields)
            })
            .collect();

        let states = program
            .states
            .iter()
            .map(|s| (s.name.as_str(), state_type(&s.init, &s.domain)))
            .collect();

        let resources = program
            .resources
            .iter()
            .filter_map(|r| {
                let item = r.ty.trim_end_matches("[]");
                types.get(item).map(|fields| (r.name.as_str(), fields.clone()))
            })
            .collect();

        Self { states, resources, item: None }
    }

    fn with_item(&self, fields: Option<HashMap<&'a str, Type>>) -> Scope<'a> {
        Scope { states: self.states.clone(), resources: self.resources.clone(), item: fields }
    }

    fn lookup(&self, name: &str) -> Type {
        // A row field shadows everything else, matching the resolver's scope rule.
        if let Some(item) = &self.item {
            if let Some(t) = item.get(name) {
                return *t;
            }
        }
        if let Some(t) = self.states.get(name) {
            return *t;
        }
        if self.resources.contains_key(name) {
            return Type::List;
        }
        Type::Unknown
    }
}

/// Infer the type of an expression, reporting mismatches as it goes.
pub fn infer(expr: &Expr, scope: &Scope, span: Span, diags: &mut Diagnostics) -> Type {
    match expr {
        Expr::Num(_) => Type::Num,
        Expr::Str(_) => Type::Str,
        Expr::Bool(_) => Type::Bool,

        Expr::Not(inner) => {
            // `!` is defined on anything — it is the truthiness operator — so this reports
            // nothing and simply yields a boolean.
            infer(inner, scope, span, diags);
            Type::Bool
        }

        Expr::Neg(inner) => {
            let t = infer(inner, scope, span, diags);
            if t.known() && t != Type::Num {
                mismatch(diags, span, format!("cannot negate {}", t.name()));
            }
            Type::Num
        }

        Expr::Path { head, steps } => path_type(head, steps, scope, span, diags),

        Expr::Bin { op, lhs, rhs } => {
            let l = infer(lhs, scope, span, diags);
            let r = infer(rhs, scope, span, diags);
            binary(*op, l, r, span, diags)
        }

        // Already reported as `GUML0023`; a second complaint about its type would be noise.
        Expr::Unknown(_) => Type::Unknown,
    }
}

/// The type of a path, checking each aggregate against what it is applied to.
///
/// The aggregates used to be unconditional — `.sum` was always `Num`, `.trim` always `Str` — which made
/// `{tasks.title.sum}` and `{count.trim()}` type-check and then emit JavaScript that returns `NaN` or
/// throws. Threading the row's field types through the steps is what turns those into diagnostics.
///
/// `Type::Unknown` is still never an error. A resource whose `type` is not declared, or a field the
/// declared type does not mention, yields `Unknown` and every check below passes — the alternative is
/// a compiler that refuses documents it merely cannot reason about.
fn path_type(
    head: &str,
    steps: &[Step],
    scope: &Scope,
    span: Span,
    diags: &mut Diagnostics,
) -> Type {
    let mut current = scope.lookup(head);
    let mut row_fields: Option<&HashMap<&str, Type>> = scope.resources.get(head);
    // What the path reads so far, for a message that points at the mistake rather than the whole line.
    let mut so_far = head.to_string();
    // Whether `current` is a *row field of a collection* rather than a scalar. This is the distinction
    // the aggregates turn on, and getting it wrong made a published fixture fail:
    // `{projects.live.count}` counts the rows where `live` is true, so `.count` after a row field is a
    // row count and the field's own type is irrelevant. `{user.name.count}` is a string length. Same
    // syntax, different meaning, and only the collection list separates them.
    let mut row_field = false;

    for step in steps {
        current = match step {
            Step::Agg(agg @ (Aggregate::Count | Aggregate::Sum)) => {
                let numeric = matches!(agg, Aggregate::Sum);
                // `.count` is a length: legal on a list and on a string. `.sum` needs numbers.
                let ok = match (current, numeric) {
                    (Type::Unknown, _) => true,
                    (Type::List, _) => true,
                    // Counting the rows where a field is truthy: legal whatever the field holds.
                    (_, false) if row_field => true,
                    (Type::Str, false) => true,
                    (Type::Num, true) => true,
                    _ => false,
                };
                if !ok {
                    let what = if numeric { "sum" } else { "count" };
                    let needs = if numeric { "a number" } else { "a list or a string" };
                    mismatch(
                        diags,
                        span,
                        format!("`{so_far}` is {current:?}, and `.{what}` needs {needs}"),
                    );
                }
                Type::Num
            }
            // A filter over a list is still a list, and only a list can be filtered.
            Step::Agg(agg @ (Aggregate::Open | Aggregate::Done)) => {
                if !matches!(current, Type::List | Type::Unknown) {
                    mismatch(
                        diags,
                        span,
                        format!(
                            "`{so_far}` is {current:?}, and `.{}` filters a list of rows",
                            if matches!(agg, Aggregate::Open) { "open" } else { "done" }
                        ),
                    );
                }
                // `.open`/`.done` mean "not in the terminal state" / "in it", and the field carrying that
                // state is whichever `bool` the row declares — not necessarily one named `done`. An
                // invoice's is `paid`, a message's is `read`. Requiring the name `done` made the idiom
                // work only for authors who guessed it, and lowered to `!it.done` on a field that did
                // not exist for everyone else: a count that was silently always zero.
                //
                // So: exactly one `bool` is the field. None means there is no state to filter on. Two or
                // more is genuinely ambiguous — `paid` and `overdue` are different questions — and
                // picking one would be a guess with no diagnostic, which is invariant 3.
                if let Some(fields) = row_fields {
                    let bools: Vec<&&str> =
                        fields.iter().filter(|(_, t)| **t == Type::Bool).map(|(n, _)| n).collect();
                    let verb = if matches!(agg, Aggregate::Open) { "open" } else { "done" };
                    if bools.is_empty() {
                        mismatch(
                            diags,
                            span,
                            format!(
                                "`{so_far}` has no boolean field, so `.{verb}` has no state to filter on"
                            ),
                        );
                    } else if bools.len() > 1 {
                        let mut names: Vec<&str> = bools.iter().map(|n| **n).collect();
                        names.sort_unstable();
                        mismatch(
                            diags,
                            span,
                            format!(
                                "`{so_far}` has more than one boolean field ({}), so `.{verb}` is \
                                 ambiguous — filter with `where` instead",
                                names.join(", ")
                            ),
                        );
                    }
                }
                Type::List
            }
            Step::Agg(agg @ (Aggregate::Trim | Aggregate::Lower | Aggregate::Upper)) => {
                if !matches!(current, Type::Str | Type::Unknown) {
                    let name = match agg {
                        Aggregate::Trim => "trim",
                        Aggregate::Lower => "lower",
                        _ => "upper",
                    };
                    mismatch(
                        diags,
                        span,
                        format!("`{so_far}` is {current:?}, and `.{name}` needs a string"),
                    );
                }
                Type::Str
            }
            Step::Field(name) => {
                row_field = row_fields.is_some();
                match row_fields.and_then(|f| f.get(name.as_str())) {
                    Some(t) => *t,
                    // The row type *is* declared and has no such field.
                    //
                    // `{members.admin.count}` over `type Member {id, name, email, role, active:bool}`
                    // lowered to `members.filter((it) => it.admin).length` — always zero, because no row
                    // has an `admin` property. The document meant "how many members have the admin role",
                    // which is a comparison and not a truthiness filter, and the compiler said nothing:
                    // the count was wrong, the banner it guarded was permanently visible, and only `tsc`
                    // over the emitted file would have noticed.
                    //
                    // This is the same shape as the `.done` hardcode two arms above, and it is refused for
                    // the same reason. Only checked at the first step, because `row_fields` is cleared
                    // after one field lookup — a deeper path has left the row type behind and nothing here
                    // knows what it is looking at.
                    None if row_fields.is_some() => {
                        let mut names: Vec<&str> =
                            row_fields.map(|f| f.keys().copied().collect()).unwrap_or_default();
                        names.sort_unstable();
                        // Its own help rather than `mismatch`'s, whose advice is about state types and
                        // would send the reader to the wrong declaration entirely.
                        diags.push(
                            Diagnostic::error(
                                Code::TypeMismatch,
                                format!(
                                    "`{so_far}` has no field `{name}`; the row declares {}",
                                    names.join(", ")
                                ),
                                span,
                            )
                            .with_help(
                                "an aggregate after a field counts the rows where that field is truthy, \
                                 so the field has to exist on the row type; comparing a field to a value \
                                 is `where=` on the repeater, not an aggregate",
                            ),
                        );
                        Type::Unknown
                    }
                    // A field of something whose type is not declared. Unknown, not an error.
                    None => Type::Unknown,
                }
            }
        };

        so_far.push('.');
        so_far.push_str(match step {
            Step::Field(n) => n.as_str(),
            Step::Agg(Aggregate::Count) => "count",
            Step::Agg(Aggregate::Sum) => "sum",
            Step::Agg(Aggregate::Open) => "open",
            Step::Agg(Aggregate::Done) => "done",
            Step::Agg(Aggregate::Trim) => "trim",
            Step::Agg(Aggregate::Lower) => "lower",
            Step::Agg(Aggregate::Upper) => "upper",
        });

        // A field lookup consumes the row context; `.open`/`.done` preserve it, because the result is
        // still a list of the same rows. That is what makes `tasks.open.count` work.
        if !matches!(step, Step::Agg(Aggregate::Open | Aggregate::Done)) {
            row_fields = None;
        }
        if !matches!(step, Step::Field(_)) {
            row_field = false;
        }
    }

    current
}

fn binary(op: BinOp, l: Type, r: Type, span: Span, diags: &mut Diagnostics) -> Type {
    use BinOp::*;
    match op {
        And | Or => {
            // Defined on anything: JavaScript's `&&` returns an operand, not a boolean, and GUML
            // keeps that semantics. Nothing to check.
            Type::Unknown
        }

        Eq | Ne => {
            // Comparing two known, different types is always false, which is never intended.
            if l.known() && r.known() && l != r {
                mismatch(
                    diags,
                    span,
                    format!("comparing {} with {} is always false", l.name(), r.name()),
                );
            }
            Type::Bool
        }

        Lt | Le | Gt | Ge => {
            for t in [l, r] {
                if t.known() && t != Type::Num {
                    mismatch(diags, span, format!("cannot order {}", t.name()));
                    break;
                }
            }
            Type::Bool
        }

        Add => {
            // `+` is string concatenation when either side is a string, which is the one place
            // JavaScript's coercion is genuinely wanted.
            match (l, r) {
                (Type::Str, _) | (_, Type::Str) => Type::Str,
                (Type::Bool, _) | (_, Type::Bool) => {
                    mismatch(diags, span, "cannot add a boolean".to_string());
                    Type::Num
                }
                (Type::List, _) | (_, Type::List) => {
                    mismatch(diags, span, "cannot add a list".to_string());
                    Type::Num
                }
                _ => Type::Num,
            }
        }

        Sub | Mul | Div => {
            for t in [l, r] {
                if t.known() && t != Type::Num {
                    mismatch(diags, span, format!("cannot do arithmetic on {}", t.name()));
                    break;
                }
            }
            Type::Num
        }
    }
}

fn mismatch(diags: &mut Diagnostics, span: Span, message: String) {
    diags.push(Diagnostic::error(Code::TypeMismatch, message, span).with_help(
        "state types are inferred from their initial value: `state n=0` is a number, \
             `state s=\"\"` a string, `state b=false` a boolean",
    ));
}

/// Walk the document and check every expression.
pub fn check(program: &Program, diags: &mut Diagnostics) {
    let scope = Scope::new(program);
    for el in &program.tree {
        element(el, program, &scope, diags);
    }

    // An effect trigger is an expression like any other, so it goes through the same inference: a bad
    // aggregate in `on {tasks.title.sum}` is the same error there as in `metric {tasks.title.sum}`.
    // Its *type* is deliberately unconstrained — anything can serve as a dependency.
    for e in &program.effects {
        if let guml_ast::Trigger::Change(text) = &e.trigger {
            let expr = guml_syntax::expr::parse(text);
            infer(&expr, &scope, e.span, diags);
            enum_comparisons(&expr, program, e.span, diags);
        }
    }
}

/// A comparison against a value the enumerated state can never hold.
///
/// Assignment was already checked — `>filter="opne"` is `GUML0080`. Comparison was not, and it is the
/// more dangerous half: `{filter == "opne"}` is not a type error, it is **dead code**. The branch
/// simply never runs, the page silently renders the wrong thing, and nothing in the pipeline had an
/// opinion about it.
///
/// This is the exhaustiveness the domain makes possible: a closed set of values is only useful if the
/// compiler holds comparisons to it.
fn enum_comparisons(expr: &Expr, program: &Program, span: Span, diags: &mut Diagnostics) {
    let Expr::Bin { op, lhs, rhs } = expr else {
        // Recurse into the shapes that can contain a comparison.
        match expr {
            Expr::Not(inner) | Expr::Neg(inner) => enum_comparisons(inner, program, span, diags),
            _ => {}
        }
        return;
    };

    if matches!(op, BinOp::Eq | BinOp::Ne) {
        // Either order: `filter == "x"` and `"x" == filter`.
        for (path, literal) in [(lhs.as_ref(), rhs.as_ref()), (rhs.as_ref(), lhs.as_ref())] {
            if let Expr::Path { head, steps } = path
                && steps.is_empty()
                && let Expr::Str(value) = literal
                && let Some(state) = program.state(head)
                && !state.domain.is_empty()
                && !state.domain.iter().any(|d| d == value)
            {
                diags.push(
                    Diagnostic::error(
                        Code::NotEnumerated,
                        format!(
                            "`{head}` can never equal `{value}`: it is not in its domain"
                        ),
                        span,
                    )
                    .with_help(format!(
                        "one of: {} — a comparison outside the domain is dead code, not a type error",
                        state.domain.join(", ")
                    )),
                );
            }
        }
    }

    enum_comparisons(lhs, program, span, diags);
    enum_comparisons(rhs, program, span, diags);
}

fn element(el: &Element, program: &Program, scope: &Scope, diags: &mut Diagnostics) {
    for p in &el.positionals {
        if let Positional::Binding(b) = p {
            infer(&b.expr, scope, el.span, diags);
            enum_comparisons(&b.expr, program, el.span, diags);
        }
    }
    for a in &el.attrs {
        if let Value::Binding(b) = &a.value {
            enum_comparisons(&b.expr, program, a.span, diags);
            let t = infer(&b.expr, scope, a.span, diags);
            attribute(&el.tag, &a.name, t, a.span, diags);
        }
    }

    // A text tag's line remainder is prose, so `metric {count + 1}` is an interpolation rather
    // than a positional. Prose is stored verbatim and has no pre-parsed tree, so it is parsed
    // here — the one place in the checker that has to.
    for text in el.content.iter().chain(el.text_lines.iter()) {
        if el.is_escape() {
            break;
        }
        for source in guml_syntax::expr::interpolations(text) {
            let expr = guml_syntax::expr::parse(source);
            infer(&expr, scope, el.span, diags);
            enum_comparisons(&expr, program, el.span, diags);
        }
    }

    // A repeater puts the row's fields in scope for its children.
    let child_scope = match repeater_fields(el, program, scope) {
        Some(fields) => scope.with_item(Some(fields)),
        None => scope.with_item(scope.item.clone()),
    };

    for child in &el.children {
        element(child, program, &child_scope, diags);
    }
}

/// Attributes whose value has a required type.
fn attribute(tag: &str, name: &str, t: Type, span: Span, diags: &mut Diagnostics) {
    // `cols` on a repeater is the column *header list*, not a count — see `validate::numeric_on`.
    if name == "cols" && matches!(tag, "list" | "table") {
        return;
    }
    let expected = match name {
        "cols" | "gap" | "min" | "max" | "open" => Type::Num,
        // `disabled={draft}` on a string is a truthiness bug: an empty string is falsy, so it
        // works by accident until someone types a space. JavaScript would not complain, which is
        // precisely why the compiler should.
        "disabled" | "readonly" | "required" | "hidden" | "strike" | "loading" => Type::Bool,
        _ => return,
    };

    if t.known() && t != expected {
        diags.push(
            Diagnostic::error(
                Code::TypeMismatch,
                format!("`{name}` takes {}, not {}", expected.name(), t.name()),
                span,
            )
            .with_help(match expected {
                Type::Bool => "compare it, or negate it: `disabled={!draft.trim()}`",
                _ => "give it a number, or a binding that produces one",
            }),
        );
    }
}

fn repeater_fields<'a>(
    el: &Element,
    program: &'a Program,
    scope: &Scope<'a>,
) -> Option<HashMap<&'a str, Type>> {
    if !matches!(el.tag.as_str(), "list" | "table") {
        return None;
    }
    // `of=` no longer names an alternative *source*; it names the row **type**. So the fields come from the
    // shared `repeater_rows`, and a derived array gets its row scope exactly like a resource's does.
    let rows = program.repeater_rows(el)?;
    if let Some(fields) = scope.resources.get(rows.source.as_str()) {
        return Some(fields.clone());
    }
    let decl = program.types.iter().find(|t| t.name == rows.ty)?;
    Some(decl.fields.iter().map(|f| (f.name.as_str(), field_type(&f.ty))).collect())
}

#[cfg(test)]
mod tests {
    // The tests drive the whole checker through `crate::check`, which is what a caller does, so
    // nothing from this module needs importing.
    fn codes(src: &str) -> Vec<String> {
        crate::check(src).1.items.iter().map(|d| d.id.to_string()).collect()
    }

    fn fires(src: &str) -> bool {
        codes(src).contains(&"GUML0065".to_string())
    }

    #[test]
    fn arithmetic_on_a_string_or_a_boolean_is_an_error() {
        assert!(fires("page P\nstate s=\"\"\n\nmetric {s - 1}\n"));
        assert!(fires("page P\nstate b=false\n\nmetric {b + 1}\n"));
        assert!(fires("page P\nstate s=\"\"\n\nmetric {s * 2}\n"));
    }

    #[test]
    fn string_concatenation_is_allowed() {
        // `+` with a string is the one coercion GUML keeps, because it is the one people want.
        assert!(!fires("page P\nstate s=\"\"\n\nhead {s + \"!\"}\n"));
        assert!(!fires("page P\nstate n=0\nstate s=\"\"\n\nhead {s + n}\n"));
    }

    #[test]
    fn ordering_needs_numbers() {
        assert!(fires("page P\nstate s=\"\"\n\nmetric {s > 5}\n"));
        assert!(!fires("page P\nstate n=0\n\nmetric {n > 5}\n"));
        // A count is a number, so this is fine.
        assert!(!fires(
            "page P\ntype T {id}\ndata rows:T[] GET /api/rows\n\nhead {rows.count > 0}\n\nlist rows\n  text {id}\n"
        ));
    }

    #[test]
    fn comparing_two_different_known_types_is_always_false() {
        assert!(fires("page P\nstate n=0\n\nmetric {n == \"1\"}\n"));
        assert!(!fires("page P\nstate n=0\n\nmetric {n == 1}\n"));
        assert!(!fires("page P\nstate s=\"\"\n\nmetric {s == \"x\"}\n"));
    }

    #[test]
    fn a_boolean_attribute_rejects_a_string() {
        // The bug this exists for: an empty string is falsy, so it works until it does not.
        assert!(fires("page P\nstate draft=\"\"\n\nbtn Go disabled={draft}\n"));
        assert!(!fires("page P\nstate draft=\"\"\n\nbtn Go disabled={!draft.trim()}\n"));
        assert!(!fires("page P\nstate busy=false\n\nbtn Go disabled={busy}\n"));
    }

    #[test]
    fn a_numeric_attribute_rejects_a_boolean() {
        assert!(fires("page P\nstate b=false\n\nsection X cols={b}\n  p y\n"));
        assert!(!fires("page P\nstate n=3\n\nsection X cols={n}\n  p y\n"));
    }

    #[test]
    fn row_fields_are_typed_from_the_declaration() {
        // `done:bool` is a boolean, so this is fine…
        assert!(!fires(
            "page P\ntype T {id, done:bool}\ndata rows:T[] GET /api/rows\n\nlist rows\n  text {id} strike={done}\n"
        ));
        // …and `id` is a string, so using it as a boolean attribute is not.
        assert!(fires(
            "page P\ntype T {id, done:bool}\ndata rows:T[] GET /api/rows\n\nlist rows\n  text {id} strike={id}\n"
        ));
    }

    #[test]
    fn an_unknown_type_is_never_an_error() {
        // No `type` directive, so nothing about the rows is knowable. A checker that guessed here
        // would report errors on a correct document.
        assert!(!fires("page P\n\nmetric {whatever + 1}\n"));
        assert!(!fires("page P\nstate x={y}\n\nmetric {x - 1}\n"));
    }

    #[test]
    fn the_fixtures_have_no_type_errors() {
        for name in ["a.guml", "b.guml", "c.guml", "portfolio.guml"] {
            let src = std::fs::read_to_string(format!("../../fixtures/{name}")).expect("fixture");
            let found = codes(&src);
            assert!(
                !found.contains(&"GUML0065".to_string()),
                "{name} should have no type errors, got {found:?}"
            );
        }
    }
}
