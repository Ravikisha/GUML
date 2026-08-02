//! The agent-UI emitters: A2UI and MCP-UI.
//!
//! These exist because of a strategic read, not a technical one. The report (§13) is blunt: A2UI is
//! simultaneously the strongest competitor and the strongest partner, the concept "an agent emits a
//! declarative UI description against a host-approved catalog" is already standardised and shipping, and
//! what those protocols lack is a token-efficient surface syntax and a compiler. Emitting them turns
//! "GUML vs A2UI" into "GUML compiles to A2UI".
//!
//! What these tests hold down is the **security posture**, because it is the part that is easy to get
//! subtly wrong and impossible to notice: a payload that quietly lost its behaviour renders as a page
//! that looks finished and does nothing, and a payload that quietly gained executable code defeats the
//! reason the protocol is non-executable.

use guml_codegen::Backend as _;
use guml_compiler::check;
use serde_json::Value;

fn emit(src: &str, backend: &str) -> (Value, Vec<String>) {
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let out = guml_codegen::backend(backend).expect("backend exists").emit(&program);
    let json: Value =
        serde_json::from_str(&out.files[0].contents).expect("the emitter must produce valid JSON");
    (json, out.diagnostics.items.iter().map(|d| d.message.clone()).collect())
}

const TASKS: &str = r#"page Tasks

type Task {id, title, done:bool}
data tasks:Task[] GET /api/tasks
  add POST /api/tasks {title} optimistic:prepend

state draft=""
state filter=all|open|done

head Tasks — {tasks.open.count} open

form >tasks.add{title:draft}; draft=""
  input draft aria="New task"
  btn Add primary disabled={!draft.trim()}

tabs filter

list tasks where={filter}
  text {title}
  empty Nothing here yet.
"#;

/* ------------------------------------------------------------------ A2UI */

#[test]
fn the_payload_is_flat_with_children_by_reference() {
    // The protocol's own choice, and a good one: a flat list is incrementally updateable, so an agent can
    // patch one component without re-sending the tree.
    let (doc, _) = emit(TASKS, "a2ui");
    let components = doc["components"].as_array().expect("a component list");
    assert!(components.len() > 5, "expected the whole tree flattened, got {}", components.len());

    // Every id a component references must exist, or the host renders a hole.
    let ids: std::collections::BTreeSet<&str> =
        components.iter().filter_map(|c| c["id"].as_str()).collect();
    for c in components {
        for child in c["children"].as_array().unwrap_or(&vec![]) {
            let id = child.as_str().expect("a child reference is an id string");
            assert!(ids.contains(id), "`{id}` is referenced but not present");
        }
    }
    for root in doc["roots"].as_array().expect("roots") {
        assert!(ids.contains(root.as_str().unwrap()), "a root is not in the component list");
    }
    // No component is orphaned: everything is reachable from a root.
    let referenced: std::collections::BTreeSet<&str> = components
        .iter()
        .flat_map(|c| c["children"].as_array().cloned().unwrap_or_default())
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect::<Vec<_>>()
        .iter()
        .map(|s| ids.get(s.as_str()).copied().unwrap_or(""))
        .collect();
    let roots: std::collections::BTreeSet<&str> =
        doc["roots"].as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    for id in &ids {
        assert!(
            roots.contains(id) || referenced.contains(id),
            "`{id}` is in the payload but nothing points at it"
        );
    }
}

#[test]
fn the_catalog_states_what_the_host_must_be_able_to_render() {
    // A host holds a pre-approved catalog and may only render from it. Stating the requirement up front
    // lets it refuse the whole payload rather than render half of one.
    let (doc, _) = emit(TASKS, "a2ui");
    let catalog: Vec<&str> =
        doc["catalog"].as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    let used: std::collections::BTreeSet<&str> =
        doc["components"].as_array().unwrap().iter().filter_map(|c| c["type"].as_str()).collect();
    assert_eq!(catalog.len(), used.len(), "the catalog and the payload disagree");
    for kind in used {
        assert!(catalog.contains(&kind), "`{kind}` is used but not in the catalog");
    }
    // The *GUML tag*, not the HTML element: a catalog is a catalog of components, and `card` carries
    // intent that `div` has thrown away.
    assert!(catalog.contains(&"head"), "{catalog:?}");
    assert!(!catalog.contains(&"p"), "the catalog was translated to HTML elements: {catalog:?}");
}

