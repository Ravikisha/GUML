//! Agent-UI wire formats: A2UI and MCP-UI.
//!
//! # Why these are targets and not competitors
//!
//! The research report's strategic read (§13) is blunabout this: A2UI is simultaneously the strongest
//! competitor and the strongest partner. The *concept* "an agent emits a declarative UI description
//! against a host-approved component catalog" is already standardised and shipping with Google's weight
//! behind it, so GUML cannot claim it. What those protocols do not have is a token-efficient surface
//! syntax, a compiler, or an application-logic layer — which is exactly what GUML is.
//!
//! Emitting them turns "GUML vs A2UI" into "GUML compiles to A2UI". A model writes 25 lines; the
//! compiler produces the JSON payload the host already knows how to render.
//!
//! # An honest caveat about conformance
//!
//! The A2UI emitter targets the **shape** the report documents (§2.5): a flat component list with
//! id references, a pre-approved catalog, declarative rather than executable. It is deliberately
//! self-describing — the payload carries `"format": "a2ui-shaped"` and its own version — because this
//! was written from a description of the protocol rather than against its published JSON schema.
//! Claiming spec conformance without validating against the schema would be exactly the kind of
//! unsupported claim `CLAUDE.md` forbids. Pinning it is a small, mechanical job once the schema is in
//! the repository, and `ROADMAP.md` says so.
//!
//! MCP-UI needs no such caveat, because it is not a new format: the protocol's documented rendering
//! modes are a sandboxed iframe of HTML and a remote-DOM script, and GUML already has backends that
//! emit both. This emitter *composes* them rather than inventing anything.
//!
//! # The security posture, which is the interesting part
//!
//! A2UI is non-executable **by design**, because a host renders documents that arrived from an untrusted
//! remote agent. That is the same reasoning behind GUML's `core` level, so the two line up exactly:
//!
//! * A `js` block cannot cross into A2UI. Not stripped silently — reported, because a document whose
//!   behaviour was quietly removed renders as a page that looks complete and does nothing.
//! * An action becomes a declared **intent**: a name and a body the host may choose to honour. The
//!   statements are not lowered to JavaScript, so nothing executable travels in the payload.
//! * A resource becomes a declared **data requirement**, not a fetch. The host decides whether to make
//!   the request, which is the only version of this that is safe when the payload is untrusted.

use crate::react::classes;
use crate::{Backend, Emitted, OutFile, component_name, modifiers_of, unsupported_in};
use guml_ast::{Element, Program, Value};
use guml_diagnostics::Diagnostics;
use serde::Serialize;

/// The shape this emitter targets. Carried in the payload so a consumer can tell what it is reading.
const A2UI_SHAPE: &str = "a2ui-shaped";
const A2UI_VERSION: &str = "0.1";

#[derive(Debug, Default)]
pub struct A2uiBackend;

#[derive(Serialize)]
struct A2uiDoc {
    /// Deliberately not `"a2ui"`. See the module docs: this targets the documented shape and has not
    /// been validated against the published schema.
    format: &'static str,
    version: &'static str,
    page: String,
    /// The component types this payload uses. A host holds a pre-approved catalog and may only render
    /// from it, so stating the requirement up front lets it reject the payload before rendering half of
    /// it — which is the difference between a refusal and a broken screen.
    catalog: Vec<String>,
    /// Ids of the top-level components, in order.
    roots: Vec<String>,
    /// **Flat**, with children referenced by id. That is the protocol's own choice and it is a good one
    /// for this purpose: a flat list is incrementally updateable, so an agent can patch one component
    /// without re-sending the tree.
    components: Vec<A2uiNode>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    state: Vec<A2uiState>,
    /// Data the payload needs. A *requirement*, not a fetch — the host decides whether to issue it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    data: Vec<A2uiData>,
    /// Actions as declared intents. No lowered JavaScript travels here.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    intents: Vec<A2uiIntent>,
}

#[derive(Serialize)]
struct A2uiNode {
    id: String,
    /// The GUML tag. Kept as the tag rather than translated to an HTML element: a host catalog is a
    /// catalog of *components*, and `card` carries intent that `div` has thrown away.
    #[serde(rename = "type")]
    kind: String,
    /// The HTML element the compiler's own backends use, as a rendering hint for a host that has no
    /// entry for this type. Advisory — a host with a `card` component should use it.
    element: Option<&'static str>,
    /// The class string the active theme produces. Advisory in the same way: a host with its own design
    /// system ignores it, and a host without one gets the compiler's.
    #[serde(skip_serializing_if = "str::is_empty")]
    class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    /// A binding, as the author wrote it. Not lowered: the host resolves it against its own state, and
    /// handing it JavaScript would defeat the non-executable guarantee.
    #[serde(skip_serializing_if = "Option::is_none")]
    bind: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    properties: Vec<A2uiProp>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    modifiers: Vec<String>,
    /// Shown only while this expression is true. The author's text, for the same reason as `bind`.
    #[serde(skip_serializing_if = "Option::is_none")]
    visible_when: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<String>,
    /// Content lines, for `tier` perks and `faq` pairs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lines: Vec<String>,
    /// Ids of the intents this component can raise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    intents: Vec<String>,
}

