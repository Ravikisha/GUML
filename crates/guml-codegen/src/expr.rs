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
    /// Each collection's single boolean field, so `.open`/`.done` can name it.
    ///
    /// These used to lower to a hardcoded `!it.done`. That worked for a fixture whose field happened to
    /// be called `done` and was silently wrong for everything else: a Phase 0 example modelling invoices
    /// with `paid:bool` emitted `!it.done` on a field that does not exist, so the count was always zero
    /// and nothing said so. A convention the compiler owns cannot depend on the author guessing the name
    /// the compiler had in mind.
    pub row_bool: Vec<(String, String)>,
    /// Common-subexpression substitutions: lowered JavaScript → the memo variable holding it.
    ///
    /// `{tasks.open.count}` lowers to `tasks.filter((it) => !it.done).length`, an O(n) scan. Used three
    /// times on a page, that is three scans per render for one value. The backend hoists the repeated
    /// ones into a `useMemo` and records the mapping here, so lowering substitutes the variable instead.
    ///
    /// Applied at the *lowered* string rather than at the source, because two spellings can lower to the
    /// same JavaScript and both should share the memo.
    pub cse: Vec<(String, String)>,
    /// Prefix for a non-row identifier — `this.#state.` for a backend holding state in an object.
    /// Empty for the frameworks where a state name is simply in scope. See [`Ctx::with_scope`].
    pub scope: String,
}

impl Ctx {
    pub fn item(fields: &[String]) -> Self {
        Self { item_fields: fields.to_vec(), item_var: "item".to_string(), ..Self::default() }
    }

    /// Records which names are arrays, for the field-aggregate rule.
    pub fn with_collections(mut self, names: &[String]) -> Self {
        self.collections = names.to_vec();
        self
    }

    pub fn with_row_bool(mut self, pairs: &[(String, String)]) -> Self {
        self.row_bool = pairs.to_vec();
        self
    }

    pub fn with_cse(mut self, cse: &[(String, String)]) -> Self {
        self.cse = cse.to_vec();
        self
    }

    /// Prefix every non-row identifier with this, for a backend that keeps state in an object rather
    /// than in scope.
    ///
    /// # Why this belongs here and not in the backend
    ///
    /// The Web Components backend needs `count` to become `this.#state.count`, and the first version did
    /// it by rewriting the *lowered string* — walking the JavaScript and prefixing every bare word. That
    /// cannot work, and the output said so:
    ///
    /// ```text
    /// `s.Invoices — ${…} s.awaiting s.payment`   // every word of the literal text
    /// (s.a, s.b) => s.a + Number(s.b)           // the lambda's own parameters
    /// s.view === "s.all"                        // inside a string literal
    /// ```
    ///
    /// Three different kinds of thing that are not identifier reads, and no amount of tightening the
    /// word-boundary rules distinguishes them from one — that information exists only in the parse tree.
    /// So the prefix is applied at the one place that knows it is looking at a path head.
    pub fn with_scope(mut self, prefix: &str) -> Self {
        self.scope = prefix.to_string();
        self
    }

    fn qualify(&self, head: &str) -> String {
        if self.item_fields.iter().any(|f| f == head) {
            let var = if self.item_var.is_empty() { "item" } else { &self.item_var };
            return format!("{var}.{head}");
        }
        if !self.scope.is_empty() {
            return format!("{}{head}", self.scope);
        }
        head.to_string()
    }

    fn is_collection(&self, head: &str) -> bool {
        self.collections.iter().any(|c| c == head)
    }

