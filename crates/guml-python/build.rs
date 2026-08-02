//! Embed the language spec into the extension.
//!
//! `guml.SPEC` is the ≤3,000-token document you put in a model's system prompt, and for the LLM half
//! of this package's audience it is arguably the most useful thing in it. It has to travel inside the
//! wheel: a `pip install` has no repository to read it from.
//!
//! Copied through `OUT_DIR` rather than `include_str!`-ed across the workspace directly, because a
//! path escaping the crate root works in this repository and breaks the moment the crate is packaged
//! on its own.
//!
//! **Missing spec is a hard failure, not an empty string.** An empty `SPEC` would produce a system
//! prompt with no language definition in it, and the symptom would be a model emitting confident
//! nonsense — a long way from the cause. Invariant 3, applied to a build script.

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let spec = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/GUML-SPEC.md");

    let text = fs::read_to_string(&spec).unwrap_or_else(|e| {
        panic!(
            "cannot read the language spec at {} ({e}).\n\
             It is embedded into the wheel as `guml.SPEC`; building without it would ship a package \
             whose system prompt contains no language definition.",
            spec.display()
        )
    });

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("spec.md");
    fs::write(&out, text).expect("write spec.md to OUT_DIR");

    println!("cargo:rerun-if-changed={}", spec.display());
}
