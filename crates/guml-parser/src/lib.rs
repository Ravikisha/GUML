//! GUML parser.
//!
//! Two properties are non-negotiable (report §6.3):
//!
//! * **Error recovery.** The input was produced by a model, so the parser collects *all*
//!   errors in one pass. A parser that reports one error per invocation turns a 1-round repair
//!   loop into an N-round one, and each round is a full LLM generation.
//! * **Registry awareness.** Whether a line's remainder is prose or structure depends on the
//!   tag's `TagKind`, so parsing and resolution are interleaved rather than sequential.
//!
//! Layout comes from the indent stack: an element's children are the following lines with a
//! strictly greater indent. Blank lines and comments were already dropped by the lexer.

use guml_ast::{
    Attr, Element, Field, Mutation, PageDecl, Positional, Program, Resource, StateDecl, TypeDecl,
    Value,
};
use guml_diagnostics::{Code, Diagnostic, Diagnostics, Span};
use guml_registry::{Registry, TagKind};
use guml_syntax::{Line, Tok, Token};

pub struct Parsed {
    pub program: Program,
    pub diagnostics: Diagnostics,
}

pub fn parse(src: &str, reg: &Registry) -> Parsed {
    let lexed = guml_syntax::lex(src);
    let mut p = Parser {
        lines: lexed.lines,
        i: 0,
        reg,
        diagnostics: lexed.diagnostics,
        program: Program::default(),
    };
    p.parse_top();

    if p.program.page.is_none() {
        p.diagnostics.push(
            Diagnostic::warning(
                Code::MissingPageDirective,
                "file has no `page` directive",
                Span::point(0, 1, 1),
            )
            .with_help("start the file with `page <Name>` so the compiler can name the output"),
        );
    }

    Parsed { program: p.program, diagnostics: p.diagnostics }
}

struct Parser<'r> {
    lines: Vec<Line>,
    i: usize,
    reg: &'r Registry,
    diagnostics: Diagnostics,
    program: Program,
}

impl<'r> Parser<'r> {
    fn parse_top(&mut self) {
        while self.i < self.lines.len() {
            if self.lines[self.i].indent > 0 {
                let line = &self.lines[self.i];
                let span = line.span();
                self.diagnostics.push(
                    Diagnostic::error(Code::UnexpectedIndent, "unexpected indentation", span)
                        .with_help("this line is indented but has no parent element above it"),
                );
                self.i += 1;
                continue;
            }
            if self.try_directive() {
                continue;
            }
            if let Some(el) = self.parse_element() {
                self.program.tree.push(el);
            }
        }
    }

    // ---------------------------------------------------------------- directives

    fn try_directive(&mut self) -> bool {
        let line = self.lines[self.i].clone();
        let Some(first) = line.tokens.first().and_then(|t| t.tok.as_word()) else {
            return false;
        };
        match first {
            "page" => {
                self.i += 1;
                let name =
                    line.tokens.get(1).and_then(|t| t.tok.text()).unwrap_or("Page").to_string();
                self.program.page = Some(PageDecl { name, span: line.span() });
                true
            }
            "type" => {
                self.i += 1;
                self.parse_type(&line);
                true
            }
            "state" | "store" => {
                self.i += 1;
                self.parse_state(&line);
                true
            }
            "data" => {
                self.i += 1;
                self.parse_data(&line);
                true
            }
            _ => false,
        }
    }

    fn parse_type(&mut self, line: &Line) {
        let name = line.tokens.get(1).and_then(|t| t.tok.text()).unwrap_or("").to_string();
        let body = line.tokens.iter().find_map(|t| match &t.tok {
            Tok::Brace(b) => Some(b.clone()),
            _ => None,
        });
        let mut fields = Vec::new();
        if let Some(body) = body {
            for raw in body.split(',') {
                let raw = raw.trim();
                if raw.is_empty() {
                    continue;
                }
                let (fname, ty) = match raw.split_once(':') {
                    Some((n, t)) => (n.trim(), t.trim()),
                    None => (raw, "string"),
                };
                fields.push(Field { name: fname.to_string(), ty: ty.to_string() });
            }
        }
        self.program.types.push(TypeDecl { name, fields, span: line.span() });
    }

