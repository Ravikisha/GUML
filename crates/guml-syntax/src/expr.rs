//! The expression language, parsed properly.
//!
//! # What was wrong with the old arrangement
//!
//! `{tasks.open.count}` was stored as a *string* and re-parsed by whoever needed it. That meant
//! the grammar existed twice in Rust — once inside the React backend, once nowhere at all for
//! the validator, which pattern-matched on substrings — and a third time in TypeScript for the
//! browser runtime. Three implementations of one grammar is how a preview starts disagreeing
//! with emitted code.
//!
//! Now there is one parser. It produces an [`Expr`] tree, reports bad syntax through the normal
//! diagnostic channel with a real span, and both the code generator and the validator consume
//! the tree rather than the text.
//!
//! # Why the AST still stores the source text
//!
//! Deliberate. The formatter round-trips documents byte for byte where it can, and the
//! authoritative text of a binding is what the author wrote. Parsing happens at the boundaries
//! that need structure. The cost is bounded — `check` on 200 lines measures 1.77 ms including
//! this — and the alternative is a second source of truth for the same characters.
//!
//! # The grammar
//!
//! Deliberately small (report §5.9): paths with aggregates, comparisons, boolean and arithmetic
//! operators, literals, and prefix `!`/`-`. No calls beyond the fixed aggregate set, no
//! indexing, no lambdas. Anything else belongs in a `js` block, and that boundary is also the
//! security boundary for rendering a document an untrusted agent produced.

use crate::{Code, Diagnostic, Diagnostics, Span};
use serde::Serialize;

/// A path segment that means something to the compiler rather than being a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Aggregate {
    /// `.count` / `.length`
    Count,
    /// `.open` — rows whose `done` is false
    Open,
    /// `.done`
    Done,
    Sum,
    Trim,
    Lower,
    Upper,
}

impl Aggregate {
    pub fn parse(s: &str) -> Option<Self> {
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

/// One step along a path: a named field, or an aggregate over what precedes it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Step {
    Field(String),
    Agg(Aggregate),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BinOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
}

impl BinOp {
    pub fn as_js(self) -> &'static str {
        match self {
            BinOp::Eq => "===",
            BinOp::Ne => "!==",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }

    /// Binding power. Comparison binds looser than arithmetic, `&&` looser than comparison,
    /// `||` loosest — the precedence every C-family language has, because a document author's
    /// expectations come from those languages and not from this one.
    fn power(self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 3,
            BinOp::Add | BinOp::Sub => 4,
            BinOp::Mul | BinOp::Div => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Expr {
    /// `count`, `tasks.open.count`, `draft.trim()`
    Path {
        head: String,
        steps: Vec<Step>,
    },
    Num(f64),
    Str(String),
    Bool(bool),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Bin {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// Syntax the grammar does not cover. Kept rather than discarded so a consumer can fall
    /// back to the author's own text instead of emitting something invented.
    Unknown(String),
}

impl Expr {
    /// The leading identifier, which is the name a resolver has to check.
    pub fn head_ident(&self) -> Option<&str> {
        match self {
            Expr::Path { head, .. } => Some(head),
            Expr::Not(inner) | Expr::Neg(inner) => inner.head_ident(),
            Expr::Bin { lhs, .. } => lhs.head_ident(),
            _ => None,
        }
    }

    /// Every identifier the expression reads, for use-tracking and scope checks.
    pub fn idents(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect_idents(&mut out);
        out
    }

    fn collect_idents<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Expr::Path { head, .. } => out.push(head),
            Expr::Not(inner) | Expr::Neg(inner) => inner.collect_idents(out),
            Expr::Bin { lhs, rhs, .. } => {
                lhs.collect_idents(out);
                rhs.collect_idents(out);
            }
            _ => {}
        }
    }

    /// Whether the expression computes rather than merely reading a name. Used to tell a
    /// derived value from a plain binding.
    pub fn is_computed(&self) -> bool {
        match self {
            Expr::Path { steps, .. } => steps.iter().any(|s| matches!(s, Step::Agg(_))),
            Expr::Bin { .. } | Expr::Not(_) | Expr::Neg(_) => true,
            _ => false,
        }
    }
}

/// Parse an expression. Never fails: unrecognised syntax becomes [`Expr::Unknown`] and, when a
/// span is supplied, a diagnostic.
pub fn parse(src: &str) -> Expr {
    let mut p = Parser { toks: lex(src), pos: 0, src };
    let out = p.expr(0);
    if p.pos < p.toks.len() { Expr::Unknown(src.trim().to_string()) } else { out }
}

/// Parse and report. `span` is the binding's own span, so the diagnostic points at the braces
/// rather than at the whole line.
pub fn parse_reported(src: &str, span: Span, diags: &mut Diagnostics) -> Expr {
    let out = parse(src);
    report_unknown(&out, span, diags);
    out
}

/// Report an expression that is already parsed.
///
/// Bindings are parsed once, when the document is parsed, so the validator has the tree and must
/// not parse the text again — re-parsing was the duplication this whole change removes.
pub fn report_unknown(expr: &Expr, span: Span, diags: &mut Diagnostics) {
    if let Expr::Unknown(text) = expr {
        diags.push(
            Diagnostic::error(
                Code::BadExpression,
                format!("`{text}` is not an expression GUML can read"),
                span,
            )
            .with_help(
                "the expression language covers paths, comparisons, arithmetic and the \
                 aggregates `count`, `sum`, `open`, `done`, `trim`, `lower`, `upper`",
            ),
        );
    }
}

/// The `{…}` groups inside a prose line.
///
/// Content is stored as raw text in the AST — prose is taken verbatim, which is what makes it
/// almost free — so an interpolation has no pre-parsed tree and every consumer has to find it the
/// same way. One implementation here rather than one per consumer: there were three.
pub fn interpolations(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                out.push(&after[..close]);
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    out
}

// ------------------------------------------------------------------- lexer

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
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
            match src[start..i].parse::<f64>() {
                Ok(n) => out.push(Tok::Num(n)),
                // A malformed number becomes an operator token, which fails the parse and is
                // reported rather than silently rounded.
                Err(_) => out.push(Tok::Op(src[start..i].to_string())),
            }
            continue;
        }

