//! GUML abstract syntax tree.
//!
//! Every node carries a `Span`. This is not optional: spans are what make diagnostics
//! machine-actionable, and diagnostics are the LLM repair loop's only input (report §6.7).
//!
//! The AST is `Serialize` so `guml ast --json` can dump it. That output is the contract for
//! external tooling and for the benchmark harness, which compares ASTs rather than text when
//! measuring generation consistency (report §8.3, metric: inter-run variance).

use guml_diagnostics::Span;
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Program {
    pub page: Option<PageDecl>,
    pub types: Vec<TypeDecl>,
    pub states: Vec<StateDecl>,
    pub resources: Vec<Resource>,
    /// Top-level element tree.
    pub tree: Vec<Element>,
}

impl Program {
    pub fn state(&self, name: &str) -> Option<&StateDecl> {
        self.states.iter().find(|s| s.name == name)
    }

    /// Depth-first walk over every element in the tree.
    pub fn walk(&self, mut f: impl FnMut(&Element)) {
        fn go(els: &[Element], f: &mut impl FnMut(&Element)) {
            for e in els {
                f(e);
                go(&e.children, f);
            }
        }
        go(&self.tree, &mut f);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PageDecl {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeDecl {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub name: String,
    /// `string` when omitted — types exist to catch model mistakes and drive codegen, not to
    /// prove soundness (report §5.7).
    pub ty: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateDecl {
    pub name: String,
    pub init: Value,
    /// `state filter=all|open|done` — an enumerated domain, used for exhaustiveness checks
    /// and to generate a segmented control instead of a free input.
    pub domain: Vec<String>,
    pub span: Span,
}

/// `data tasks:Task[] GET /api/tasks` plus its mutations.
#[derive(Debug, Clone, Serialize)]
pub struct Resource {
    pub name: String,
    pub ty: String,
    pub method: String,
    pub url: String,
    pub mutations: Vec<Mutation>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct Mutation {
    pub name: String,
    pub method: String,
    pub url: String,
    /// Field names taken from state/scope and sent as the JSON body.
    pub body: Vec<String>,
    /// `optimistic`, `optimistic:prepend`, … — drives generated rollback code.
    pub optimistic: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct Element {
    pub tag: String,
    pub positionals: Vec<Positional>,
    pub attrs: Vec<Attr>,
    /// Raw action bodies (`count++`, `tasks.add{title:draft}; draft=""`). Parsed by the
    /// action lowering pass, not here.
    pub actions: Vec<String>,
    /// Prose content for text-kind tags, or the right-hand side of a `|` separator.
    pub content: Option<String>,
    pub children: Vec<Element>,
    /// Raw child lines for tags whose children are content rather than elements (`tier`
    /// perks, `faq` question/answer pairs). Keeps those blocks at ~1 token of overhead per
    /// line instead of requiring a wrapper tag on each.
    pub text_lines: Vec<String>,
    pub span: Span,
}

impl Element {
    pub fn new(tag: impl Into<String>, span: Span) -> Self {
        Self {
            tag: tag.into(),
            positionals: Vec::new(),
            attrs: Vec::new(),
            actions: Vec::new(),
            content: None,
            children: Vec::new(),
            text_lines: Vec::new(),
            span,
        }
    }

    pub fn attr(&self, name: &str) -> Option<&Value> {
        self.attrs.iter().find(|a| a.name == name).map(|a| &a.value)
    }

    pub fn has_modifier(&self, m: &str) -> bool {
        self.positionals.iter().any(|p| matches!(p, Positional::Modifier(x) if x == m))
    }

    pub fn modifiers(&self) -> impl Iterator<Item = &str> {
        self.positionals.iter().filter_map(|p| match p {
            Positional::Modifier(m) => Some(m.as_str()),
            _ => None,
        })
    }

    /// First label-ish positional: the button text, the card title, and so on.
    pub fn label(&self) -> Option<&str> {
        self.positionals.iter().find_map(|p| match p {
            Positional::Text(t) => Some(t.as_str()),
            _ => None,
        })
    }

    pub fn route(&self) -> Option<&str> {
        self.positionals.iter().find_map(|p| match p {
            Positional::Route(r) => Some(r.as_str()),
            _ => None,
        })
    }

    pub fn anchor(&self) -> Option<&str> {
        self.positionals.iter().find_map(|p| match p {
            Positional::Anchor(a) => Some(a.as_str()),
            _ => None,
        })
    }

    pub fn binding(&self) -> Option<&str> {
        self.positionals.iter().find_map(|p| match p {
            Positional::Binding(b) => Some(b.as_str()),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Positional {
    /// A label or free word that is not a known modifier.
    Text(String),
    /// A registry-known modifier (`primary`, `ghost`, `sm`, `center`).
    Modifier(String),
    /// `{expr}`
    Binding(String),
    /// `/signup`
    Route(String),
    /// `#features`
    Anchor(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Attr {
    pub name: String,
    pub value: Value,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
    /// `{expr}` — kept as source text; the expression language is lowered separately.
    Binding(String),
    /// Bare word (`prepend`, `all`).
    Word(String),
    /// Present-but-valueless attribute (`featured`).
    Flag,
}

impl Value {
    /// JS/JSX literal rendering. Bindings render as bare expressions so the caller decides
    /// whether to wrap in `{}`.
    pub fn to_js(&self) -> String {
        match self {
            Value::Str(s) => format!("{:?}", s),
            Value::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Binding(b) => b.clone(),
            Value::Word(w) => format!("{:?}", w),
            Value::Flag => "true".to_string(),
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Str(s) | Value::Word(s) => Some(s),
            _ => None,
        }
    }
}
