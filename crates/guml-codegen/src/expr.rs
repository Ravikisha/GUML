//! Lowering GUML binding expressions to JavaScript.
//!
//! Until now bindings were passed through verbatim, which happens to work for
//! `{count}` and breaks for everything interesting: `{tasks.open.count}` is not
//! valid JS, and emitting it produced code that could not run. That gap is also
//! why the browser runtime and the React backend disagreed — the runtime had a
//! real evaluator and the code generator did not.
//!
//! The semantics here mirror `packages/guml/src/eval.ts` deliberately. If one
//! changes, the other has to: a preview that disagrees with emitted code is worse
//! than no preview.
//!
//! The language is small on purpose (report §5.9): paths, comparisons, boolean
//! and arithmetic operators, and a fixed set of collection aggregates. Anything
//! else belongs in a `js` block.

use std::fmt::Write as _;

/// A path segment that means something to the compiler rather than being a field.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Aggregate {
    Count,
    Open,
    Done,
    Sum,
    Trim,
    Lower,
    Upper,
}

impl Aggregate {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "count" | "length" => Aggregate::Count,
            "open" => Aggregate::Open,
            "done" => Aggregate::Done,
            "sum" => Aggregate::Sum,
            "trim" => Aggregate::Trim,
            "lower" => Aggregate::Lower,
            "upper" => Aggregate::Upper,
            _ => None?,
        })
    }
}

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
    /// `user.name.count`, where `.count` is a string length. Without the distinction one of
    /// the two has to be lowered wrongly, and guessing produces code that does not compile.
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

    fn qualify(&self, head: &str) -> Option<String> {
        if self.item_fields.iter().any(|f| f == head) {
            let var = if self.item_var.is_empty() { "item" } else { &self.item_var };
            return Some(format!("{var}.{head}"));
        }
        None
    }
}

/// Lower one GUML expression to a JavaScript expression.
///
/// An unknown leading identifier is still emitted — the resolver reports it as
/// `GUML0033`, and codegen does not need to duplicate that judgement.
pub fn lower(expr: &str) -> String {
    lower_in(expr, &Ctx::default())
}

pub fn lower_in(expr: &str, ctx: &Ctx) -> String {
    let mut p = Parser { toks: lex(expr), pos: 0, ctx: ctx.clone() };
    let out = p.or();
    // Anything left over means the expression used syntax outside the grammar.
    // Emit what we have rather than inventing: the diagnostic already fired.
    if p.pos < p.toks.len() {
        return expr.trim().to_string();
    }
    out
}

/// Interpolate a prose string containing `{expr}` into a JSX-safe expression list.
///
/// `Tasks — {tasks.open.count} open` becomes
/// `Tasks — {tasks.filter((it) => !it.done).length} open`, which is valid JSX text
/// with an embedded expression.
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
/// `aria="Delete {title}"` lowered as text yields `aria-label="Delete {item.title}"`
/// — and inside quotes JSX has no expression syntax, so the braces reach the DOM
/// verbatim and the accessible name reads "Delete {item.title}". A binding therefore
/// forces the whole value into a template literal instead.
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
                // Unterminated: keep the text rather than emitting broken JS. The
                // parser has already reported the brace.
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
/// interpolation. `{` is escaped rather than only `${` because the `$` and the `{` can
/// arrive from different segments: `a${x` with no closing brace would otherwise emit
/// `` `a${x` `` — an unterminated interpolation, and a syntax error. `\{` is a
/// NonEscapeCharacter, so it is legal and means a literal brace.
fn escape_template(s: &str) -> String {
    s.replace('\\', "\\\\").replace('`', "\\`").replace('{', "\\{")
}

// ---------------------------------------------------------------- lexer

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(String),
    Str(String),
    Ident(String),
    Op(String),
}

fn lex(src: &str) -> Vec<Tok> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'"' || c == b'\'' {
            let quote = c;
            let start = i + 1;
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            out.push(Tok::Str(src[start..i.min(src.len())].to_string()));
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            out.push(Tok::Num(src[start..i].to_string()));
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$' || bytes[i] == b'.')
            {
                i += 1;
            }
            out.push(Tok::Ident(src[start..i].to_string()));
            continue;
        }
        let two = &src[i..(i + 2).min(src.len())];
        if matches!(two, "==" | "!=" | "<=" | ">=" | "&&" | "||") {
            out.push(Tok::Op(two.to_string()));
            i += 2;
            continue;
        }
        out.push(Tok::Op((c as char).to_string()));
        i += 1;
    }
    out
}

