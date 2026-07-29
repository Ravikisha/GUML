//! Lowering GUML expressions to JavaScript.
//!
//! # One grammar, one parser
//!
//! This file used to contain its own lexer and precedence-climbing parser, which meant the
//! expression grammar existed twice in Rust and a third time in TypeScript for the browser
//! runtime. Three implementations of one grammar is how a live preview starts disagreeing with
//! emitted code, which is the most misleading thing this project could ship.
//!
//! The parser now lives in [`guml_syntax::expr`] and this file is only a *lowering*: tree in,
//! JavaScript out. `packages/guml/src/eval.ts` remains a second implementation by necessity —
//! it evaluates rather than emits — and is held to this one by a parity test.
//!
//! # What is not lowered
//!
//! [`Expr::Unknown`] is emitted as the author's own text. The parser has already reported it as
//! `GUML0023`; inventing a lowering for syntax the grammar does not cover would be the silent
//! mis-lowering invariant 3 forbids.

use guml_syntax::expr::{Aggregate, Expr, Step, parse};
use std::fmt::Write as _;

/// Names that resolve against the current repeater item rather than the component.
///
/// Inside `list tasks`, `{title}` means the row's title, so it has to be emitted as
/// `item.title`. Everything else — state, resources — stays as written.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    pub item_fields: Vec<String>,
    /// Variable the row is bound to in emitted code.
    pub item_var: String,
    /// Names that hold arrays (the declared resources).
    ///
    /// Needed to tell `projects.live.count` — count the rows where `live` is true — from
    /// `user.name.count`, where `.count` is a string length. Without the distinction one of the
    /// two has to be lowered wrongly, and guessing produces code that does not compile.
    pub collections: Vec<String>,
}

impl Ctx {
    pub fn item(fields: &[String]) -> Self {
        Self { item_fields: fields.to_vec(), item_var: "item".to_string(), collections: Vec::new() }
    }

    /// Records which names are arrays, for the field-aggregate rule.
    pub fn with_collections(mut self, names: &[String]) -> Self {
        self.collections = names.to_vec();
        self
    }

    fn qualify(&self, head: &str) -> String {
        if self.item_fields.iter().any(|f| f == head) {
            let var = if self.item_var.is_empty() { "item" } else { &self.item_var };
            return format!("{var}.{head}");
        }
        head.to_string()
    }

    fn is_collection(&self, head: &str) -> bool {
        self.collections.iter().any(|c| c == head)
    }
}

/// Lower one GUML expression to a JavaScript expression.
pub fn lower(expr: &str) -> String {
    lower_in(expr, &Ctx::default())
}

pub fn lower_in(expr: &str, ctx: &Ctx) -> String {
    emit(&parse(expr), ctx)
}

/// Lower an already-parsed expression. The path a caller takes when it parsed once and wants to
/// both check and emit — which is the point of having a shared tree.
pub fn lower_expr(expr: &Expr, ctx: &Ctx) -> String {
    emit(expr, ctx)
}

