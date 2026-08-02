//! The expression parser and its JavaScript lowering.
//!
//! The newest component and the one consuming the least structured input, so it gets its own target
//! rather than being reached only through whole documents. Both entry points are exercised: the tree,
//! and the lowering that consumes it — a panic in either would surface in the browser runtime.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };

    let parsed = guml_syntax::expr::parse(src);
    // Tree walks: a cycle or unbounded recursion would show here.
    let _ = parsed.idents();
    let _ = parsed.head_ident();
    let _ = parsed.is_computed();

    // The lowering the emitted code and the browser runtime both rely on.
    let _ = guml_codegen::expr::lower(src);
    let _ = guml_codegen::expr::lower_text(src);
});