#[derive(Serialize)]
struct A2uiProp {
    name: String,
    value: serde_json::Value,
    /// True when `value` is an expression for the host to resolve rather than a literal.
    bound: bool,
}

#[derive(Serialize)]
struct A2uiState {
    name: String,
    initial: serde_json::Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    domain: Vec<String>,
}

#[derive(Serialize)]
struct A2uiData {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    method: String,
    url: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    mutations: Vec<A2uiMutation>,
}

#[derive(Serialize)]
struct A2uiMutation {
    name: String,
    method: String,
    url: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    body: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optimistic: Option<String>,
}

#[derive(Serialize)]
struct A2uiIntent {
    id: String,
    /// The event that raises it, named the way the host will recognise it.
    on: &'static str,
    /// The author's statements, unlowered and unparsed into code. A host may honour them, map them onto
    /// its own actions, or refuse.
    statements: Vec<String>,
}

impl Backend for A2uiBackend {
    fn name(&self) -> &'static str {
        "a2ui"
    }

    fn emit(&self, program: &Program) -> Emitted {
        let mut out = Emitted::default();
        let mut doc = A2uiDoc {
            format: A2UI_SHAPE,
            version: A2UI_VERSION,
            page: component_name(program.page.as_ref().map(|p| p.name.as_str()).unwrap_or("Page")),
            catalog: Vec::new(),
            roots: Vec::new(),
            components: Vec::new(),
            state: program
                .states
                .iter()
                .map(|s| A2uiState {
                    name: s.name.clone(),
                    initial: json_value(&s.init),
                    domain: s.domain.clone(),
                })
                .collect(),
            data: program
                .resources
                .iter()
                .map(|r| A2uiData {
                    name: r.name.clone(),
                    ty: r.ty.clone(),
                    method: r.method.clone(),
                    url: r.url.clone(),
                    mutations: r
                        .mutations
                        .iter()
                        .map(|m| A2uiMutation {
                            name: m.name.clone(),
                            method: m.method.clone(),
                            url: m.url.clone(),
                            body: m.body.clone(),
                            optimistic: m.optimistic.clone(),
                        })
                        .collect(),
                })
                .collect(),
            intents: Vec::new(),
        };

        let mut next = 0usize;
        for el in &program.tree {
            if let Some(id) = flatten(el, &mut doc, &mut next, &mut out.diagnostics) {
                doc.roots.push(id);
            }
        }

        doc.catalog = {
            let mut types: Vec<String> = doc.components.iter().map(|c| c.kind.clone()).collect();
            types.sort();
            types.dedup();
            types
        };

        let json = serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".into());
        out.files.push(OutFile {
            path: format!("{}.a2ui.json", doc.page),
            contents: format!("{json}\n"),
            source_map: None,
        });
        out
    }
}