#[test]
fn nothing_executable_travels_in_the_payload() {
    // The reason the protocol exists in this shape. An action becomes a declared *intent* — the author's
    // statements, unlowered — so a host may honour it, map it onto its own action, or refuse. Lowering it
    // to JavaScript would hand a host exactly what non-executable was protecting it from.
    let (doc, _) = emit(TASKS, "a2ui");
    let intents = doc["intents"].as_array().expect("intents");
    assert!(!intents.is_empty(), "the form's action was dropped entirely");
    let text = serde_json::to_string(&doc).unwrap();
    for js in ["=>", "function", "setDraft", "this.#state", "useState"] {
        assert!(!text.contains(js), "lowered JavaScript (`{js}`) reached the payload:\n{text}");
    }
    // The statements are the author's, recognisable as GUML rather than as code.
    let statements = intents[0]["statements"].as_array().unwrap();
    assert!(statements.iter().any(|s| s.as_str().unwrap().contains("tasks.add")), "{statements:?}");
}

#[test]
fn a_js_block_is_refused_rather_than_silently_stripped() {
    // A payload whose behaviour vanished with no diagnostic renders as a page that looks complete and
    // does nothing — strictly worse than a refusal, because nobody knows to look.
    let src = "page P\njs\n  const x = 1;\np Body.\n";
    let (doc, warnings) = emit(src, "a2ui");
    assert!(
        warnings.iter().any(|w| w.contains("declarative by design")),
        "the `js` block was dropped without a word: {warnings:?}"
    );
    let text = serde_json::to_string(&doc).unwrap();
    assert!(!text.contains("const x"), "executable code reached the payload:\n{text}");
    // And the rest of the document still made it.
    assert!(text.contains("Body."), "the document was abandoned rather than partially emitted");
}

#[test]
fn a_data_requirement_is_declared_rather_than_performed() {
    // The host decides whether to issue the request. That is the only version of this that is safe when
    // the payload came from an untrusted agent.
    let (doc, _) = emit(TASKS, "a2ui");
    let data = doc["data"].as_array().expect("data requirements");
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["name"], "tasks");
    assert_eq!(data[0]["method"], "GET");
    assert_eq!(data[0]["url"], "/api/tasks");
    // Mutations travel as declarations too, with the optimistic strategy the author chose.
    assert_eq!(data[0]["mutations"][0]["optimistic"], "prepend");
}

