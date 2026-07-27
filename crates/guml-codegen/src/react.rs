//! React + TypeScript + Tailwind backend.
//!
//! Chosen as the v1 target for three reasons (report §6.6, §10.1): the largest ecosystem
//! gravity, the easiest hand-off to a human who outgrows GUML, and direct comparability with
//! the benchmark's React baseline.
//!
//! Everything in `classes()` is a token the model no longer emits. That table *is* the design
//! system, and swapping it for an organisation's own tokens is how the compiler enforces
//! design-system compliance (report §1.4, enterprise governance).

use crate::{
    Backend, Emitted, OutFile, component_name, jsx_attr, jsx_text, lower_action, modifiers_of,
    state_names, unsupported,
};
use guml_ast::{Element, Program, Value};

#[derive(Debug, Default)]
pub struct ReactBackend;

impl Backend for ReactBackend {
    fn name(&self) -> &'static str {
        "react"
    }

    fn emit(&self, program: &Program) -> Emitted {
        let mut out = Emitted::default();
        let name = component_name(program.page.as_ref().map(|p| p.name.as_str()).unwrap_or("Page"));
        let states = state_names(program);

        let mut body = String::new();
        for el in &program.tree {
            body.push_str(&self.element(el, 2, &states, &mut out));
        }

        if !program.resources.is_empty() {
            for r in &program.resources {
                unsupported(&mut out.diagnostics, r.span, format!("resource `{}`", r.name));
            }
        }

        let mut src = String::new();
        if !program.states.is_empty() {
            src.push_str("import { useState } from \"react\";\n\n");
        }
        src.push_str(&format!("export default function {name}() {{\n"));
        for s in &program.states {
            src.push_str(&format!(
                "  const [{}, {}] = useState({});\n",
                s.name,
                crate::setter(&s.name),
                initial(&s.init)
            ));
        }
        if !program.states.is_empty() {
            src.push('\n');
        }
        src.push_str("  return (\n");
        if body.is_empty() {
            src.push_str("    <></>\n");
        } else {
            src.push_str(&body);
        }
        src.push_str("  );\n}\n");

        out.files.push(OutFile { path: format!("{name}.tsx"), contents: src });
        out
    }
}

impl ReactBackend {
    fn element(&self, el: &Element, depth: usize, states: &[String], out: &mut Emitted) -> String {
        let pad = " ".repeat(depth * 2);
        let mods = modifiers_of(el);
        let class = classes(&el.tag, &mods);

        let (tag_name, extra) = html_tag(&el.tag, el);
        if tag_name.is_none() {
            unsupported(&mut out.diagnostics, el.span, format!("tag `{}`", el.tag));
            return format!(
                "{pad}{{/* TODO(guml): `{}` not lowered by the v0.1 React backend */}}\n",
                el.tag
            );
        }
        let tag_name = tag_name.unwrap();

        let mut attrs: Vec<String> = Vec::new();
        attrs.extend(extra);
        if !class.is_empty() {
            attrs.push(format!("className={:?}", class));
        }
        if let Some(a) = el.anchor() {
            attrs.push(format!("id={:?}", a));
        }
        for a in &el.attrs {
            match a.name.as_str() {
                "aria" => attrs.push(jsx_attr("aria-label", &a.value)),
                "class" | "id" => {} // reserved: theme overrides land with the resolver
                _ => attrs.push(jsx_attr(&a.name, &a.value)),
            }
        }
        if !el.actions.is_empty() {
            let js = lower_action(&el.actions[0], states, &mut out.diagnostics, el.span);
            if !js.is_empty() {
                let handler = if el.tag == "check" || el.tag == "toggle" {
                    "onChange"
                } else if el.tag == "form" {
                    "onSubmit"
                } else {
                    "onClick"
                };
                attrs.push(format!("{handler}={{() => {{ {js}; }}}}"));
            }
        }

        let attr_str =
            if attrs.is_empty() { String::new() } else { format!(" {}", attrs.join(" ")) };

        // Leaf with text content.
        let text = el
            .content
            .clone()
            .or_else(|| el.label().map(str::to_string))
            .filter(|_| el.children.is_empty() && el.text_lines.is_empty());

        if is_void(&el.tag) {
            return format!("{pad}<{tag_name}{attr_str} />\n");
        }

        match text {
            Some(t) if el.children.is_empty() => {
                format!("{pad}<{tag_name}{attr_str}>{}</{tag_name}>\n", jsx_text(&t))
            }
            _ => {
                let mut s = format!("{pad}<{tag_name}{attr_str}>\n");
                if let Some(label) = el.label() {
                    // Container with a title positional: emit it as a heading.
                    s.push_str(&format!(
                        "{}  <h3 className=\"font-medium\">{}</h3>\n",
                        pad,
                        jsx_text(label)
                    ));
                }
                if let Some(c) = &el.content {
                    s.push_str(&format!(
                        "{}  <p className=\"mt-2 text-sm text-slate-600\">{}</p>\n",
                        pad,
                        jsx_text(c)
                    ));
                }
                for line in &el.text_lines {
                    s.push_str(&format!("{}  <li>{}</li>\n", pad, jsx_text(line)));
                }
                for child in &el.children {
                    s.push_str(&self.element(child, depth + 1, states, out));
                }
                s.push_str(&format!("{pad}</{tag_name}>\n"));
                s
            }
        }
    }
}