    /// The boolean field `.open`/`.done` filter on, for the collection `head`.
    ///
    /// Falls back to `done` when the collection is unknown — a `Ctx` built without row types (the
    /// expression unit tests, a snippet lowered on its own) then behaves as it always did, and the
    /// analyser has already rejected the case where no such field exists.
    pub(crate) fn row_bool_field(&self, head: &str) -> &str {
        self.row_bool.iter().find(|(c, _)| c == head).map(|(_, f)| f.as_str()).unwrap_or("done")
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
    let out = emit_raw(expr, ctx);
    // Substitution happens after lowering, and only on a whole expression: replacing a fragment of a
    // larger expression would produce `tasksOpenCount > 0` correctly but also corrupt any expression
    // that merely contains the same substring.
    for (lowered, name) in &ctx.cse {
        if out == *lowered {
            return name.clone();
        }
    }
    out
}

fn emit_raw(expr: &Expr, ctx: &Ctx) -> String {
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
            // The field name comes from the row type, not from a guess. `head` is the collection the
            // path started at, so a `.open` deeper in a chain still resolves against the right rows.
            Step::Agg(Aggregate::Open) => {
                let field = ctx.row_bool_field(head);
                out = format!("{out}.filter((it) => !it.{field})");
            }
            Step::Agg(Aggregate::Done) => {
                let field = ctx.row_bool_field(head);
                out = format!("{out}.filter((it) => it.{field})");
            }
            Step::Agg(Aggregate::Sum) => {
                out = format!("{out}.reduce((a, b) => a + Number(b), 0)");
                collection = false;
            }
            Step::Agg(Aggregate::Trim) => out = format!("{out}.trim()"),
            Step::Agg(Aggregate::Lower) => out = format!("{out}.toLowerCase()"),
            Step::Agg(Aggregate::Upper) => out = format!("{out}.toUpperCase()"),
            Step::Field(name) => {
                // `projects.live.count` — a field of the row, then an aggregate over it. Only applied to
                // a known array, so `user.name.count` still means string length.
                //
                // # The two-field chain, and the mis-lowering it used to be
                //
                // `invoices.paid.amount.sum` — "the sum of the amounts of the paid invoices" — is the
                // shape every dashboard and cart total in GUML-Bench needs, and it emitted
                // `invoices.paid.amount.reduce(…)`. `.paid` on an *array* is `undefined`, so that throws
                // at runtime, and the compiler said nothing: only the field immediately before the
                // aggregate was recognised, and anything earlier fell through to a plain property read.
                //
                // A field on a collection with an aggregate still to come is a filter, because that is the
                // only reading under which the chain means anything: you cannot sum a boolean and the
                // aggregate needs rows to work on. So the collection survives the step.
                let aggregate_still_to_come = steps[i + 1..]
                    .iter()
                    .any(|s| matches!(s, Step::Agg(Aggregate::Count) | Step::Agg(Aggregate::Sum)));
                match (collection, steps.get(i + 1)) {
                    // The last field before the aggregate. `.sum` needs the values, `.count` the rows.
                    (true, Some(Step::Agg(Aggregate::Sum))) => {
                        out = format!("{out}.map((it) => it.{name})");
                    }
                    (true, Some(Step::Agg(Aggregate::Count))) => {
                        out = format!("{out}.filter((it) => it.{name})");
                    }
                    // A field earlier in the chain, narrowing the rows the aggregate will see.
                    (true, _) if aggregate_still_to_come => {
                        out = format!("{out}.filter((it) => it.{name})");
                    }
                    _ => {
                        out = format!("{out}.{name}");
                        collection = false;
                    }
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
    fn a_field_chain_filters_then_aggregates() {
        // The mis-lowering: `invoices.paid.amount.sum` emitted `invoices.paid.amount.reduce(…)`, and
        // `.paid` on an array is `undefined`, so the emitted code threw at runtime with no diagnostic.
        // Only the field immediately before the aggregate was recognised.
        let ctx = Ctx::default().with_collections(&["invoices".to_string()]);
        assert_eq!(
            lower_in("invoices.paid.amount.sum", &ctx),
            "invoices.filter((it) => it.paid).map((it) => it.amount).reduce((a, b) => a + Number(b), 0)"
        );
        // The one-field forms are unchanged: `.sum` wants the values, `.count` wants the rows.
        assert_eq!(
            lower_in("invoices.amount.sum", &ctx),
            "invoices.map((it) => it.amount).reduce((a, b) => a + Number(b), 0)"
        );
        assert_eq!(
            lower_in("invoices.paid.count", &ctx),
            "invoices.filter((it) => it.paid).length"
        );
        // And a chain with no aggregate at the end is still a plain property read, so a document that
        // means `user.address.city` is not turned into a filter.
        assert_eq!(lower_in("invoices.client.name", &ctx), "invoices.client.name");
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