    fn parse_state(&mut self, line: &Line) {
        let name = line.tokens.get(1).and_then(|t| t.tok.text()).unwrap_or("").to_string();

        if self.program.states.iter().any(|s| s.name == name) {
            self.diagnostics.push(Diagnostic::error(
                Code::DuplicateState,
                format!("state `{name}` is declared more than once"),
                line.span(),
            ));
        }

        // Everything after `=`: first value is the initial value, `|`-separated words form an
        // enumerated domain.
        let mut init = Value::Str(String::new());
        let mut domain = Vec::new();
        let eq_at = line.tokens.iter().position(|t| matches!(t.tok, Tok::Eq));
        if let Some(eq) = eq_at {
            let rest = &line.tokens[eq + 1..];
            if rest.is_empty() {
                self.diagnostics.push(
                    Diagnostic::error(
                        Code::ExpectedValue,
                        "expected a value after `=`",
                        line.span(),
                    )
                    .with_help("e.g. `state count=0` or `state draft=\"\"`"),
                );
            }
            let mut first = true;
            for t in rest {
                match &t.tok {
                    Tok::Pipe => {}
                    _ => {
                        let v = token_value(&t.tok);
                        if first {
                            init = v.clone();
                            first = false;
                        }
                        if let Some(w) = v.as_text() {
                            domain.push(w.to_string());
                        }
                    }
                }
            }
            // A single value is not a domain, it is just an initial value.
            if !rest.iter().any(|t| matches!(t.tok, Tok::Pipe)) {
                domain.clear();
            }
        }

        self.program.states.push(StateDecl { name, init, domain, span: line.span() });
    }

    fn parse_data(&mut self, line: &Line) {
        let toks = &line.tokens;
        let name = toks.get(1).and_then(|t| t.tok.text()).unwrap_or("").to_string();

        let mut ty = String::new();
        let mut method = "GET".to_string();
        let mut url = String::new();
        let mut idx = 2;
        if matches!(toks.get(idx).map(|t| &t.tok), Some(Tok::Colon)) {
            idx += 1;
            ty = toks.get(idx).and_then(|t| t.tok.text()).unwrap_or("").to_string();
            idx += 1;
        }
        while idx < toks.len() {
            match &toks[idx].tok {
                Tok::Word(w) if is_http_method(w) => method = w.clone(),
                Tok::Route(r) => url = r.clone(),
                _ => {}
            }
            idx += 1;
        }

        // Indented children of a `data` block are mutations.
        let mut mutations = Vec::new();
        while self.i < self.lines.len() && self.lines[self.i].indent > line.indent {
            let m = self.lines[self.i].clone();
            self.i += 1;
            mutations.push(self.parse_mutation(&m));
        }

        self.program.resources.push(Resource {
            name,
            ty,
            method,
            url,
            mutations,
            span: line.span(),
        });
    }

