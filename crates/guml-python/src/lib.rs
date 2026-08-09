//! Python bindings for the GUML compiler.
//!
//! # Shape of this module, and why it is so thin
//!
//! Every function here returns a `String` — source text, or JSON. Nothing constructs a Python object,
//! nothing takes one apart. The dataclasses, keyword defaults, docstrings and type hints all live in
//! `python/guml/__init__.py`, where they are ordinary Python that any Python programmer can read and
//! change without a Rust toolchain.
//!
//! That split is deliberate. Modelling `Diagnostic` in PyO3 would put the public API of a Python
//! package inside a Rust file, behind a compile step, expressed in a DSL — for a struct with five
//! string fields. JSON across the boundary costs a serialise/parse of a few hundred bytes against a
//! compile measured in milliseconds, and buys an API surface that is editable in the language it is an
//! API *for*.
//!
//! # Threading
//!
//! The compiler touches no Python objects, so every entry point releases the GIL for the duration of
//! the work. Without that, a Flask app on threads or a FastAPI endpoint dispatched to the threadpool
//! would serialise every compile through one interpreter lock — the CPU-bound work would look
//! concurrent and behave sequentially, which is the worst of both.
//!
//! # Conformance level
//!
//! `level="core"` compiles against `Registry::core()`: markup only — no `state`, no `data`, no
//! actions, no `js`. It is the setting a server rendering **model-generated** documents wants, because
//! `js` and `raw` compile through unchanged by design. The Python wrapper defaults `render()` to core
//! for exactly that reason; see its docstring.

use guml_registry::Registry;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde_json::json;

/// The language spec.
///
/// Owned by `guml-registry`, which every consumer of the spec already depends on — `guml mcp` serves
/// the same constant. It used to be vendored here as well, which made two copies of one document with
/// two freshness gates; the drift that arrangement invites is the one this repository has spent the
/// most time on.
const SPEC: &str = guml_registry::SPEC;

fn registry_for(level: &str) -> PyResult<Registry> {
    match level {
        "core" => Ok(Registry::core()),
        "app" => Ok(Registry::builtin()),
        other => Err(PyValueError::new_err(format!(
            "unknown level {other:?}: expected \"core\" (markup only — no state, data, actions or js) \
             or \"app\" (the full vocabulary)"
        ))),
    }
}

/// One diagnostic, flattened to the shape `python/guml/__init__.py` turns into a dataclass.
fn diagnostic_json(d: &guml_diagnostics::Diagnostic) -> serde_json::Value {
    json!({
        "code": d.id,
        "severity": format!("{:?}", d.severity).to_lowercase(),
        "message": d.message,
        "line": d.span.line,
        "column": d.span.col,
        "start": d.span.start,
        "end": d.span.end,
        "help": d.help,
        "suggestion": d.suggestion,
    })
}

fn diagnostics_json(diags: &guml_diagnostics::Diagnostics) -> serde_json::Value {
    serde_json::Value::Array(diags.items.iter().map(diagnostic_json).collect())
}

/// The compiler version, which is the crate version, which is the wheel version.
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The embedded language spec.
#[pyfunction]
fn spec() -> &'static str {
    SPEC
}

/// Every backend name the compiler can resolve, read from its own registry of backends.
#[pyfunction]
fn backends() -> Vec<String> {
    guml_codegen::backend_names().iter().map(|s| s.to_string()).collect()
}

/// Parse and analyse. Returns a JSON array of diagnostics — every problem in one pass, never just the
/// first, because a caller feeding this back to a model pays for each round.
#[pyfunction]
#[pyo3(signature = (source, level="app"))]
fn check(py: Python<'_>, source: &str, level: &str) -> PyResult<String> {
    let reg = registry_for(level)?;
    py.detach(|| {
        let (_, diags) = guml_compiler::check_with(source, &reg);
        Ok(diagnostics_json(&diags).to_string())
    })
}

/// Compile to a backend. Returns `{"files": [{"path", "contents"}], "diagnostics": [...]}`.
#[pyfunction]
#[pyo3(signature = (source, backend="react", level="app"))]
fn compile(py: Python<'_>, source: &str, backend: &str, level: &str) -> PyResult<String> {
    let reg = registry_for(level)?;
    if guml_codegen::backend(backend).is_none() {
        return Err(PyValueError::new_err(format!(
            "unknown backend {backend:?}: expected one of {}",
            guml_codegen::backend_names().join(", ")
        )));
    }
    py.detach(|| {
        let opts = guml_compiler::Options { backend: backend.to_string(), registry: reg };
        let result = guml_compiler::compile(source, &opts);
        let files: Vec<_> = result
            .files
            .iter()
            .map(|f| json!({ "path": f.path, "contents": f.contents }))
            .collect();
        Ok(json!({ "files": files, "diagnostics": diagnostics_json(&result.diagnostics) })
            .to_string())
    })
}

