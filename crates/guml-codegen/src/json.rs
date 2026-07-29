//! UI-tree backend.
//!
//! Emits a resolved render tree instead of source text, which is what makes a
//! browser runtime — and therefore live preview and a playground — possible
//! without shipping a JS compiler.
//!
//! The important property: **the tree carries the same classes the React backend
//! writes**, because both call `react::classes`. A preview cannot drift from the
//! code you would have generated, which would otherwise be the single most
//! misleading thing a playground could do.
//!
//! Shape-wise this is deliberately close to A2UI and to server-driven UI: a
//! nested component tree over a closed catalog, plus the state and resource
//! declarations the runtime needs. That makes it the natural base for the A2UI
//! and MCP-UI emitters on the roadmap.

use crate::react;
use guml_ast::{Element, Program, Value};
use guml_diagnostics::Diagnostics;
use serde::Serialize;

use crate::{Backend, Emitted, OutFile, component_name, modifiers_of};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiTree {
    pub page: String,
    pub state: Vec<StateInit>,
    pub resources: Vec<ResourceSpec>,
    pub nodes: Vec<UiNode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateInit {
    pub name: String,
    pub init: serde_json::Value,
    pub domain: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSpec {
    pub name: String,
    pub ty: String,
    pub method: String,
    pub url: String,
    pub mutations: Vec<MutationSpec>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationSpec {
    pub name: String,
    pub method: String,
    pub url: String,
    pub body: Vec<String>,
    pub optimistic: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiNode {
    /// GUML tag, kept so a host can swap in its own component per tag.
    pub tag: String,
    /// HTML element the default runtime renders. `null` for tags the compiler
    /// cannot lower yet — the runtime shows the gap instead of guessing.
    pub el: Option<String>,
    /// Classes from the compiler's design-system table.
    pub class: String,
    /// Prose, possibly containing `{binding}` interpolations.
    pub text: Option<String>,
    /// First label-ish positional.
    pub label: Option<String>,
    /// Primary binding: `metric {count}`, `check {done}`, `input draft`.
    pub bind: Option<String>,
    pub props: Vec<Prop>,
    /// Raw action bodies; the runtime lowers them the same way the React backend
    /// does, so behaviour matches the emitted code.
    pub actions: Vec<String>,
    /// Repeater source resource, and its filter.
    pub source: Option<String>,
    pub filter: Option<String>,
    /// Binding a row's controls take their accessible name from.
    pub aria_from: Option<String>,
    pub lines: Vec<String>,
    pub children: Vec<UiNode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prop {
    pub name: String,
    pub value: serde_json::Value,
    /// True when `value` is a binding expression to evaluate rather than a literal.
    pub bound: bool,
}

#[derive(Debug, Default)]
pub struct JsonBackend;

impl Backend for JsonBackend {
    fn name(&self) -> &'static str {
        "json"
    }

    fn emit(&self, program: &Program) -> Emitted {
        let mut out = Emitted::default();
        let tree = ui_tree(program, &mut out.diagnostics);
        let contents = serde_json::to_string_pretty(&tree).unwrap_or_else(|_| "{}".into());
        // A render tree has no line correspondence worth claiming: it is one JSON object for the
        // whole document.
        out.files.push(OutFile {
            path: format!("{}.ui.json", tree.page),
            contents,
            source_map: None,
        });
        out
    }
}

/// Build the render tree. Never fails: unlowerable tags come back with
/// `el: null` so the runtime can display the gap honestly.
pub fn ui_tree(program: &Program, diags: &mut Diagnostics) -> UiTree {
    let _ = diags; // the tree reports gaps structurally rather than as warnings
    // Same dead-declaration elimination as the React backend, and for a sharper reason here: an
    // unreferenced `data` in this tree is a request the runtime fires on mount for data nothing
    // renders. See `guml_ast::referenced_names` for why eliding is safe.
    let live = guml_ast::referenced_names(program);
    UiTree {
        page: component_name(program.page.as_ref().map(|p| p.name.as_str()).unwrap_or("Page")),
        state: program
            .states
            .iter()
            .filter(|s| live.contains(&s.name))
            .map(|s| StateInit {
                name: s.name.clone(),
                init: json_value(&s.init),
                domain: s.domain.clone(),
            })
            .collect(),
        resources: program
            .resources
            .iter()
            .filter(|r| live.contains(&r.name))
            .map(|r| ResourceSpec {
                name: r.name.clone(),
                ty: r.ty.clone(),
                method: r.method.clone(),
                url: r.url.clone(),
                mutations: r
                    .mutations
                    .iter()
                    .map(|m| MutationSpec {
                        name: m.name.clone(),
                        method: m.method.clone(),
                        url: m.url.clone(),
                        body: m.body.clone(),
                        optimistic: m.optimistic.clone(),
                    })
                    .collect(),
            })
            .collect(),
        nodes: program.tree.iter().map(node).collect(),
    }
}

fn node(el: &Element) -> UiNode {
    // An escape hatch is arbitrary code, and this tree is consumed by the browser runtime — which
    // renders documents that may have come from an untrusted agent. So the *content* is dropped
    // here rather than passed along: there is no path from a `js` block to `eval` because the code
    // never reaches the client. The emitted file is the only place it runs.
    if el.tag == "js" || el.tag == "raw" {
        return UiNode {
            tag: format!("{}-placeholder", el.tag),
            // `el: None` is the existing signal for "the runtime shows the gap instead of
            // guessing", which is exactly right here.
            el: None,
            class: String::new(),
            text: Some(format!(
                "{} block: {} line(s), present in the emitted code but not run in the preview",
                el.tag,
                el.text_lines.len()
            )),
            label: None,
            bind: None,
            props: Vec::new(),
            actions: Vec::new(),
            source: None,
            filter: None,
            aria_from: None,
            // Deliberately empty: `lines` is what the runtime renders, and the block body must
            // not reach the client.
            lines: Vec::new(),
            children: Vec::new(),
        };
    }

    let mods = modifiers_of(el);
    let (element, fixed) = react::html_tag(&el.tag, el);

    let mut props: Vec<Prop> = Vec::new();
    for f in fixed {
        // `type="button"` and `href="…"` arrive as pre-rendered attribute strings.
        if let Some((name, raw)) = f.split_once('=') {
            props.push(Prop {
                name: name.to_string(),
                value: serde_json::Value::String(raw.trim_matches('"').to_string()),
                bound: false,
            });
        }
    }
    if let Some(anchor) = el.anchor() {
        props.push(Prop {
            name: "id".into(),
            value: serde_json::Value::String(anchor.to_string()),
            bound: false,
        });
    }
    for a in &el.attrs {
        let name = match a.name.as_str() {
            "aria" => "aria-label".to_string(),
            other => other.to_string(),
        };
        match &a.value {
            Value::Binding(b) => props.push(Prop {
                name,
                value: serde_json::Value::String(b.source.clone()),
                bound: true,
            }),
            v => props.push(Prop { name, value: json_value(v), bound: false }),
        }
    }

    let is_repeater = matches!(el.tag.as_str(), "list" | "table");

    UiNode {
        tag: el.tag.clone(),
        el: element.map(str::to_string),
        class: react::classes(&el.tag, &mods),
        text: el.content.clone(),
        label: el.label().map(str::to_string),
        bind: el.binding().map(str::to_string).or_else(|| {
            // `input draft` and `list tasks` name their target positionally.
            if matches!(el.tag.as_str(), "input" | "select" | "tabs") {
                el.label().map(str::to_string)
            } else {
                None
            }
        }),
        props,
        actions: el.actions.clone(),
        source: if is_repeater { el.label().map(str::to_string) } else { None },
        filter: el.attr("where").and_then(|v| match v {
            Value::Binding(b) => Some(b.source.clone()),
            v => v.as_text().map(str::to_string),
        }),
        aria_from: if is_repeater { row_binding(el) } else { None },
        lines: el.text_lines.clone(),
        children: el.children.iter().map(node).collect(),
    }
}

/// The binding a repeater's row is identified by — mirrors the accessibility rule
/// in `guml-compiler::sema`, so a control the analyser accepted as "named by its
/// row" actually receives that name here.
fn row_binding(repeater: &Element) -> Option<String> {
    repeater.children.iter().find_map(|c| {
        if !matches!(c.tag.as_str(), "text" | "p" | "h" | "h1" | "h2" | "h3" | "head") {
            return None;
        }
        c.binding().map(str::to_string).or_else(|| {
            let content = c.content.as_deref()?;
            let open = content.find('{')?;
            let close = content[open + 1..].find('}')?;
            Some(content[open + 1..open + 1 + close].trim().to_string())
        })
    })
}

fn json_value(v: &Value) -> serde_json::Value {
    match v {
        Value::Str(s) | Value::Word(s) => serde_json::Value::String(s.clone()),
        Value::Num(n) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Binding(b) => serde_json::Value::String(b.source.clone()),
        Value::Flag => serde_json::Value::Bool(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guml_ast::{Element, PageDecl, Positional, StateDecl};
    use guml_diagnostics::Span;

    fn program() -> Program {
        let span = Span::point(0, 1, 1);
        let mut card = Element::new("card", span);
        card.positionals.push(Positional::Modifier("sm".into()));
        let mut btn = Element::new("btn", span);
        btn.positionals.push(Positional::Text("Increment".into()));
        btn.positionals.push(Positional::Modifier("primary".into()));
        btn.actions.push("count++".into());
        card.children.push(btn);

        Program {
            page: Some(PageDecl { name: "Counter".into(), meta: Default::default(), span }),
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

    #[test]
    fn tree_carries_state_and_nodes() {
        let mut d = Diagnostics::new();
        let tree = ui_tree(&program(), &mut d);
        assert_eq!(tree.page, "Counter");
        assert_eq!(tree.state[0].name, "count");
        assert_eq!(tree.nodes[0].tag, "card");
        assert_eq!(tree.nodes[0].children[0].tag, "btn");
    }

    #[test]
    fn classes_match_the_react_backend() {
        let mut d = Diagnostics::new();
        let tree = ui_tree(&program(), &mut d);
        let btn = &tree.nodes[0].children[0];
        // Same table, so a preview cannot drift from emitted code.
        assert_eq!(btn.class, react::classes("btn", &["primary"]));
        assert!(btn.class.contains("bg-slate-900"));
    }

    #[test]
    fn buttons_carry_their_fixed_attributes_and_actions() {
        let mut d = Diagnostics::new();
        let tree = ui_tree(&program(), &mut d);
        let btn = &tree.nodes[0].children[0];
        assert!(btn.props.iter().any(|p| p.name == "type"));
        assert_eq!(btn.actions, vec!["count++".to_string()]);
        assert_eq!(btn.label.as_deref(), Some("Increment"));
    }

    #[test]
    fn serialises_to_camel_case_json() {
        let mut d = Diagnostics::new();
        let json = serde_json::to_string(&ui_tree(&program(), &mut d)).unwrap();
        assert!(json.contains("\"ariaFrom\""));
        assert!(json.contains("\"page\":\"Counter\""));
    }
}