    fn parse_mutation(&mut self, line: &Line) -> Mutation {
        let toks = &line.tokens;
        let name = toks.first().and_then(|t| t.tok.text()).unwrap_or("").to_string();
        let mut method = String::new();
        let mut url = String::new();
        let mut body = Vec::new();
        let mut optimistic = None;

        let mut i = 1;
        while i < toks.len() {
            match &toks[i].tok {
                Tok::Word(w) if is_http_method(w) => method = w.clone(),
                Tok::Route(r) => url = r.clone(),
                Tok::Brace(b) => {
                    body = b
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                Tok::Word(w) if w == "optimistic" => {
                    // `optimistic` or `optimistic:<strategy>`
                    if matches!(toks.get(i + 1).map(|t| &t.tok), Some(Tok::Colon)) {
                        let strat = toks
                            .get(i + 2)
                            .and_then(|t| t.tok.text())
                            .unwrap_or("replace")
                            .to_string();
                        optimistic = Some(strat);
                        i += 2;
                    } else {
                        optimistic = Some("replace".to_string());
                    }
                }
                _ => {}
            }
            i += 1;
        }

        if method.is_empty() {
            self.diagnostics.push(
                Diagnostic::error(
                    Code::ExpectedValue,
                    format!("mutation `{name}` has no HTTP method"),
                    line.span(),
                )
                .with_help("e.g. `add POST /api/tasks {title} optimistic:prepend`"),
            );
        }

        Mutation { name, method, url, body, optimistic, span: line.span() }
    }

    // ---------------------------------------------------------------- elements

    fn parse_element(&mut self) -> Option<Element> {
        let line = self.lines[self.i].clone();
        let base = line.indent;
        self.i += 1;

        let Some(tag) = line.tokens.first().and_then(|t| t.tok.as_word()).map(str::to_string)
        else {
            let span = line.tokens.first().map(|t| t.span).unwrap_or_else(|| line.span());
            self.diagnostics.push(
                Diagnostic::error(
                    Code::ExpectedTag,
                    "expected a tag name at the start of the line",
                    span,
                )
                .with_help("lines start with a tag, e.g. `card`, `btn`, `p`"),
            );
            self.skip_children(base);
            return None;
        };

        let known = self.reg.get(&tag);
        if known.is_none() {
            let span = line.tokens[0].span;
            let mut d = Diagnostic::error(Code::UnknownTag, format!("unknown tag `{tag}`"), span);
            if let Some(s) = self.reg.suggest(&tag) {
                d = d.with_help(format!("did you mean `{s}`?")).with_suggestion(s.to_string());
            } else {
                d = d.with_help("see the component registry for the available tags");
            }
            self.diagnostics.push(d);
        }
        let kind = known.map(|c| c.kind).unwrap_or(TagKind::Container);

        let mut el = Element::new(tag.clone(), line.span());
        self.fill_element(&mut el, &line, kind);

        // Children: following lines with a strictly greater indent.
        if self.reg.children_are_text(&tag) {
            while self.i < self.lines.len() && self.lines[self.i].indent > base {
                el.text_lines.push(self.lines[self.i].text.clone());
                self.i += 1;
            }
        } else {
            let mut first_child_indent = None;
            while self.i < self.lines.len() && self.lines[self.i].indent > base {
                let ind = self.lines[self.i].indent;
                match first_child_indent {
                    None => first_child_indent = Some(ind),
                    Some(expected) if ind != expected => {
                        let span = self.lines[self.i].span();
                        self.diagnostics.push(
                            Diagnostic::warning(
                                Code::InconsistentDedent,
                                format!(
                                    "child is indented {ind} spaces but its siblings use {expected}"
                                ),
                                span,
                            )
                            .with_help("use a consistent 2-space step per nesting level"),
                        );
                    }
                    _ => {}
                }
                if let Some(child) = self.parse_element() {
                    el.children.push(child);
                }
            }
        }

        Some(el)
    }

    /// Positionals, attributes, actions and content for a single line.
    fn fill_element(&mut self, el: &mut Element, line: &Line, kind: TagKind) {
        let toks: &[Token] = &line.tokens;
        let has_attr = toks.iter().skip(1).any(|t| matches!(t.tok, Tok::Eq));

        // Text tags with no attributes take the whole remainder as prose. This is the rule
        // that lets prose cost ~0 extra tokens (report §1.5).
        if kind == TagKind::Text && !has_attr {
            let rest = line.rest_from(1);
            if !rest.is_empty() {
                el.content = Some(rest.to_string());
            }
            if let Some(a) = toks.iter().find_map(|t| match &t.tok {
                Tok::Action(a) => Some(a.clone()),
                _ => None,
            }) {
                el.actions.push(a);
            }
            return;
        }

        let mut i = 1;
        while i < toks.len() {
            match &toks[i].tok {
                Tok::Word(w) => {
                    // `name=value` is an attribute; a bare word is a modifier or a label.
                    if matches!(toks.get(i + 1).map(|t| &t.tok), Some(Tok::Eq)) {
                        let name = w.clone();
                        if !self.reg.accepts_attr(&el.tag, &name) {
                            self.diagnostics.push(
                                Diagnostic::warning(
                                    Code::UnknownAttr,
                                    format!("`{}` does not accept the attribute `{name}`", el.tag),
                                    toks[i].span,
                                )
                                .with_help("unknown attributes are dropped by the code generator"),
                            );
                        }
                        match toks.get(i + 2) {
                            Some(v) => {
                                el.attrs.push(Attr {
                                    name,
                                    value: token_value(&v.tok),
                                    span: toks[i].span.to(v.span),
                                });
                                i += 3;
                            }
                            None => {
                                self.diagnostics.push(
                                    Diagnostic::error(
                                        Code::ExpectedValue,
                                        format!("expected a value after `{name}=`"),
                                        toks[i].span,
                                    )
                                    .with_help(
                                        "e.g. `placeholder=\"Add a task…\"` or `disabled={!count}`",
                                    ),
                                );
                                i += 2;
                            }
                        }
                        continue;
                    }

                    if self.reg.is_modifier(w) {
                        el.positionals.push(Positional::Modifier(w.clone()));
                    } else {
                        // Lower-case single words that are near-misses for a modifier are
                        // almost always typos; labels are normally capitalised or quoted.
                        if w.chars().next().is_some_and(char::is_lowercase)
                            && !w.contains('.')
                            && let Some(s) = Registry::suggest_modifier_close(w)
                        {
                            self.diagnostics.push(
                                Diagnostic::warning(
                                    Code::UnknownModifier,
                                    format!("`{w}` is not a known modifier"),
                                    toks[i].span,
                                )
                                .with_help(format!("did you mean `{s}`?"))
                                .with_suggestion(s.to_string()),
                            );
                        }
                        el.positionals.push(Positional::Text(w.clone()));
                    }
                    i += 1;
                }
                Tok::Str(s) => {
                    el.positionals.push(Positional::Text(s.clone()));
                    i += 1;
                }
                Tok::Num(n) => {
                    el.positionals.push(Positional::Text(n.clone()));
                    i += 1;
                }
                Tok::Brace(b) => {
                    el.positionals.push(Positional::Binding(b.clone()));
                    i += 1;
                }
                Tok::Route(r) => {
                    el.positionals.push(Positional::Route(r.clone()));
                    i += 1;
                }
                Tok::Anchor(a) => {
                    el.positionals.push(Positional::Anchor(a.clone()));
                    i += 1;
                }
                Tok::Action(a) => {
                    // `>` consumes the rest of the line by design (that is what makes actions
                    // lexable in one pass), so a modifier written *after* the action is
                    // swallowed into the action body. Catch the common case: an action body
                    // whose last word is a known modifier is almost always a misplaced one.
                    if let Some(last) = a.rsplit(' ').next()
                        && a.contains(' ')
                        && self.reg.is_modifier(last)
                    {
                        self.diagnostics.push(
                            Diagnostic::error(
                                Code::TrailingTokensAfterAction,
                                format!("modifier `{last}` appears after the action and was swallowed by it"),
                                toks[i].span,
                            )
                            .with_help("`>` consumes the rest of the line — put modifiers before it"),
                        );
                    }
                    el.actions.push(a.clone());
                    i += 1;
                }
                Tok::Pipe => {
                    // Everything after `|` is content, taken raw.
                    let rest = line.rest_from(i + 1);
                    if !rest.is_empty() {
                        el.content = Some(rest.to_string());
                    }
                    break;
                }
                Tok::Eq | Tok::Colon | Tok::Comma => {
                    i += 1;
                }
            }
        }
    }

    fn skip_children(&mut self, base: usize) {
        while self.i < self.lines.len() && self.lines[self.i].indent > base {
            self.i += 1;
        }
    }
}

fn token_value(t: &Tok) -> Value {
    match t {
        Tok::Str(s) => Value::Str(s.clone()),
        Tok::Num(n) => n.parse::<f64>().map(Value::Num).unwrap_or_else(|_| Value::Word(n.clone())),
        Tok::Brace(b) => Value::Binding(b.clone()),
        Tok::Word(w) => match w.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => Value::Word(w.clone()),
        },
        Tok::Route(r) => Value::Word(r.clone()),
        Tok::Anchor(a) => Value::Word(format!("#{a}")),
        _ => Value::Flag,
    }
}