/// Append `el` and its subtree to the flat list, returning its id.
fn flatten(
    el: &Element,
    doc: &mut A2uiDoc,
    next: &mut usize,
    diags: &mut Diagnostics,
) -> Option<String> {
    // A `js` block is executable code, and this format exists precisely so a host never has to run
    // any. Reported rather than dropped: a payload whose behaviour vanished silently renders as a page
    // that looks finished and does nothing.
    if el.tag == "js" {
        unsupported_in(
            diags,
            "a2ui",
            el.span,
            "a `js` block cannot be represented: A2UI is declarative by design, so a host never receives executable code",
        );
        return None;
    }
    // A `raw` block is host markup. Only one aimed at this format could be carried, and there is no
    // sensible target — a JSON payload has nowhere to put a fragment of JSX.
    if el.tag == "raw" {
        unsupported_in(
            diags,
            "a2ui",
            el.span,
            "a `raw` block is host markup, and a component payload has nowhere to put it",
        );
        return None;
    }

    let id = format!("n{}", *next);
    *next += 1;

    let mut properties = Vec::new();
    let mut visible_when = None;
    for a in &el.attrs {
        match (&a.name[..], &a.value) {
            ("if", Value::Binding(b)) => visible_when = Some(b.source.clone()),
            ("if", v) => {
                properties.push(A2uiProp { name: "if".into(), value: json_value(v), bound: false })
            }
            (_, Value::Binding(b)) => properties.push(A2uiProp {
                name: a.name.clone(),
                // The author's expression, not lowered JavaScript. The host resolves it.
                value: serde_json::Value::String(b.source.clone()),
                bound: true,
            }),
            (_, v) => properties.push(A2uiProp {
                name: a.name.clone(),
                value: json_value(v),
                bound: false,
            }),
        }
    }

    // A repeater's source is a positional; a control's is its label. Recording which is which is what
    // lets a host bind the right thing.
    if let Some(source) = el.label().filter(|_| matches!(el.tag.as_str(), "list" | "table")) {
        properties.push(A2uiProp {
            name: "source".into(),
            value: serde_json::Value::String(source.to_string()),
            bound: true,
        });
    }
    if let Some(route) = el.route() {
        properties.push(A2uiProp {
            name: "route".into(),
            value: serde_json::Value::String(route.to_string()),
            bound: false,
        });
    }
    if let Some(anchor) = el.anchor() {
        properties.push(A2uiProp {
            name: "anchor".into(),
            value: serde_json::Value::String(anchor.to_string()),
            bound: false,
        });
    }

    let mut intents = Vec::new();
    for action in &el.actions {
        let intent_id = format!("i{}", doc.intents.len());
        doc.intents.push(A2uiIntent {
            id: intent_id.clone(),
            on: match el.tag.as_str() {
                "form" => "submit",
                "check" | "toggle" | "input" | "select" => "change",
                _ => "activate",
            },
            statements: action
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        });
        intents.push(intent_id);
    }

    let mods: Vec<String> = modifiers_of(el).into_iter().map(str::to_string).collect();
    let node_index = doc.components.len();
    doc.components.push(A2uiNode {
        id: id.clone(),
        kind: el.tag.clone(),
        element: crate::element_for(&el.tag),
        class: classes(&el.tag, &modifiers_of(el)),
        text: el
            .content
            .clone()
            .or_else(|| el.label().filter(|_| !is_bind_slot(&el.tag)).map(str::to_string)),
        bind: el
            .binding()
            .map(str::to_string)
            .or_else(|| el.label().filter(|_| is_bind_slot(&el.tag)).map(str::to_string)),
        properties,
        modifiers: mods,
        visible_when,
        children: Vec::new(),
        lines: el.text_lines.clone(),
        intents,
    });

    let mut children = Vec::new();
    for child in &el.children {
        if let Some(child_id) = flatten(child, doc, next, diags) {
            children.push(child_id);
        }
    }
    doc.components[node_index].children = children;
    Some(id)
}