fn emit(expr: &Expr, ctx: &Ctx) -> String {
    match expr {
        Expr::Num(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        Expr::Str(s) => format!("{s:?}"),
        Expr::Bool(b) => b.to_string(),
        Expr::Not(inner) => format!("!{}", emit(inner, ctx)),
        Expr::Neg(inner) => format!("-{}", emit(inner, ctx)),
        Expr::Bin { op, lhs, rhs } => {
            // Parenthesised unconditionally: JavaScript's precedence agrees with GUML's here,
            // but relying on that agreement makes every future operator a correctness risk.
            format!("({} {} {})", emit(lhs, ctx), op.as_js(), emit(rhs, ctx))
        }
        Expr::Path { head, steps } => path(head, steps, ctx),
        // Reported as `GUML0023` at parse time. Emitting the author's text keeps the failure
        // visible rather than inventing a meaning for it.
        Expr::Unknown(text) => text.clone(),
    }
}

fn path(head: &str, steps: &[Step], ctx: &Ctx) -> String {
    let mut out = ctx.qualify(head);
    // Whether what we have so far is an array, which decides how a bare field followed by
    // `.count` is read.
    let mut collection = ctx.is_collection(head);
    let mut i = 0;

    while i < steps.len() {
        match &steps[i] {
            Step::Agg(Aggregate::Count) => {
                out = format!("{out}.length");
                collection = false;
            }
            // `it` rather than a single letter: the emitted code is read by people.
            Step::Agg(Aggregate::Open) => out = format!("{out}.filter((it) => !it.done)"),
            Step::Agg(Aggregate::Done) => out = format!("{out}.filter((it) => it.done)"),
            Step::Agg(Aggregate::Sum) => {
                out = format!("{out}.reduce((a, b) => a + Number(b), 0)");
                collection = false;
            }
            Step::Agg(Aggregate::Trim) => out = format!("{out}.trim()"),
            Step::Agg(Aggregate::Lower) => out = format!("{out}.toLowerCase()"),
            Step::Agg(Aggregate::Upper) => out = format!("{out}.toUpperCase()"),
            Step::Field(name) => {
                // `projects.live.count` — a field of the row, then an aggregate over it. Only
                // applied to a known array, so `user.name.count` still means string length.
                let next_aggregates = matches!(
                    steps.get(i + 1),
                    Some(Step::Agg(Aggregate::Count)) | Some(Step::Agg(Aggregate::Sum))
                );
                if collection && next_aggregates {
                    out = match steps.get(i + 1) {
                        Some(Step::Agg(Aggregate::Sum)) => format!("{out}.map((it) => it.{name})"),
                        _ => format!("{out}.filter((it) => it.{name})"),
                    };
                } else {
                    out = format!("{out}.{name}");
                    collection = false;
                }
            }
        }
        i += 1;
    }

    out
}

/// Interpolate a prose string containing `{expr}` into a JSX-safe expression list.
///
/// `Tasks — {tasks.open.count} open` becomes
/// `Tasks — {tasks.filter((it) => !it.done).length} open`, which is valid JSX text with an
/// embedded expression.
pub fn lower_text(text: &str) -> String {
    lower_text_in(text, &Ctx::default())
}

pub fn lower_text_in(text: &str, ctx: &Ctx) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let _ = write!(out, "{{{}}}", lower_in(&after[..close], ctx));
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(&rest[open..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A string **attribute** value, which cannot be interpolated the way JSX text is.
///
/// `aria="Delete {title}"` lowered as text yields `aria-label="Delete {item.title}"` — and
/// inside quotes JSX has no expression syntax, so the braces reach the DOM verbatim and the
/// accessible name reads "Delete {item.title}". A binding therefore forces the whole value into
/// a template literal instead.
pub fn lower_attr_value_in(text: &str, ctx: &Ctx) -> String {
    if !text.contains('{') {
        return format!("{text:?}");
    }
    let mut out = String::from("{`");
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&escape_template(&rest[..open]));
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let _ = write!(out, "${{{}}}", lower_in(&after[..close], ctx));
                rest = &after[close + 1..];
            }
            None => {
                // Unterminated: keep the text rather than emitting broken JS. The parser has
                // already reported the brace.
                out.push_str(&escape_template(&rest[open..]));
                rest = "";
                break;
            }
        }
    }
    out.push_str(&escape_template(rest));
    out.push_str("`}");
    out
}

