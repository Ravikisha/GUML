//! WebAssembly bindings for the GUML compiler.
//!
//! The same Rust that powers the CLI, compiled to wasm32 — so a browser gets the
//! *actual* compiler rather than a re-implementation that can drift from it. That
//! matters for the two things this enables: a live preview whose classes match
//! emitted code exactly, and a playground whose diagnostics are the real ones.
//!
//! Everything crosses the boundary as plain JSON-shaped values via
//! `serde-wasm-bindgen`, so the TypeScript wrapper needs no glue beyond types.

use guml_diagnostics::Diagnostics;
use guml_registry::Registry;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckResult {
    ok: bool,
    error_count: usize,
    diagnostics: Vec<guml_diagnostics::Diagnostic>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct File {
    path: String,
    contents: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompileResult {
    ok: bool,
    files: Vec<File>,
    diagnostics: Vec<guml_diagnostics::Diagnostic>,
    stats: Stats,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    source_bytes: usize,
    source_lines: usize,
    emitted_bytes: usize,
    approx_source_tokens: usize,
    approx_emitted_tokens: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeResult {
    ok: bool,
    tree: guml_codegen::json::UiTree,
    diagnostics: Vec<guml_diagnostics::Diagnostic>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryEntry {
    name: String,
    kind: String,
    doc: String,
    requires_label: bool,
    attrs: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryResult {
    components: Vec<RegistryEntry>,
    modifiers: Vec<String>,
    global_attrs: Vec<String>,
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse and analyse. Returns every diagnostic in one pass, which is what keeps
/// an editor or a repair loop to a single round trip.
#[wasm_bindgen]
pub fn check(source: &str) -> Result<JsValue, JsValue> {
    let (_, diags) = guml_compiler::check(source);
    to_js(&CheckResult {
        ok: !diags.has_errors(),
        error_count: diags.error_count(),
        diagnostics: diags.items,
    })
}

/// Compile to a backend: `"react"` for source text, `"json"` for a render tree.
#[wasm_bindgen]
pub fn compile(source: &str, backend: Option<String>) -> Result<JsValue, JsValue> {
    let backend = backend.unwrap_or_else(|| "react".to_string());
    if guml_compiler::resolve_backend(&backend).is_none() {
        return Err(JsValue::from_str(&format!(
            "unknown backend `{backend}` (available: {})",
            guml_compiler::backend_names().join(", ")
        )));
    }
    let res =
        guml_compiler::compile(source, &guml_compiler::Options { backend, ..Default::default() });
    to_js(&CompileResult {
        ok: res.ok(),
        files: res
            .files
            .iter()
            .map(|f| File { path: f.path.clone(), contents: f.contents.clone() })
            .collect(),
        diagnostics: res.diagnostics.items,
        stats: Stats {
            source_bytes: res.stats.source_bytes,
            source_lines: res.stats.source_lines,
            emitted_bytes: res.stats.emitted_bytes,
            approx_source_tokens: res.stats.approx_source_tokens,
            approx_emitted_tokens: res.stats.approx_emitted_tokens,
        },
    })
}

/// The render tree, for the runtime renderer. Diagnostics come along so a preview
/// can show the problem instead of rendering something misleading.
#[wasm_bindgen]
pub fn tree(source: &str) -> Result<JsValue, JsValue> {
    let (program, diags) = guml_compiler::check(source);
    let mut sink = Diagnostics::new();
    let tree = guml_codegen::json::ui_tree(&program, &mut sink);
    to_js(&TreeResult { ok: !diags.has_errors(), tree, diagnostics: diags.items })
}

/// The component vocabulary. `tags` narrows it to a prompt-sized slice.
#[wasm_bindgen]
pub fn registry(tags: Option<String>) -> Result<JsValue, JsValue> {
    let reg = Registry::builtin();
    let wanted: Option<Vec<String>> = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect());

    let components = reg
        .names()
        .filter(|n| wanted.as_ref().is_none_or(|w| w.iter().any(|x| x == n)))
        .filter_map(|n| reg.get(n))
        .map(|c| RegistryEntry {
            name: c.name.to_string(),
            kind: format!("{:?}", c.kind),
            doc: c.doc.to_string(),
            requires_label: c.requires_label(),
            attrs: c.attrs.iter().map(|a| a.to_string()).collect(),
        })
        .collect();

    to_js(&RegistryResult {
        components,
        modifiers: guml_registry::MODIFIERS.iter().map(|m| m.to_string()).collect(),
        global_attrs: guml_registry::GLOBAL_ATTRS.iter().map(|a| a.to_string()).collect(),
    })
}

/// Format source. `canonical` strips comments, blank lines and declaration order so that
/// two semantically identical documents produce identical bytes.
#[wasm_bindgen]
pub fn format(source: &str, canonical: Option<bool>) -> Result<JsValue, JsValue> {
    let out = guml_fmt::format(source, guml_fmt::Options { canonical: canonical.unwrap_or(false) });
    to_js(&FormatResult { text: out.text, changed: out.changed })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FormatResult {
    text: String,
    changed: bool,
}

/// Apply every unambiguous diagnostic suggestion, with no model in the loop.
///
/// The free layer of the repair loop. The harness runs this through the CLI; the browser gets
/// the same implementation so a page can fix what the compiler already knows how to fix
/// before it tells anyone the generation failed.
#[wasm_bindgen]
pub fn fix(source: &str, rounds: Option<usize>) -> Result<JsValue, JsValue> {
    let out = guml_compiler::fix::fix(source, rounds.unwrap_or(3));
    to_js(&FixResult { text: out.text, codes: out.codes, rounds: out.rounds })
}

#[derive(Serialize)]
struct FixResult {
    text: String,
    codes: Vec<String>,
    rounds: usize,
}

/// The whole free repair pipeline: sanitise, format, fix. No model call.
///
/// `fix` only applies edits the compiler described. This also *removes* what a model wrapped around the
/// document — a ``` fence, markdown rules, trailing commentary — which is a different promise and so a
/// different function rather than a flag.
///
/// Exposed to the browser for the same reason `fix` is: a playground or a chat surface that shows
/// "generation failed" before running the free layers is reporting a failure the tool could have
/// repaired itself.
#[wasm_bindgen]
pub fn repair(source: &str, rounds: Option<usize>) -> Result<JsValue, JsValue> {
    let out = guml_compiler::repair::repair(
        source,
        rounds.unwrap_or(guml_compiler::repair::DEFAULT_ROUNDS),
    );
    // Derived values read before anything is moved out of `out`.
    let (ok, changed, report) = (out.ok(), out.changed(), out.report());
    to_js(&RepairResult {
        text: out.text,
        ok,
        changed,
        errors_before: out.errors_before,
        errors_after: out.errors_after,
        sanitize: SanitizeReport {
            fence: out.stripped.fence,
            rules: out.stripped.rules,
            trailing: out.stripped.trailing,
        },
        reformatted: out.reformatted,
        applied: out.applied,
        rounds: out.rounds,
        report,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RepairResult {
    text: String,
    ok: bool,
    changed: bool,
    errors_before: usize,
    errors_after: usize,
    sanitize: SanitizeReport,
    reformatted: bool,
    applied: Vec<String>,
    rounds: usize,
    /// One human-readable line per layer that did something.
    report: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SanitizeReport {
    fence: bool,
    rules: usize,
    trailing: usize,
}

/// Syntax classification from the real lexer and registry, so a browser highlighter cannot
/// drift from the compiler. Returns one entry per coloured span, in source order.
#[wasm_bindgen]
pub fn highlight(source: &str) -> Result<JsValue, JsValue> {
    let spans: Vec<HighlightSpan> = guml_fmt::highlight::classify(source)
        .into_iter()
        .map(|s| HighlightSpan {
            start: s.start,
            end: s.end,
            line: s.line,
            class: s.class.name().to_string(),
            lsp: s.class.lsp_type().to_string(),
        })
        .collect();
    to_js(&spans)
}

#[derive(Serialize)]
struct HighlightSpan {
    start: usize,
    end: usize,
    line: u32,
    class: String,
    lsp: String,
}

/// Compiler version, so a host can report which build produced a result.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    // Host-side tests: the binding layer is thin, so what matters is that the
    // shapes serialise and that the wasm target sees the same behaviour as the CLI.
    #[test]
    fn check_serialises() {
        let (_, diags) = guml_compiler::check("page P\nstate n=0\n\nbtn Go primary >n++\n");
        assert!(!diags.has_errors());
    }

    #[test]
    fn tree_has_nodes_and_classes() {
        let (program, _) = guml_compiler::check("page P\ncard sm\n  h Hi\n");
        let mut sink = guml_diagnostics::Diagnostics::new();
        let tree = guml_codegen::json::ui_tree(&program, &mut sink);
        assert_eq!(tree.nodes[0].tag, "card");
        assert!(tree.nodes[0].class.contains("rounded-xl"));
    }

    #[test]
    fn unknown_backend_is_rejected() {
        // `svelte` used to be the example of a rejected name here, and the assertion outlived the fact:
        // adding the backend made a passing test into a false one. A name nothing will ever claim is the
        // stable way to test the negative.
        assert!(guml_compiler::resolve_backend("qt-widgets").is_none());
        assert!(guml_compiler::resolve_backend("").is_none());
    }

    #[test]
    fn every_shipped_backend_resolves() {
        for name in ["json", "react", "svelte", "html"] {
            assert!(guml_compiler::resolve_backend(name).is_some(), "{name} should resolve");
        }
    }
}