fn initial(v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{:?}", s),
        Value::Num(_) | Value::Bool(_) => v.to_js(),
        Value::Word(w) => format!("{:?}", w),
        Value::Binding(b) => b.clone(),
        Value::Flag => "true".into(),
    }
}

/// GUML tag -> HTML element, plus any fixed attributes. `None` means "not lowered yet".
pub(crate) fn html_tag(tag: &str, el: &Element) -> (Option<&'static str>, Vec<String>) {
    match tag {
        "card" | "row" | "col" | "metric" => (Some("div"), vec![]),
        "section" => (Some("section"), vec![]),
        "nav" => (Some("nav"), vec![]),
        "hero" => (Some("header"), vec![]),
        "footer" => (Some("footer"), vec![]),
        "h" | "h2" => (Some("h2"), vec![]),
        "h1" => (Some("h1"), vec![]),
        "h3" => (Some("h3"), vec![]),
        "p" | "head" | "empty" => (Some("p"), vec![]),
        "text" => (Some("span"), vec![]),
        "btn" => (Some("button"), vec!["type=\"button\"".to_string()]),
        "link" => {
            let href = el
                .route()
                .map(str::to_string)
                .or_else(|| el.anchor().map(|a| format!("#{a}")))
                .unwrap_or_else(|| "#".to_string());
            (Some("a"), vec![format!("href={:?}", href)])
        }
        "check" => (Some("input"), vec!["type=\"checkbox\"".to_string()]),
        "input" => (Some("input"), vec!["type=\"text\"".to_string()]),
        _ => (None, vec![]),
    }
}

pub(crate) fn is_void(tag: &str) -> bool {
    matches!(tag, "input" | "check")
}