// ---------------------------------------------------------------- parser

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    ctx: Ctx,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn eat_op(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Op(o)) if o == op) {
            self.pos += 1;
            return true;
        }
        false
    }

    fn or(&mut self) -> String {
        let mut left = self.and();
        while self.eat_op("||") {
            left = format!("{left} || {}", self.and());
        }
        left
    }

    fn and(&mut self) -> String {
        let mut left = self.cmp();
        while self.eat_op("&&") {
            left = format!("{left} && {}", self.cmp());
        }
        left
    }

    fn cmp(&mut self) -> String {
        let left = self.add();
        for op in ["==", "!=", "<=", ">=", "<", ">"] {
            if self.eat_op(op) {
                // GUML's `==` is JS's `===`; there is no loose equality in the language.
                let js = match op {
                    "==" => "===",
                    "!=" => "!==",
                    other => other,
                };
                return format!("{left} {js} {}", self.add());
            }
        }
        left
    }

    fn add(&mut self) -> String {
        let mut left = self.mul();
        loop {
            if self.eat_op("+") {
                left = format!("{left} + {}", self.mul());
            } else if self.eat_op("-") {
                left = format!("{left} - {}", self.mul());
            } else {
                return left;
            }
        }
    }

    fn mul(&mut self) -> String {
        let mut left = self.unary();
        loop {
            if self.eat_op("*") {
                left = format!("{left} * {}", self.unary());
            } else if self.eat_op("/") {
                left = format!("{left} / {}", self.unary());
            } else if self.eat_op("%") {
                left = format!("{left} % {}", self.unary());
            } else {
                return left;
            }
        }
    }

    fn unary(&mut self) -> String {
        if self.eat_op("!") {
            return format!("!{}", self.unary());
        }
        if self.eat_op("-") {
            return format!("-{}", self.unary());
        }
        self.primary()
    }

    fn primary(&mut self) -> String {
        if self.eat_op("(") {
            let inner = self.or();
            self.eat_op(")");
            return format!("({inner})");
        }

        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                n
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                format!("{s:?}")
            }
            Some(Tok::Ident(path)) => {
                self.pos += 1;
                // A trailing `()` is how `.trim()` is written; the call is implied
                // by the aggregate, so drop the parentheses.
                if self.eat_op("(") {
                    self.eat_op(")");
                }
                lower_path(&path, &self.ctx)
            }
            _ => String::new(),
        }
    }
}