        if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
            {
                i += 1;
            }
            out.push(Tok::Ident(src[start..i].to_string()));
            continue;
        }

        // Two-character operators first, so `<=` never lexes as `<` then `=`.
        let two = src.get(i..i + 2).unwrap_or("");
        if matches!(two, "==" | "!=" | "<=" | ">=" | "&&" | "||" | "()") {
            out.push(Tok::Op(two.to_string()));
            i += 2;
            continue;
        }

        out.push(Tok::Op((c as char).to_string()));
        i += 1;
    }

    out
}

// ------------------------------------------------------------------ parser

struct Parser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    src: &'a str,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn bin_op(&self) -> Option<BinOp> {
        let Some(Tok::Op(op)) = self.peek() else { return None };
        Some(match op.as_str() {
            "==" => BinOp::Eq,
            "!=" => BinOp::Ne,
            "<" => BinOp::Lt,
            "<=" => BinOp::Le,
            ">" => BinOp::Gt,
            ">=" => BinOp::Ge,
            "+" => BinOp::Add,
            "-" => BinOp::Sub,
            "*" => BinOp::Mul,
            "/" => BinOp::Div,
            "&&" => BinOp::And,
            "||" => BinOp::Or,
            _ => return None,
        })
    }

    /// Precedence climbing. One loop instead of one function per level, so adding an operator
    /// is a table entry rather than a new rung.
    fn expr(&mut self, min_power: u8) -> Expr {
        let mut lhs = self.unary();

        while let Some(op) = self.bin_op() {
            let power = op.power();
            if power < min_power {
                break;
            }
            self.pos += 1;
            let rhs = self.expr(power + 1);
            lhs = Expr::Bin { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }

        lhs
    }

    fn unary(&mut self) -> Expr {
        match self.peek() {
            Some(Tok::Op(op)) if op == "!" => {
                self.pos += 1;
                Expr::Not(Box::new(self.unary()))
            }
            Some(Tok::Op(op)) if op == "-" => {
                self.pos += 1;
                Expr::Neg(Box::new(self.unary()))
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Expr {
        let tok = self.peek().cloned();
        match tok {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Expr::Num(n)
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Expr::Str(s)
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                // A trailing `()` is accepted so `draft.trim()` and `draft.trim` mean the same
                // thing; models write both and the difference carries no information.
                if matches!(self.peek(), Some(Tok::Op(op)) if op == "()") {
                    self.pos += 1;
                }
                match name.as_str() {
                    "true" => Expr::Bool(true),
                    "false" => Expr::Bool(false),
                    _ => path(&name),
                }
            }
            Some(Tok::Op(op)) if op == "(" => {
                self.pos += 1;
                let inner = self.expr(0);
                if matches!(self.peek(), Some(Tok::Op(op)) if op == ")") {
                    self.pos += 1;
                    inner
                } else {
                    Expr::Unknown(self.src.trim().to_string())
                }
            }
            _ => {
                // Consume so the caller cannot loop forever on an unexpected token.
                self.pos += 1;
                Expr::Unknown(self.src.trim().to_string())
            }
        }
    }
}

fn path(name: &str) -> Expr {
    let mut parts = name.split('.');
    let head = parts.next().unwrap_or("").to_string();
    let steps = parts
        .map(|p| match Aggregate::parse(p) {
            Some(a) => Step::Agg(a),
            None => Step::Field(p.to_string()),
        })
        .collect();
    Expr::Path { head, steps }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(src: &str) -> Expr {
        parse(src)
    }

    #[test]
    fn a_path_keeps_its_aggregates_apart_from_its_fields() {
        assert_eq!(
            p("tasks.open.count"),
            Expr::Path {
                head: "tasks".into(),
                steps: vec![Step::Agg(Aggregate::Open), Step::Agg(Aggregate::Count)],
            }
        );
        assert_eq!(
            p("user.name"),
            Expr::Path { head: "user".into(), steps: vec![Step::Field("name".into())] }
        );
    }

    #[test]
    fn call_parentheses_are_optional() {
        assert_eq!(p("draft.trim()"), p("draft.trim"));
    }

    #[test]
    fn precedence_follows_the_languages_people_already_know() {
        // 1 + 2 * 3, not (1 + 2) * 3.
        let e = p("1 + 2 * 3");
        let Expr::Bin { op: BinOp::Add, rhs, .. } = &e else { panic!("{e:?}") };
        assert!(matches!(**rhs, Expr::Bin { op: BinOp::Mul, .. }), "{rhs:?}");

        // Comparison binds looser than arithmetic.
        let e = p("count + 1 > 2");
        let Expr::Bin { op: BinOp::Gt, lhs, .. } = &e else { panic!("{e:?}") };
        assert!(matches!(**lhs, Expr::Bin { op: BinOp::Add, .. }));

        // `&&` looser than comparison, `||` loosest.
        let e = p("a == 1 && b || c");
        assert!(matches!(e, Expr::Bin { op: BinOp::Or, .. }), "{e:?}");
    }

    #[test]
    fn parentheses_override_precedence() {
        let e = p("(1 + 2) * 3");
        let Expr::Bin { op: BinOp::Mul, lhs, .. } = &e else { panic!("{e:?}") };
        assert!(matches!(**lhs, Expr::Bin { op: BinOp::Add, .. }));
    }

    #[test]
    fn prefix_operators() {
        assert_eq!(p("!done"), Expr::Not(Box::new(p("done"))));
        assert_eq!(p("-count"), Expr::Neg(Box::new(p("count"))));
        // The disabled-button idiom.
        assert_eq!(p("!draft.trim()"), Expr::Not(Box::new(p("draft.trim"))));
    }

    #[test]
    fn literals() {
        assert_eq!(p("42"), Expr::Num(42.0));
        assert_eq!(p("\"text\""), Expr::Str("text".into()));
        assert_eq!(p("true"), Expr::Bool(true));
        assert_eq!(p("false"), Expr::Bool(false));
    }

    #[test]
    fn syntax_outside_the_grammar_is_reported_not_guessed() {
        // The old behaviour was to pass this straight through into emitted JavaScript.
        assert_eq!(p("a ? b : c"), Expr::Unknown("a ? b : c".into()));
        assert_eq!(p("fetch(url)"), Expr::Unknown("fetch(url)".into()));

        let mut diags = Diagnostics::new();
        parse_reported("a ? b : c", Span::new(0, 9, 1, 1), &mut diags);
        assert_eq!(diags.items.len(), 1);
        assert_eq!(diags.items[0].id, "GUML0023");
    }

    #[test]
    fn a_valid_expression_reports_nothing() {
        let mut diags = Diagnostics::new();
        parse_reported("tasks.open.count > 0", Span::new(0, 20, 1, 1), &mut diags);
        assert!(diags.is_empty(), "{:?}", diags.items);
    }

    #[test]
    fn identifiers_are_collectable_for_scope_checks() {
        assert_eq!(p("tasks.open.count").head_ident(), Some("tasks"));
        assert_eq!(p("!draft.trim()").head_ident(), Some("draft"));
        let e = p("a + b * c");
        assert_eq!(e.idents(), vec!["a", "b", "c"]);
    }

    #[test]
    fn computed_is_distinguishable_from_a_plain_read() {
        assert!(!p("count").is_computed());
        assert!(!p("user.name").is_computed());
        assert!(p("tasks.open.count").is_computed());
        assert!(p("count + 1").is_computed());
        assert!(p("!done").is_computed());
    }

    #[test]
    fn interpolations_are_found_in_prose() {
        assert_eq!(interpolations("Tasks — {tasks.open.count} open"), vec!["tasks.open.count"]);
        assert_eq!(interpolations("a {x} b {y}"), vec!["x", "y"]);
        assert!(interpolations("no bindings here").is_empty());
        // Unterminated: stop rather than guess where it ended.
        assert!(interpolations("a {unclosed").is_empty());
    }

    #[test]
    fn parsing_terminates_on_hostile_input() {
        // A document may come from an untrusted agent, so an unbounded loop here is a denial of
        // service rather than a bug.
        for src in ["", "(", ")", "((((", "&&", "1 +", "a.", ".", "!!!", "\"unterminated"] {
            let _ = p(src);
        }
    }
}