/// Backslash, backtick and `{` are what would otherwise end the literal or open an
/// interpolation. `{` is escaped rather than only `${` because the `$` and the `{` can arrive
/// from different segments: `a${x` with no closing brace would otherwise emit `` `a${x` `` — an
/// unterminated interpolation, and a syntax error. `\{` is a NonEscapeCharacter, so it is legal
/// and means a literal brace.
fn escape_template(s: &str) -> String {
    s.replace('\\', "\\\\").replace('`', "\\`").replace('{', "\\{")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_paths_pass_through() {
        assert_eq!(lower("count"), "count");
        assert_eq!(lower("user.name"), "user.name");
    }

    #[test]
    fn aggregates_become_real_javascript() {
        let ctx = Ctx::default().with_collections(&["tasks".to_string()]);
        assert_eq!(lower_in("tasks.count", &ctx), "tasks.length");
        assert_eq!(lower_in("tasks.open.count", &ctx), "tasks.filter((it) => !it.done).length");
        assert_eq!(lower_in("tasks.done.count", &ctx), "tasks.filter((it) => it.done).length");
        assert_eq!(lower("draft.trim()"), "draft.trim()");
    }

    #[test]
    fn a_field_aggregate_needs_a_known_collection() {
        let ctx = Ctx::default().with_collections(&["projects".to_string()]);
        assert_eq!(
            lower_in("projects.live.count", &ctx),
            "projects.filter((it) => it.live).length"
        );
        // Not a collection, so `.count` is a string length and `.name` is a field.
        assert_eq!(lower_in("user.name.count", &ctx), "user.name.length");
    }

    #[test]
    fn operators_and_precedence() {
        assert_eq!(lower("count > 0"), "(count > 0)");
        assert_eq!(lower("1 + 2 * 3"), "(1 + (2 * 3))");
        assert_eq!(lower("!done"), "!done");
        assert_eq!(lower("!draft.trim()"), "!draft.trim()");
        assert_eq!(lower("filter == \"open\""), "(filter === \"open\")");
    }

    #[test]
    fn item_fields_are_qualified_inside_a_row() {
        let ctx = Ctx::item(&["title".to_string(), "done".to_string()]);
        assert_eq!(lower_in("title", &ctx), "item.title");
        assert_eq!(lower_in("!done", &ctx), "!item.done");
        // Names that are not fields of the row are left alone.
        assert_eq!(lower_in("filter", &ctx), "filter");
        assert_eq!(lower_text_in("Delete {title}", &ctx), "Delete {item.title}");
    }

    #[test]
    fn prose_interpolation() {
        let ctx = Ctx::default().with_collections(&["tasks".to_string()]);
        assert_eq!(
            lower_text_in("Tasks — {tasks.open.count} open", &ctx),
            "Tasks — {tasks.filter((it) => !it.done).length} open"
        );
    }

    #[test]
    fn unbalanced_braces_are_left_alone() {
        assert_eq!(lower_text("a {count"), "a {count");
    }

    #[test]
    fn string_helpers() {
        assert_eq!(lower("t.lower"), "t.toLowerCase()");
        assert_eq!(lower("t.upper"), "t.toUpperCase()");
    }

    #[test]
    fn attribute_values_switch_to_a_template_literal_when_bound() {
        let ctx = Ctx::item(&["title".to_string()]);
        assert_eq!(lower_attr_value_in("Delete", &ctx), "\"Delete\"");
        assert_eq!(lower_attr_value_in("Delete {title}", &ctx), "{`Delete ${item.title}`}");
        assert_eq!(lower_attr_value_in("{title}", &ctx), "{`${item.title}`}");
    }

    #[test]
    fn template_literal_escaping_cannot_break_out_of_the_literal() {
        let ctx = Ctx::item(&["title".to_string()]);
        assert_eq!(lower_attr_value_in("a`b {title}", &ctx), "{`a\\`b ${item.title}`}");
        // `{x}` is a binding in GUML even behind a `$`, so the `$` stays literal text and the
        // binding interpolates normally.
        assert_eq!(
            lower_attr_value_in("cost ${x} for {title}", &ctx),
            "{`cost $${x} for ${item.title}`}"
        );
        // An unterminated brace must not become an interpolation.
        assert_eq!(lower_attr_value_in("a${x", &ctx), "{`a$\\{x`}");
    }

    #[test]
    fn syntax_outside_the_grammar_is_returned_verbatim() {
        // `GUML0023` reports it; codegen must not invent a lowering.
        let weird = "a ? b : c";
        assert_eq!(lower(weird), weird);
    }
}
