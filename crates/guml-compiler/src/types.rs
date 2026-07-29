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

        Expr::Path { head, steps } => path_type(head, steps, scope),

        Expr::Bin { op, lhs, rhs } => {
            let l = infer(lhs, scope, span, diags);
            let r = infer(rhs, scope, span, diags);
            binary(*op, l, r, span, diags)
        }

        // Already reported as `GUML0023`; a second complaint about its type would be noise.
        Expr::Unknown(_) => Type::Unknown,
    }
}

fn path_type(head: &str, steps: &[Step], scope: &Scope) -> Type {
    let mut current = scope.lookup(head);
    let mut row_fields: Option<&HashMap<&str, Type>> = scope.resources.get(head);

    for step in steps {
        current = match step {
            Step::Agg(Aggregate::Count) => Type::Num,
            Step::Agg(Aggregate::Sum) => Type::Num,
            // A filter over a list is still a list.
            Step::Agg(Aggregate::Open | Aggregate::Done) => Type::List,
            Step::Agg(Aggregate::Trim | Aggregate::Lower | Aggregate::Upper) => Type::Str,
            Step::Field(name) => match row_fields.and_then(|f| f.get(name.as_str())) {
                Some(t) => *t,
                // A field of something whose type is not declared. Unknown, not an error.
                None => Type::Unknown,
            },
        };
        // Only the first step can be a row field of the head resource; after that the type is a
        // scalar and there is nothing further to look up.
        if !matches!(step, Step::Agg(Aggregate::Open | Aggregate::Done)) {
            row_fields = None;
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
}

fn element(el: &Element, program: &Program, scope: &Scope, diags: &mut Diagnostics) {
    for p in &el.positionals {
        if let Positional::Binding(b) = p {
            infer(&b.expr, scope, el.span, diags);
        }
    }
    for a in &el.attrs {
        if let Value::Binding(b) = &a.value {
            let t = infer(&b.expr, scope, a.span, diags);
            attribute(&a.name, t, a.span, diags);
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
            infer(&guml_syntax::expr::parse(source), scope, el.span, diags);
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
fn attribute(name: &str, t: Type, span: Span, diags: &mut Diagnostics) {
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
    let source = el
        .positionals
        .iter()
        .find_map(|p| match p {
            Positional::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .or_else(|| el.attr("of").and_then(|v| v.as_text()))?;
    let _ = program;
    scope.resources.get(source).cloned()
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
