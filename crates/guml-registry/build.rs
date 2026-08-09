//! Embed the language spec into the registry crate.
//!
//! The ≤3,000-token document that goes in a model's system prompt — or, better, comes back from the
//! MCP server's `guml_spec` tool. It has to travel inside every published artifact: a `pip install`,
//! a `cargo install` and an `npm install` all have no repository to read it from.
//!
//! **It lives here rather than in each consumer.** `guml-cli` needs it for `guml mcp`, and
//! `guml-python` needs it for `guml.SPEC`; both already depend on this crate, so one vendored copy
//! with one freshness gate serves both. Three copies of a specification is precisely the drift this
//! repository keeps being bitten by.
//!
//! # Why there are two copies of the spec, and why that is safe
//!
//! The original lives at the workspace root. maturin refuses an `include` pattern containing `..`, and
//! vendors nothing above the crate directory — so an sdist built without a crate-local copy is
//! *unbuildable*, which is not a thing to discover from a bug report. That was verified by extracting
//! the tarball and building it, not by reading a file list.
//!
//! So `spec/GUML-SPEC.md` here is a committed copy, and **this build script is its freshness gate**:
//! when the workspace original is reachable it is the source of truth, and a stale copy fails the
//! build with the command to fix it. Building the crate at all therefore verifies the copy, which
//! means there is no separate check to remember to run.
//!
//! # Missing spec is a hard failure, not an empty string
//!
//! An empty `SPEC` would produce a system prompt containing no language definition, and the symptom —
//! a model emitting confident nonsense — is a very long way from the cause. Invariant 3, applied to a
//! build script.

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_spec = crate_dir.join("../../spec/GUML-SPEC.md");
    let vendored_spec = crate_dir.join("spec/GUML-SPEC.md");

    println!("cargo:rerun-if-changed={}", workspace_spec.display());
    println!("cargo:rerun-if-changed={}", vendored_spec.display());

    let text = match fs::read_to_string(&workspace_spec) {
        // Building inside the repository: the workspace copy wins, and the vendored one must match it.
        Ok(original) => {
            match fs::read_to_string(&vendored_spec) {
                Ok(copy) if copy == original => {}
                Ok(_) => panic!(
                    "crates/guml-registry/spec/GUML-SPEC.md is stale.\n\
                     It is a committed copy of spec/GUML-SPEC.md, needed because maturin cannot \
                     include a file above the crate directory and an sdist without it does not build.\n\
                     Refresh it:  cp spec/GUML-SPEC.md crates/guml-registry/spec/GUML-SPEC.md"
                ),
                Err(_) => panic!(
                    "crates/guml-registry/spec/GUML-SPEC.md is missing.\n\
                     Create it:   cp spec/GUML-SPEC.md crates/guml-registry/spec/GUML-SPEC.md"
                ),
            }
            original
        }
        // Building from an sdist: only the vendored copy exists, and that is the case it is for.
        Err(_) => fs::read_to_string(&vendored_spec).unwrap_or_else(|e| {
            panic!(
                "cannot read the language spec from either {} or {} ({e}).\n\
                 It is embedded as `guml.SPEC`; building without it would ship a package whose \
                 system prompt contains no language definition.",
                workspace_spec.display(),
                vendored_spec.display()
            )
        }),
    };

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("spec.md");
    fs::write(&out, text).expect("write spec.md to OUT_DIR");
}
