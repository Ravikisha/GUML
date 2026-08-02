//! The whole front end, on arbitrary bytes.
//!
//! What is asserted is what a *host* depends on, not that the output is correct — for garbage input
//! there is no correct output:
//!
//! 1. **No panic.** A GUML document may come from an untrusted agent. A panic in the wasm build takes
//!    the page down; in the language server it takes the editor's diagnostics with it.
//! 2. **Spans stay inside the source and on char boundaries.** A span past the end makes an editor
//!    throw when it highlights it, and makes `guml fix` slice out of bounds.
//!
//! Termination is checked by libFuzzer's own timeout rather than asserted here.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };

    let (program, diags) = guml_compiler::check(src);

    for d in &diags.items {
        assert!(
            d.span.start <= d.span.end && d.span.end <= src.len(),
            "{} has span {}..{} in a {}-byte document",
            d.id,
            d.span.start,
            d.span.end,
            src.len()
        );
        assert!(
            src.is_char_boundary(d.span.start) && src.is_char_boundary(d.span.end),
            "{} span is not on a char boundary",
            d.id
        );
        assert!(d.span.line >= 1, "{} reports line 0", d.id);
    }

    // Codegen only runs on documents that parsed, which mirrors the driver.
    //
    // Enumerated from `backend_names()` rather than by hand. The hand-written list named four backends and
    // the compiler had grown to eight — `wc`, `a2ui` and `mcp-ui` were never fuzzed at all, which is the
    // same drift the element table, the class table and the tree-sitter tag list each suffered from. A
    // backend added after this file was written is now fuzzed the day it is registered.
    if !diags.has_errors() {
        for name in guml_compiler::backend_names() {
            if let Some(backend) = guml_codegen::backend(name) {
                let _ = backend.emit(&program);
            }
        }
    }
});