#[test]
fn a_bound_positional_is_a_binding_and_a_label_is_text() {
    // `input draft` names state; `btn Add` is a label. Getting this backwards would make a host bind a
    // button's text as a state name.
    let (doc, _) = emit(TASKS, "a2ui");
    let by_type = |t: &str| -> Value {
        doc["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == t)
            .cloned()
            .unwrap_or(Value::Null)
    };
    assert_eq!(by_type("input")["bind"], "draft");
    assert!(by_type("input")["text"].is_null(), "a state name leaked out as text");
    assert_eq!(by_type("btn")["text"], "Add");
    assert!(by_type("btn")["bind"].is_null());
    assert_eq!(by_type("tabs")["bind"], "filter");
}

#[test]
fn the_format_does_not_claim_conformance_it_has_not_earned() {
    // Written from a description of the protocol rather than against its published schema. Saying so in
    // the payload is the difference between an honest emitter and an unsupported claim.
    let (doc, _) = emit(TASKS, "a2ui");
    assert_eq!(doc["format"], "a2ui-shaped");
    assert!(doc["version"].is_string());
}

/* ---------------------------------------------------------------- MCP-UI */

#[test]
fn a_document_needing_no_runtime_becomes_sandboxable_html() {
    // The strongest sandbox costs nothing when there is nothing to run, so a host should get `text/html`
    // whenever that is truthful.
    let src = std::fs::read_to_string("../../fixtures/c.guml").expect("c.guml");
    let (doc, _) = emit(&src, "mcp-ui");
    assert_eq!(doc["mimeType"], "text/html");
    assert_eq!(doc["_guml"]["level"], "core");
    assert_eq!(doc["_guml"]["executable"], false);
    assert!(doc["text"].as_str().unwrap().starts_with("<!doctype html>"));
    // A `core` resource requests nothing, so a host has nothing to allow.
    assert!(doc["_guml"]["requests"].is_null(), "{}", doc["_guml"]);
}

#[test]
fn a_document_needing_a_runtime_becomes_a_remote_dom_script() {
    // MCP-UI's remote DOM already supports Web Components, so a custom element is the native fit rather
    // than an adaptation — and the wc backend already emits one.
    let (doc, _) = emit(TASKS, "mcp-ui");
    assert_eq!(doc["mimeType"], "application/vnd.mcp-ui.remote-dom+javascript");
    assert_eq!(doc["_guml"]["level"], "app");
    assert_eq!(doc["_guml"]["executable"], true);
    assert!(doc["text"].as_str().unwrap().contains("customElements.define"));
    assert!(doc["uri"].as_str().unwrap().starts_with("ui://guml/"));
}

#[test]
fn the_choice_is_made_by_capability_not_by_a_flag() {
    // A host receiving `text/html` knows there is no script in it, and that knowledge is worth nothing
    // unless the compiler never sends HTML for a document that needed behaviour. Each of these three
    // things alone is enough to require a runtime.
    for src in [
        "page P\nstate n=0\n\nmetric {n}\n",
        "page P\ntype T {id}\ndata rows:T[] GET /api/rows\n\nlist rows\n  text {id}\n  empty None.\n",
        "page P\nbtn Go >nowhere\n",
    ] {
        let (program, _) = check(src);
        let out = guml_codegen::agent::McpUiBackend.emit(&program);
        let doc: Value = serde_json::from_str(&out.files[0].contents).unwrap();
        assert_eq!(
            doc["_guml"]["executable"], true,
            "a document with behaviour was sent as inert HTML:\n{src}"
        );
    }
    // And a document with none of them is not upgraded needlessly.
    let (program, _) = check("page P\ncard \"Title\"\n  p Body.\n");
    let out = guml_codegen::agent::McpUiBackend.emit(&program);
    let doc: Value = serde_json::from_str(&out.files[0].contents).unwrap();
    assert_eq!(doc["_guml"]["executable"], false);
}

#[test]
fn an_absolute_url_keeps_its_scheme() {
    // A lexer bug the origin list uncovered. `https://api.example.com/rows` lexed as the word `https`, a
    // `:`, and a route `//api.example.com/rows` — so the scheme the author wrote was discarded and the
    // emitted code fetched a *protocol-relative* URL. It worked in a browser, which is why it survived,
    // and it meant `validate::check_url`'s `starts_with("http")` branch was unreachable: no absolute URL
    // ever reached it intact.
    let src = "page P\ntype T {id}\ndata rows:T[] GET https://api.example.com/rows\n\nlist rows\n  text {id}\n  empty None.\n";
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    assert_eq!(program.resources[0].url, "https://api.example.com/rows");

    // Prose is unaffected: a URL in a sentence is still a sentence.
    let (program, _) = check("page P\np Visit https://example.com/docs for more.\n");
    assert_eq!(
        program.tree[0].content.as_deref(),
        Some("Visit https://example.com/docs for more."),
        "a URL in prose was tokenised"
    );

    // Only `http` and `https`. A general scheme rule would make these lexable as request targets, and a
    // URL a document can name is a URL the compiler emits a fetch to.
    for hostile in ["javascript://x/y", "data://text/html"] {
        let src = format!(
            "page P\ntype T {{id}}\ndata rows:T[] GET {hostile}\n\nlist rows\n  text {{id}}\n  empty None.\n"
        );
        let (_, diags) = check(&src);
        assert!(
            diags.has_errors(),
            "`{hostile}` was accepted as a request target: {:?}",
            diags.items
        );
    }
}

#[test]
fn the_origins_a_resource_will_contact_are_declared_before_it_renders() {
    // A host deciding whether to render an untrusted resource needs this *before* rendering, not from a
    // network log afterwards. The same list becomes a CSP in `guml capabilities`.
    let src = "page P\ntype T {id}\ndata rows:T[] GET https://api.example.com/rows\n  save PATCH /local/rows/{id} {id}\n\nlist rows\n  text {id}\n  empty None.\n";
    let (doc, _) = emit(src, "mcp-ui");
    let requests: Vec<&str> =
        doc["_guml"]["requests"].as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert!(requests.contains(&"https://api.example.com"), "{requests:?}");
    assert!(requests.contains(&"self"), "a same-origin path was not reported: {requests:?}");
}