/// Whether a tag's first positional names state rather than being a label.
fn is_bind_slot(tag: &str) -> bool {
    matches!(tag, "input" | "select" | "tabs")
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

// --------------------------------------------------------------------- MCP-UI

/// MCP-UI `UIResource`, composed from the backends that already exist.
///
/// No new format is invented here, which is the whole point. MCP-UI's documented rendering modes are a
/// sandboxed iframe of HTML and a remote-DOM script rendered with host-native components — and GUML
/// already emits both. So the choice is *which existing backend to wrap*, and it is decided by whether
/// the document needs a runtime:
///
/// * **No state, no data, no actions** → `text/html`, from the static-HTML backend. Nothing to run, so
///   the strongest possible sandbox costs nothing.
/// * **Otherwise** → a remote-DOM script, from the Web Components backend. MCP-UI's remote DOM already
///   supports Web Components, so a custom element is the native fit rather than an adaptation.
///
/// Picking by capability rather than by flag matters: a host receiving `text/html` knows there is no
/// script in it, and that knowledge is only worth anything if the compiler never sends HTML for a
/// document that needed behaviour.
#[derive(Debug, Default)]
pub struct McpUiBackend;

#[derive(Serialize)]
struct UiResource {
    uri: String,
    #[serde(rename = "mimeType")]
    mime_type: &'static str,
    /// The payload. `text` rather than `blob`: everything GUML emits is source, and base64 would make
    /// the resource unreadable for no benefit.
    text: String,
    /// What the host must provide for this resource to work. Not part of the protocol's required fields
    /// — extra, and useful: a host can refuse before rendering rather than after.
    #[serde(rename = "_guml")]
    guml: ResourceMeta,
}

#[derive(Serialize)]
struct ResourceMeta {
    page: String,
    /// `core` or `app`. A `core` resource is markup a host can render from an untrusted agent.
    level: &'static str,
    /// True when the resource contains script.
    executable: bool,
    /// Origins the resource will request, so a host can decide before rendering. Empty for `core`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    requests: Vec<String>,
}

impl Backend for McpUiBackend {
    fn name(&self) -> &'static str {
        "mcp-ui"
    }

    fn emit(&self, program: &Program) -> Emitted {
        let mut out = Emitted::default();
        let page = component_name(program.page.as_ref().map(|p| p.name.as_str()).unwrap_or("Page"));

        // Does this document need a runtime? The same question the `core`/`app` split answers, asked of
        // one document rather than of the vocabulary.
        let needs_runtime = !program.states.is_empty()
            || !program.resources.is_empty()
            || !program.effects.is_empty()
            || has_behaviour(&program.tree);

        let (mime_type, inner) = if needs_runtime {
            let emitted = crate::wc::WcBackend.emit(program);
            out.diagnostics.extend(emitted.diagnostics);
            // `application/vnd.mcp-ui.remote-dom` is the media type MCP-UI uses for a remote-DOM script.
            // The script is a module defining a custom element, which the host mounts.
            ("application/vnd.mcp-ui.remote-dom+javascript", emitted.files[0].contents.clone())
        } else {
            let emitted =
                crate::html::HtmlBackend { style: crate::html::Style::Inline }.emit(program);
            out.diagnostics.extend(emitted.diagnostics);
            ("text/html", emitted.files[0].contents.clone())
        };

        let resource = UiResource {
            // `ui://` is MCP-UI's scheme for an embedded UI resource.
            uri: format!("ui://guml/{}", page.to_lowercase()),
            mime_type,
            text: inner,
            guml: ResourceMeta {
                page: page.clone(),
                level: if needs_runtime { "app" } else { "core" },
                executable: needs_runtime,
                requests: request_origins(program),
            },
        };

        let json = serde_json::to_string_pretty(&resource).unwrap_or_else(|_| "{}".into());
        out.files.push(OutFile {
            path: format!("{page}.mcp-ui.json"),
            contents: format!("{json}\n"),
            source_map: None,
        });
        out
    }
}

/// Whether any element in the tree carries behaviour.
fn has_behaviour(els: &[Element]) -> bool {
    els.iter().any(|el| !el.actions.is_empty() || el.tag == "js" || has_behaviour(&el.children))
}

/// Every distinct origin (or same-origin path prefix) the document will request.
///
/// A host deciding whether to render an untrusted resource needs to know what it will talk to *before*
/// it renders it, not from a network log afterwards. Shared with `guml capabilities`, which is where the
/// same list becomes a CSP.
pub fn request_origins(program: &Program) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in &program.resources {
        for url in std::iter::once(&r.url).chain(r.mutations.iter().map(|m| &m.url)) {
            let origin = origin_of(url);
            if !origin.is_empty() && !out.contains(&origin) {
                out.push(origin);
            }
        }
    }
    out.sort();
    out
}

/// The origin of a URL, or `"self"` for a same-origin path.
///
/// Public so `guml_compiler::capabilities` derives its origin list from the same function this emitter
/// uses. A host reading the manifest and a host reading the resource must not be told two different
/// things about the same document.
pub fn request_origins_of(url: &str) -> String {
    origin_of(url)
}

fn origin_of(url: &str) -> String {
    if url.starts_with('/') {
        return "self".to_string();
    }
    let rest = match url.split_once("://") {
        Some((scheme, rest)) => {
            if !matches!(scheme, "http" | "https") {
                return String::new();
            }
            rest
        }
        None => return String::new(),
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() {
        String::new()
    } else {
        let scheme = url.split("://").next().unwrap_or("https");
        format!("{scheme}://{host}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_origin_is_extracted_or_named_self() {
        assert_eq!(origin_of("/api/tasks"), "self");
        assert_eq!(origin_of("https://api.example.com/v1/tasks"), "https://api.example.com");
        assert_eq!(origin_of("http://localhost:3000/x"), "http://localhost:3000");
        // A scheme a browser will not fetch over is not an origin to allow.
        assert_eq!(origin_of("javascript:alert(1)"), "");
        assert_eq!(origin_of("data:text/html,x"), "");
        assert_eq!(origin_of("api/tasks"), "");
    }

    #[test]
    fn a_positional_is_a_label_or_a_binding_depending_on_the_tag() {
        // `input draft` names state; `btn Save` is a label. Getting this backwards would make a host bind
        // a button's text as a state name.
        assert!(is_bind_slot("input"));
        assert!(is_bind_slot("select"));
        assert!(is_bind_slot("tabs"));
        assert!(!is_bind_slot("btn"));
        assert!(!is_bind_slot("card"));
    }
}