/// The design system. Every string here is a token the model does not have to produce, and a
/// presentational decision it cannot get wrong.
pub(crate) fn classes(tag: &str, mods: &[&str]) -> String {
    let has = |m: &str| mods.contains(&m);
    let mut c: Vec<&str> = Vec::new();

    match tag {
        "card" => {
            c.push("rounded-xl border border-slate-200 bg-white p-6 shadow-sm");
            if has("sm") {
                c.push("mx-auto mt-10 w-full max-w-sm");
            }
            if has("center") {
                c.push("text-center");
            }
        }
        "row" => {
            c.push("flex items-center gap-3");
            if has("center") {
                c.push("justify-center");
            }
            if has("between") {
                c.push("justify-between");
            }
            if has("wrap") {
                c.push("flex-wrap");
            }
        }
        "col" => c.push("flex flex-col gap-3"),
        "section" => c.push("mx-auto max-w-6xl px-6 py-16"),
        "nav" => c.push("mx-auto flex max-w-6xl items-center justify-between px-6 py-5"),
        "hero" => c.push("mx-auto max-w-3xl px-6 py-24 text-center"),
        "footer" => c.push("border-t border-slate-200 px-6 py-8 text-sm text-slate-500"),
        "h" | "h2" => c.push("text-lg font-semibold text-slate-900"),
        "h1" => c.push("text-4xl font-semibold tracking-tight text-slate-900 sm:text-5xl"),
        "p" => c.push("mt-1 text-sm text-slate-500"),
        "head" => c.push("text-2xl font-semibold text-slate-900"),
        "empty" => c.push("mt-10 text-center text-sm text-slate-500"),
        "metric" => c.push("mt-6 text-center text-5xl font-bold tabular-nums text-slate-900"),
        "text" => {
            c.push("text-sm");
            if has("quiet") {
                c.push("text-slate-500");
            }
        }
        "btn" => {
            c.push("rounded-md px-4 py-2 text-sm font-medium transition-colors");
            if has("primary") {
                c.push("bg-slate-900 text-white hover:bg-slate-800");
            } else if has("outline") {
                c.push("border border-slate-300 text-slate-700 hover:bg-slate-50");
            } else if has("quiet") {
                c.push("text-slate-500 hover:text-slate-900");
            } else if has("danger") {
                c.push("bg-red-600 text-white hover:bg-red-700");
            } else {
                // `ghost` and the default share a neutral bordered look.
                c.push("border border-slate-300 text-slate-700 hover:bg-slate-50");
            }
            c.push("disabled:opacity-40");
        }
        "link" => c.push("text-sm text-slate-600 hover:text-slate-900"),
        "input" => c.push(
            "w-full rounded-md border border-slate-300 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-slate-900",
        ),
        "check" => c.push("h-4 w-4 rounded border-slate-300"),
        _ => {}
    }

    if has("full") {
        c.push("w-full");
    }
    c.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use guml_registry::Registry;

    fn build(src: &str) -> (String, guml_diagnostics::Diagnostics) {
        let reg = Registry::builtin();
        let parsed = guml_parser_stub::parse(src, &reg);
        let emitted = ReactBackend.emit(&parsed);
        (emitted.files[0].contents.clone(), emitted.diagnostics)
    }

    // guml-codegen must not depend on guml-parser (that would be a cycle through the
    // compiler crate), so unit tests here build the AST by hand. End-to-end coverage from
    // source text lives in `crates/guml-compiler/tests`.
    mod guml_parser_stub {
        use guml_ast::*;
        use guml_diagnostics::Span;
        use guml_registry::Registry;

        pub fn parse(_src: &str, _reg: &Registry) -> Program {
            let span = Span::point(0, 1, 1);
            let mut card = Element::new("card", span);
            card.positionals.push(Positional::Modifier("sm".into()));
            let mut h = Element::new("h", span);
            h.content = Some("Clicks".into());
            let mut metric = Element::new("metric", span);
            metric.content = Some("{count}".into());
            let mut btn = Element::new("btn", span);
            btn.positionals.push(Positional::Text("Increment".into()));
            btn.positionals.push(Positional::Modifier("primary".into()));
            btn.actions.push("count++".into());
            card.children = vec![h, metric, btn];

            Program {
                page: Some(PageDecl { name: "Counter".into(), span }),
                states: vec![StateDecl {
                    name: "count".into(),
                    init: Value::Num(0.0),
                    domain: vec![],
                    span,
                }],
                tree: vec![card],
                ..Default::default()
            }
        }
    }

    #[test]
    fn emits_a_compilable_component_shape() {
        let (src, diags) = build("");
        assert!(src.starts_with("import { useState } from \"react\";"));
        assert!(src.contains("export default function Counter()"));
        assert!(src.contains("const [count, setCount] = useState(0);"));
        assert!(!diags.has_errors());
    }

    #[test]
    fn model_never_writes_the_class_strings() {
        let (src, _) = build("");
        assert!(src.contains("rounded-xl border border-slate-200"));
        assert!(src.contains("bg-slate-900 text-white"));
    }

    #[test]
    fn actions_become_handlers() {
        let (src, _) = build("");
        assert!(src.contains("onClick={() => { setCount(count + 1); }}"));
    }

    #[test]
    fn bindings_pass_through_to_jsx() {
        let (src, _) = build("");
        assert!(src.contains(">{count}<"));
    }

    #[test]
    fn button_gets_an_explicit_type() {
        let (src, _) = build("");
        assert!(src.contains("<button type=\"button\""));
    }
}