/// Lower a dotted path, expanding aggregates into JS.
fn lower_path(path: &str, ctx: &Ctx) -> String {
    let mut parts = path.split('.');
    let Some(head) = parts.next() else { return path.to_string() };

    let mut out = match head {
        "true" | "false" | "null" => head.to_string(),
        _ => ctx.qualify(head).unwrap_or_else(|| head.to_string()),
    };

    // Whether what we have so far is an array, which decides how a bare field followed by
    // `.count` is read.
    let mut collection = ctx.collections.iter().any(|c| c == head);
    let mut parts = parts.peekable();

    while let Some(part) = parts.next() {
        match Aggregate::parse(part) {
            Some(Aggregate::Count) => {
                out = format!("{out}.length");
                collection = false;
            }
            // `it` rather than a single letter: the emitted code is read by people.
            Some(Aggregate::Open) => out = format!("{out}.filter((it) => !it.done)"),
            Some(Aggregate::Done) => out = format!("{out}.filter((it) => it.done)"),
            Some(Aggregate::Sum) => {
                out = format!("{out}.reduce((a, b) => a + Number(b), 0)");
                collection = false;
            }
            Some(Aggregate::Trim) => out = format!("{out}.trim()"),
            Some(Aggregate::Lower) => out = format!("{out}.toLowerCase()"),
            Some(Aggregate::Upper) => out = format!("{out}.toUpperCase()"),
            // `projects.live.count` — a field of the row, then an aggregate over it. Only
            // applied to a known array, so `user.name.count` still means string length.
            None if collection
                && parts.peek().is_some_and(|next| {
                    matches!(Aggregate::parse(next), Some(Aggregate::Count) | Some(Aggregate::Sum))
                }) =>
            {
                out = match Aggregate::parse(parts.peek().copied().unwrap_or("")) {
                    Some(Aggregate::Sum) => format!("{out}.map((it) => it.{part})"),
                    _ => format!("{out}.filter((it) => it.{part})"),
                };
            }
            None => {
                out = format!("{out}.{part}");
                collection = false;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_paths_pass_through() {
        assert_eq!(lower("count"), "count");
        assert_eq!(lower("task.title"), "task.title");
    }

    #[test]
    fn aggregates_become_real_javascript() {
        // This is the case that produced invalid output before: `.open.count` is
        // not JS, and it appears in the very first line of the task fixture.
        assert_eq!(lower("tasks.open.count"), "tasks.filter((it) => !it.done).length");
        assert_eq!(lower("tasks.done.count"), "tasks.filter((it) => it.done).length");
        assert_eq!(lower("tasks.count"), "tasks.length");
    }

    #[test]
    fn string_helpers() {
        assert_eq!(lower("draft.trim()"), "draft.trim()");
        assert_eq!(lower("!draft.trim()"), "!draft.trim()");
        assert_eq!(lower("title.lower"), "title.toLowerCase()");
    }

    #[test]
    fn operators_and_precedence() {
        assert_eq!(lower("count > 0"), "count > 0");
        assert_eq!(lower("!count"), "!count");
        assert_eq!(lower("a + b * c"), "a + b * c");
        assert_eq!(lower("(a + b) * c"), "(a + b) * c");
    }

    #[test]
    fn equality_becomes_strict() {
        assert_eq!(lower("filter == \"open\""), "filter === \"open\"");
        assert_eq!(lower("filter != \"done\""), "filter !== \"done\"");
    }

    #[test]
    fn booleans_chain() {
        assert_eq!(lower("done && count > 0"), "done && count > 0");
        assert_eq!(lower("a || b"), "a || b");
    }

    #[test]
    fn prose_interpolation() {
        assert_eq!(
            lower_text("Tasks — {tasks.open.count} open"),
            "Tasks — {tasks.filter((it) => !it.done).length} open"
        );
        assert_eq!(lower_text("no bindings here"), "no bindings here");
        assert_eq!(lower_text("{count}"), "{count}");
    }

    #[test]
    fn unbalanced_braces_are_left_alone() {
        assert_eq!(lower_text("half {open"), "half {open");
    }

    #[test]
    fn item_fields_are_qualified_inside_a_row() {
        let ctx = Ctx::item(&["title".to_string(), "done".to_string()]);
        assert_eq!(lower_in("title", &ctx), "item.title");
        assert_eq!(lower_in("!done", &ctx), "!item.done");
        // Names that are not fields of the row are left alone.
        assert_eq!(lower_in("filter", &ctx), "filter");
        assert_eq!(
            lower_text_in("Delete {title}", &ctx),
            "Delete {item.title}"
        );
    }

    #[test]
    fn attribute_values_switch_to_a_template_literal_when_bound() {
        let ctx = Ctx::item(&["title".to_string()]);
        // No binding: an ordinary quoted string.
        assert_eq!(lower_attr_value_in("Delete", &ctx), "\"Delete\"");
        // Bound: braces cannot survive inside JSX quotes.
        assert_eq!(lower_attr_value_in("Delete {title}", &ctx), "{`Delete ${item.title}`}");
        assert_eq!(lower_attr_value_in("{title}", &ctx), "{`${item.title}`}");
    }

    #[test]
    fn template_literal_escaping_cannot_break_out_of_the_literal() {
        let ctx = Ctx::item(&["title".to_string()]);
        // A backtick would otherwise end the literal.
        assert_eq!(lower_attr_value_in("a`b {title}", &ctx), "{`a\\`b ${item.title}`}");
        // `{x}` is a binding in GUML even behind a `$`, so the `$` stays literal text
        // and the binding interpolates normally.
        assert_eq!(
            lower_attr_value_in("cost ${x} for {title}", &ctx),
            "{`cost $${x} for ${item.title}`}"
        );
        // An unterminated brace must not become an interpolation. The `$` and the `{`
        // arrive from different segments, which is why `{` is escaped and not `${`.
        assert_eq!(lower_attr_value_in("a${x", &ctx), "{`a$\\{x`}");
    }

    #[test]
    fn syntax_outside_the_grammar_is_returned_verbatim() {
        // The resolver reports it; codegen must not invent a lowering.
        let weird = "a ? b : c";
        assert_eq!(lower(weird), weird);
    }
}
