//! WebAssembly bindings for the GUML formatter and syntax classifier.
//!
//! **Separate from `guml-wasm` because of what it does not depend on.** `guml-fmt` sits below the
//! parser: it needs `guml-syntax`, `guml-registry` and `guml-diagnostics`, and nothing else. No parser,
//! no semantic analysis, no code generation, no backends.
//!
//! That is worth a second artifact rather than a feature flag, because the difference is not marginal:
//!
//! | build | size |
//! |---|---|
//! | `guml-wasm` — the whole compiler | 787 KB |
//! | this — formatter and classifier | 178 KB |
//!
//! A tool that formats GUML, or colours it, has no reason to download the code generator for seven
//! backends. Anyone who *does* need to compile keeps using `@guml/core`, which is unchanged and still
//! exposes `format` and `highlight` from these same Rust functions — this is a smaller door to the same
//! room, not a fork.
//!
//! The `String` returns are deliberate. `serde-wasm-bindgen` would let these hand back structured
//! values, but it is a dependency the full build already pays for and this one does not, and JSON the
//! caller parses costs nothing measurable next to a 600 KB saving.

use wasm_bindgen::prelude::*;

/// Format a document. Idempotent: formatting formatted output returns it unchanged.
#[wasm_bindgen]
pub fn format(src: &str) -> String {
    guml_fmt::format_str(src)
}

/// Canonical form: comments and blank lines stripped, directives hoisted and sorted, the shortest
/// spelling of every value preferred.
///
/// Two documents that mean the same thing become byte-identical, which is what makes independent
/// generations of one interface comparable. Not what you want in an editor — this is a normaliser, and
/// it deletes commentary on purpose.
#[wasm_bindgen]
pub fn canonical(src: &str) -> String {
    guml_fmt::format(src, guml_fmt::Options::canonical()).text
}

/// Classify every byte for highlighting, as JSON:
/// `[{ "line": 1, "start": 0, "end": 4, "class": "tag" }]`.
///
/// The same classifier `guml highlight` uses, so the class names are the compiler's own. For a page
/// that renders on a server, `@guml/highlight` does this in ~15 KB of TypeScript with no wasm at all,
/// held to this implementation by a parity test — prefer it unless you need exactness by construction
/// rather than by test.
#[wasm_bindgen]
pub fn highlight(src: &str) -> String {
    guml_fmt::highlight::to_json(src)
}

/// The crate version this wasm was built from.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