/// Format. Idempotent, and preserves comments and blank lines.
#[pyfunction]
fn format(py: Python<'_>, source: &str) -> String {
    py.detach(|| guml_fmt::format_str(source))
}

/// Canonical form: comments and blank lines stripped, directives hoisted and sorted. Two documents
/// that mean the same thing become byte-identical — which is why it is not a formatter.
#[pyfunction]
fn canonical(py: Python<'_>, source: &str) -> String {
    py.detach(|| guml_fmt::format(source, guml_fmt::Options::canonical()).text)
}

/// Apply every unambiguous suggestion, re-checking until nothing changes. No model call.
#[pyfunction]
#[pyo3(signature = (source, max_rounds=3))]
fn fix(py: Python<'_>, source: &str, max_rounds: usize) -> String {
    py.detach(|| {
        let applied = guml_compiler::fix::fix(source, max_rounds);
        json!({ "text": applied.text, "codes": applied.codes, "rounds": applied.rounds })
            .to_string()
    })
}

/// The full mechanical repair pass: sanitise, format, then fix. Everything a repair loop can do
/// without asking a model again.
#[pyfunction]
#[pyo3(signature = (source, max_rounds=3))]
fn repair(py: Python<'_>, source: &str, max_rounds: usize) -> String {
    py.detach(|| {
        let r = guml_compiler::repair::repair(source, max_rounds);
        json!({
            "text": r.text,
            "reformatted": r.reformatted,
            "applied": r.applied,
            "rounds": r.rounds,
        })
        .to_string()
    })
}

/// What a document will actually do — network, storage, script evaluation — and a matching CSP.
#[pyfunction]
#[pyo3(signature = (source, backend="html"))]
fn capabilities(py: Python<'_>, source: &str, backend: &str) -> PyResult<String> {
    py.detach(|| {
        let (program, _) = guml_compiler::check(source);
        let manifest = guml_compiler::capabilities::analyse(&program, source.lines().count());
        let csp = manifest.csp(backend);
        let mut value = serde_json::to_value(&manifest)
            .map_err(|e| PyValueError::new_err(format!("serialising the manifest: {e}")))?;
        value["csp"] = json!(csp);
        Ok(value.to_string())
    })
}

/// The active theme's stylesheet.
///
/// Fragments deliberately do not carry one — a site with fifty fragments wants a single copy in its
/// layout, not fifty beside the content. This is how you get that copy.
#[pyfunction]
fn stylesheet() -> PyResult<String> {
    guml_codegen::theme::active().css.clone().ok_or_else(|| {
        PyValueError::new_err(format!(
            "theme `{}` ships no stylesheet",
            guml_codegen::theme::active().name
        ))
    })
}

/// The component vocabulary, or a prompt-sized slice of it.
///
/// The slice is the point: the assembled prompt stays under budget because a document needs a dozen
/// tags, not the whole registry.
#[pyfunction]
#[pyo3(signature = (tags=None))]
fn registry(py: Python<'_>, tags: Option<Vec<String>>) -> String {
    py.detach(|| {
        let reg = Registry::builtin();

        // `None` means the whole vocabulary, not an empty slice. `prompt_context` iterates the names
        // it is given, so passing `&[]` returns an empty string — and the caller's system prompt would
        // then describe no tags at all while looking perfectly well-formed. The model is told the
        // language has no vocabulary and confidently invents one.
        let owned: Vec<String> = match tags {
            Some(t) => t,
            None => reg.names().map(str::to_string).collect(),
        };
        let names: Vec<&str> = owned.iter().map(String::as_str).collect();
        reg.prompt_context(&names)
    })
}

#[pymodule]
fn _guml(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(spec, m)?)?;
    m.add_function(wrap_pyfunction!(backends, m)?)?;
    m.add_function(wrap_pyfunction!(check, m)?)?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(format, m)?)?;
    m.add_function(wrap_pyfunction!(canonical, m)?)?;
    m.add_function(wrap_pyfunction!(fix, m)?)?;
    m.add_function(wrap_pyfunction!(repair, m)?)?;
    m.add_function(wrap_pyfunction!(capabilities, m)?)?;
    m.add_function(wrap_pyfunction!(stylesheet, m)?)?;
    m.add_function(wrap_pyfunction!(registry, m)?)?;
    Ok(())
}