fn is_http_method(w: &str) -> bool {
    matches!(w, "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(src: &str) -> Program {
        let reg = Registry::builtin();
        let p = parse(src, &reg);
        assert!(
            !p.diagnostics.has_errors(),
            "unexpected errors: {}",
            p.diagnostics.render(src, "test.guml")
        );
        p.program
    }

    #[test]
    fn parses_page_and_state() {
        let prog = ok("page Counter\nstate count=0\n");
        assert_eq!(prog.page.as_ref().unwrap().name, "Counter");
        assert_eq!(prog.states.len(), 1);
        assert_eq!(prog.states[0].name, "count");
        assert_eq!(prog.states[0].init, Value::Num(0.0));
    }

    #[test]
    fn parses_enumerated_state_domain() {
        let prog = ok("page P\nstate filter=all|open|done\n");
        let s = &prog.states[0];
        assert_eq!(s.domain, vec!["all", "open", "done"]);
        assert_eq!(s.init, Value::Word("all".into()));
    }

    #[test]
    fn single_value_state_has_no_domain() {
        let prog = ok("page P\nstate draft=\"\"\n");
        assert!(prog.states[0].domain.is_empty());
        assert_eq!(prog.states[0].init, Value::Str(String::new()));
    }

    #[test]
    fn nesting_follows_indentation() {
        let prog = ok("page P\ncard sm center\n  h Clicks\n  row center\n    btn Go primary\n");
        assert_eq!(prog.tree.len(), 1);
        let card = &prog.tree[0];
        assert_eq!(card.tag, "card");
        assert!(card.has_modifier("sm") && card.has_modifier("center"));
        assert_eq!(card.children.len(), 2);
        assert_eq!(card.children[0].content.as_deref(), Some("Clicks"));
        assert_eq!(card.children[1].children[0].tag, "btn");
        assert_eq!(card.children[1].children[0].label(), Some("Go"));
    }

    #[test]
    fn prose_survives_verbatim() {
        let prog = ok("page P\np Press the buttons to change the value.\n");
        assert_eq!(prog.tree[0].content.as_deref(), Some("Press the buttons to change the value."));
    }

    #[test]
    fn text_tag_with_attrs_parses_structurally() {
        let prog = ok("page P\ntext {title} strike={done}\n");
        let el = &prog.tree[0];
        assert_eq!(el.binding(), Some("title"));
        assert_eq!(el.attr("strike"), Some(&Value::Binding("done".into())));
        assert!(el.content.is_none());
    }

    #[test]
    fn actions_and_bindings_on_a_control() {
        let prog = ok("page P\nbtn Decrement ghost disabled={!count} >count--\n");
        let el = &prog.tree[0];
        assert_eq!(el.label(), Some("Decrement"));
        assert!(el.has_modifier("ghost"));
        assert_eq!(el.attr("disabled"), Some(&Value::Binding("!count".into())));
        assert_eq!(el.actions, vec!["count--".to_string()]);
    }

    #[test]
    fn pipe_splits_title_from_body() {
        let prog = ok("page P\ncard \"Ship in minutes\" | Describe the page, get a build.\n");
        let el = &prog.tree[0];
        assert_eq!(el.label(), Some("Ship in minutes"));
        assert_eq!(el.content.as_deref(), Some("Describe the page, get a build."));
    }

    #[test]
    fn parses_a_resource_with_mutations() {
        let src = "page Tasks\n\
                   type Task {id, title, done:bool}\n\
                   data tasks:Task[] GET /api/tasks\n\
                   \x20 add  POST   /api/tasks         {title}  optimistic:prepend\n\
                   \x20 drop DELETE /api/tasks/{id}             optimistic\n";
        let prog = ok(src);
        assert_eq!(prog.types[0].fields.len(), 3);
        assert_eq!(prog.types[0].fields[2].ty, "bool");
        let r = &prog.resources[0];
        assert_eq!(
            (r.name.as_str(), r.ty.as_str(), r.method.as_str(), r.url.as_str()),
            ("tasks", "Task[]", "GET", "/api/tasks")
        );
        assert_eq!(r.mutations.len(), 2);
        assert_eq!(r.mutations[0].body, vec!["title".to_string()]);
        assert_eq!(r.mutations[0].optimistic.as_deref(), Some("prepend"));
        assert_eq!(r.mutations[1].optimistic.as_deref(), Some("replace"));
        assert_eq!(r.mutations[1].url, "/api/tasks/{id}");
    }

    #[test]
    fn text_child_tags_keep_children_as_lines() {
        let src = "page P\n\
                   tier Pro $24/mo \"For working developers\" cta=\"Go Pro\" /signup featured\n\
                   \x20 Unlimited projects\n\
                   \x20 Custom domains\n";
        let prog = ok(src);
        let tier = &prog.tree[0];
        assert_eq!(tier.text_lines, vec!["Unlimited projects", "Custom domains"]);
        assert!(tier.has_modifier("featured"));
        assert_eq!(tier.route(), Some("/signup"));
        assert_eq!(tier.attr("cta"), Some(&Value::Str("Go Pro".into())));
    }

    // ---- error recovery: the property that keeps the repair loop to one round ----

    #[test]
    fn collects_every_error_in_one_pass() {
        let src = "page P\nbuton Go\ncrad\n  h Fine\nbtn\n";
        let reg = Registry::builtin();
        let p = parse(src, &reg);
        let unknown: Vec<_> =
            p.diagnostics.items.iter().filter(|d| d.code == Code::UnknownTag).collect();
        assert_eq!(unknown.len(), 2, "both unknown tags reported in one pass");
        assert_eq!(unknown[0].suggestion.as_deref(), Some("btn"));
        assert_eq!(unknown[1].suggestion.as_deref(), Some("card"));
        // Parsing continued: the well-formed subtree is still present.
        assert!(p.program.tree.iter().any(|e| e.tag == "crad" && e.children.len() == 1));
    }

    #[test]
    fn missing_attr_value_is_an_error_not_a_panic() {
        let reg = Registry::builtin();
        let p = parse("page P\ninput draft placeholder=\n", &reg);
        assert!(p.diagnostics.items.iter().any(|d| d.code == Code::ExpectedValue));
    }

    #[test]
    fn trailing_tokens_after_action_are_rejected() {
        let reg = Registry::builtin();
        let p = parse("page P\nbtn Go >count++ primary\n", &reg);
        assert!(p.diagnostics.items.iter().any(|d| d.code == Code::TrailingTokensAfterAction));
    }

    #[test]
    fn missing_page_directive_warns_but_parses() {
        let reg = Registry::builtin();
        let p = parse("card\n  h Hi\n", &reg);
        assert!(!p.diagnostics.has_errors());
        assert!(p.diagnostics.items.iter().any(|d| d.code == Code::MissingPageDirective));
    }
}
