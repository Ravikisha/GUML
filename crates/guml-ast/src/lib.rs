//! GUML abstract syntax tree.
//!
//! Every node carries a `Span`. This is not optional: spans are what make diagnostics
//! machine-actionable, and diagnostics are the LLM repair loop's only input (report §6.7).
//!
//! The AST is `Serialize` so `guml ast --json` can dump it. That output is the contract for
//! external tooling and for the benchmark harness, which compares ASTs rather than text when
//! measuring generation consistency (report §8.3, metric: inter-run variance).

use guml_diagnostics::Span;
use guml_syntax::expr::Expr;
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Program {
    pub page: Option<PageDecl>,
    pub types: Vec<TypeDecl>,
    pub states: Vec<StateDecl>,
    pub resources: Vec<Resource>,
    /// User-defined components, in declaration order.
    ///
    /// Expanded by the compiler before codegen, so a backend never sees one — which is what lets a
    /// `def` work in every backend, including the no-JavaScript HTML one, with no per-backend support.
    pub defs: Vec<DefDecl>,
    /// `on mount` / `on {expr}` — effects the author *declares* rather than writes.
    ///
    /// The whole point is the absence of a dependency array. `useEffect(fn, [deps])` is wrong in two
    /// directions — a missing dep is a stale read, a spurious one is an infinite loop — and it is a
    /// bug a model produces readily, because the correct list is not derivable from the lines nearby.
    /// Here the dependency *is* the trigger, so there is no second list to keep in sync with the
    /// first.
    pub effects: Vec<Effect>,
    /// Top-level element tree.
    pub tree: Vec<Element>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Effect {
    pub trigger: Trigger,
    /// Raw action bodies, in the same form as `Element::actions`, lowered by the same pass. An effect
    /// that ran a *different* action language from a button's would be two things to learn.
    pub actions: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Trigger {
    /// Once, after the first render.
    Mount,
    /// Whenever the expression's value changes. Text, lowered like any other binding.
    Change(String),
}

/// `def stat label value` plus an indented body.
///
/// A compile-time macro, not a runtime component. That choice is what keeps "no imports, no
/// framework concepts" true: expansion happens in the compiler, so the emitted output looks exactly
/// as if the author had written the body inline.
#[derive(Debug, Clone, Serialize)]
pub struct DefDecl {
    pub name: String,
    /// Positional parameter names, in order. Referenced in the body as `{name}`, which resolves
    /// against the def's parameters first and the surrounding document second — the same shadowing
    /// rule a repeater's row fields already use.
    pub params: Vec<String>,
    pub body: Vec<Element>,
    pub span: Span,
}

impl Program {
    pub fn state(&self, name: &str) -> Option<&StateDecl> {
        self.states.iter().find(|s| s.name == name)
    }

    /// What a repeater iterates, and where its row type comes from.
    ///
    /// # The gap this exists to close
    ///
    /// A repeater's source had to be a **declared resource**, and that is the single most consequential
    /// limitation the GUML-Bench reference corpus turned up. It means only one client-side filter is ever
    /// expressible: `where=` takes one enumerated state, and there is no way to iterate a value computed
    /// from several. Composing the predicate in a `js` block computes the right *numbers* and cannot feed
    /// the `list`, because the list needs a resource — so `v01-event-filters` and `v02-cohort` both had to
    /// filter on the *server* and fail their own "one fetch, not one per change" criterion on purpose.
    ///
    /// `of=Type` closes it: the source is any name in scope and `of=` names the row type, so
    /// `list matches of=Event` iterates a `js`-computed array with `{name}` and `{country}` resolving
    /// against `Event`'s fields.
    ///
    /// # Why `of=` and not a new attribute
    ///
    /// `of` was already declared on `list`/`table` and read as an *alternative source name* — a fallback
    /// used by no fixture, no conformance case and no test. So this is a meaning change to an attribute
    /// nothing depended on, done before 1.0, and recorded in `spec/STABILITY.md` rather than slipped in.
    /// "Of" reading as "of what type" is also the reading a person gives it.
    pub fn repeater_rows(&self, el: &Element) -> Option<RepeaterRows> {
        if !matches!(el.tag.as_str(), "list" | "table") {
            return None;
        }
        let source = el.label()?.to_string();
        // A resource brings its own row type; that path is unchanged and stays the common case.
        if let Some(r) = self.resources.iter().find(|r| r.name == source) {
            let ty = r.ty.trim_end_matches("[]").to_string();
            return Some(RepeaterRows { source, ty, from_resource: true });
        }
        // Otherwise the document has to say, because nothing else can: a `js` block's array has no
        // declared element type and the compiler does not read the block.
        let ty = el.attr("of").and_then(|v| v.as_text())?.to_string();
        Some(RepeaterRows { source, ty, from_resource: false })
    }

    /// The field names of a repeater's row type, or empty when the type is not declared.
    pub fn repeater_fields(&self, el: &Element) -> Vec<String> {
        let Some(rows) = self.repeater_rows(el) else { return Vec::new() };
        self.types
            .iter()
            .find(|t| t.name == rows.ty)
            .map(|t| t.fields.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default()
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

/// What a repeater iterates. See [`Program::repeater_rows`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeaterRows {
    /// The name being iterated: a resource, or any other array in scope.
    pub source: String,
    /// The declared type of one row.
    pub ty: String,
    /// Whether the source is a declared resource, and therefore brings a fetch, loading and error state
    /// with it. A derived array has none of that and must not be given the scaffolding — emitting
    /// `matchesLoading` for a `js` const would reference a name that does not exist.
    pub from_resource: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageDecl {
    pub name: String,
    /// Document metadata, from attributes on the `page` line.
    ///
    /// A markup language emits documents, and a document has a title, a language and a text
    /// direction. `page Name` gives the *component* a name — useful to a backend, no use at all to a
    /// browser, a search engine or a screen reader. Without `lang` the whole output is an
    /// accessibility and i18n hole: assistive technology guesses the pronunciation.
    pub meta: PageMeta,
    pub span: Span,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PageMeta {
    /// Human-facing document title. Falls back to the page name when absent.
    pub title: Option<String>,
    pub description: Option<String>,
    /// BCP 47 language tag. Defaults to `en` at emit time rather than here, so a backend can tell
    /// "the author said `en`" from "the author said nothing".
    pub lang: Option<String>,
    /// `ltr` or `rtl`.
    pub dir: Option<String>,
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
    /// `js` and `raw` escape hatches. Their `text_lines` are not GUML, so every pass that reads
    /// prose has to ask this before reading it: checking the body would report errors against code
    /// the compiler explicitly promised not to look at.
    ///
    /// Recording *uses* is still correct — `raw react` with `data={rows}` really does use `rows`,
    /// and skipping it would produce a false "declared but never used".
    pub fn is_escape(&self) -> bool {
        self.tag == "js" || self.tag == "raw"
    }

    /// Names a `js` block declares at its top level, which bindings elsewhere in the document may read.
    ///
    /// # Why the escape hatch is allowed to introduce a name
    ///
    /// Without this the hatch is a dead end rather than an escape. The case that showed it: the sum of a
    /// *computed* per-row value — a cart subtotal, `Σ unitPrice × quantity` — is not expressible as a
    /// binding, because an aggregate applies to a field and not to an expression. So the answer should
    /// have been "drop into `js` and count it", which is what the spec tells a generator to do. It did not
    /// work: a `js` block could compute `subtotal` and no binding could read it (`GUML0033`), leaving
    /// `raw <backend>` — verbatim markup for one backend, skipped by the rest — as the only route. A
    /// document forced to choose one backend has given up the property that makes GUML an IR.
    ///
    /// **This does not check the body.** Nothing here parses JavaScript; it reads the declaration keyword
    /// and the name that follows, and it is deliberately conservative — indented lines are skipped, so a
    /// `const` inside a function body or a block does not leak out. A name it misses costs a `GUML0033`
    /// the author can work around; a name it invents would put an undeclared identifier into scope.
    ///
    /// `raw` is excluded on purpose. A `raw` body is markup for one backend, not component-body code, and
    /// every other backend drops it — so a binding depending on a name from it would compile in one
    /// target and be undefined in the others.
    pub fn escape_declares(&self) -> Vec<String> {
        if self.tag != "js" {
            return Vec::new();
        }
        let mut out = Vec::new();
        for line in &self.text_lines {
            // Top level only. `text_lines` keeps a `js` body's own indentation, so a nested `const` is
            // still indented relative to the block and is not in the component's scope.
            if line.starts_with([' ', '\t']) {
                continue;
            }
            let mut words = line.split_whitespace();
            let Some(keyword) = words.next() else { continue };
            if !matches!(keyword, "const" | "let" | "var" | "function" | "async") {
                continue;
            }
            // `async function name(…)`.
            let name = if keyword == "async" {
                match words.next() {
                    Some("function") => words.next(),
                    _ => None,
                }
            } else {
                words.next()
            };
            let Some(name) = name else { continue };
            // `const [a, b] = …` and `const {a} = …` are destructurings this does not try to read, and
            // `name(` is a function. Take the leading identifier or nothing.
            let name: String = name
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
                .collect();
            if !name.is_empty() {
                out.push(name);
            }
        }
        out
    }

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

/// A `{…}` binding: the author's text *and* the parsed expression.
///
/// Both, deliberately. The expression is parsed once at parse time, so the code generator and the
/// validator share one tree instead of re-parsing a string — that duplication is what let
/// `{a ? b : c}` reach emitted JavaScript. The text is kept because it is what the author wrote,
/// and the formatter round-trips documents byte for byte where it can; regenerating a binding from
/// its tree would silently rewrite spacing inside it.
#[derive(Debug, Clone, Serialize)]
pub struct Binding {
    /// Between the braces, exactly as written.
    pub source: String,
    pub expr: Expr,
}

impl Binding {
    pub fn new(source: impl Into<String>) -> Self {
        let source = source.into();
        let expr = guml_syntax::expr::parse(&source);
        Self { source, expr }
    }

    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// The leading identifier, which is the name a resolver checks.
    pub fn head_ident(&self) -> Option<&str> {
        self.expr.head_ident()
    }
}

impl From<&str> for Binding {
    fn from(s: &str) -> Self {
        Binding::new(s)
    }
}

/// Compared by source text: two bindings that read the same are the same binding, and the parsed
/// tree is derived from it.
impl PartialEq for Binding {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Eq for Binding {}

impl std::fmt::Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Positional {
    /// A label or free word that is not a known modifier.
    Text(String),
    /// A registry-known modifier (`primary`, `ghost`, `sm`, `center`).
    Modifier(String),
    /// `{expr}`
    Binding(Binding),
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
    /// `{expr}`, parsed at parse time.
    Binding(Binding),
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
            Value::Binding(b) => b.source.clone(),
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

/// Every declaration name the document *refers to*, anywhere: bindings, positionals, prose
/// interpolations, action bodies, and escape-hatch bodies.
///
/// # Why this lives in the AST crate
///
/// Two consumers need the same answer and must not be allowed to disagree. `guml-compiler` reports
/// `GUML0074`/`GUML0075` ("declared but never used") from it, and `guml-codegen` uses it to decide
/// what *not* to emit. If codegen's idea of "referenced" were narrower than the validator's — a
/// second walker, written later, missing one of the reference forms — it would elide a declaration
/// the emitted code still mentions, and the output would not compile. The duplicated expression
/// grammar caused exactly that class of bug before it was unified.
///
/// States and resources share one set because every reference form is ambiguous between them at
/// this stage: `list tasks` and `input draft` are the same shape. Reporting is per-kind, but the
/// *evidence* is not.
///
/// Deliberately over-approximate. A name that is only mentioned in a `js` body counts, as does a
/// word that merely looks like an identifier. Over-approximating suppresses a warning and keeps a
/// declaration; under-approximating deletes code the output depends on.
pub fn referenced_names(program: &Program) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for el in &program.tree {
        walk_references(el, &mut out);
    }
    // An effect is a use in both halves. `on {filter} >tasks.list` reads `filter` and calls `tasks`,
    // and missing either would report a declaration the document plainly depends on as dead — then the
    // optimizer would delete it.
    for e in &program.effects {
        if let Trigger::Change(expr) = &e.trigger {
            for b in guml_syntax::expr::interpolations(&format!("{{{expr}}}")) {
                let head = head_ident(b);
                if !head.is_empty() {
                    out.insert(head.to_string());
                }
            }
            let head = head_ident(expr);
            if !head.is_empty() {
                out.insert(head.to_string());
            }
        }
        note_actions(&e.actions, &mut out);
    }
    out
}

/// `tasks.open.count` → `tasks`; `!draft.trim()` → `draft`.
///
/// Only `!` and `(` are stripped, not every leading non-identifier character: `"a literal"` must
/// yield nothing rather than `a`, or a string's contents would register as a reference.
pub fn head_ident(expr: &str) -> &str {
    let expr = expr.trim().trim_start_matches(['!', '(']);
    let end = expr.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(expr.len());
    &expr[..end]
}

fn walk_references(el: &Element, out: &mut std::collections::HashSet<String>) {
    let mut note = |expr: &str| {
        let head = head_ident(expr);
        if !head.is_empty() {
            out.insert(head.to_string());
        }
    };

    // A `js`/`raw` body is another language, so a reference in it need not be a `{binding}`:
    // `const q = month === "q1"` uses `month`. Nothing here parses the body — every
    // identifier-ish word counts.
    if el.is_escape() {
        for line in &el.text_lines {
            for word in line.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
                if !word.is_empty() {
                    out.insert(word.to_string());
                }
            }
        }
        return;
    }

    for p in &el.positionals {
        match p {
            Positional::Binding(b) => note(&b.source),
            // `input draft` and `list tasks` name a declaration in a positional slot.
            Positional::Text(t) => note(t),
            _ => {}
        }
    }
    for a in &el.attrs {
        if let Value::Binding(b) = &a.value {
            note(&b.source);
        }
    }
    for text in el.content.iter().chain(el.text_lines.iter()) {
        for b in guml_syntax::expr::interpolations(text) {
            note(b);
        }
    }
    note_actions(&el.actions, out);

    for child in &el.children {
        walk_references(child, out);
    }
}

/// Names an action body reads. Shared by elements and effects, because `>tasks.list` means the same
/// thing on a button and on an `on` line — and a second copy of this would be a second chance to
/// disagree about it.
fn note_actions(actions: &[String], out: &mut std::collections::HashSet<String>) {
    let mut note = |expr: &str| {
        let head = head_ident(expr);
        if !head.is_empty() {
            out.insert(head.to_string());
        }
    };
    for action in actions {
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
