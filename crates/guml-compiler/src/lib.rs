//! Compiler driver.
//!
//! One entry point, because the LLM repair loop calls it in a hot path and wants a single
//! structured answer: emitted files plus every diagnostic from every phase, collected in one
//! pass (report §6.7).

use guml_ast::Program;
use guml_codegen::{Backend, Emitted, OutFile};
use guml_diagnostics::Diagnostics;
use guml_registry::Registry;

pub mod sema;

pub use guml_codegen::backend_names;

#[derive(Debug, Clone)]
pub struct Options {
    pub backend: String,
}

impl Default for Options {
    fn default() -> Self {
        Self { backend: "react".to_string() }
    }
}

#[derive(Debug, Default)]
pub struct CompileResult {
    pub program: Program,
    pub files: Vec<OutFile>,
    pub diagnostics: Diagnostics,
    pub stats: Stats,
}

impl CompileResult {
    pub fn ok(&self) -> bool {
        !self.diagnostics.has_errors()
    }
}

/// Size accounting. `approx_tokens` is a cheap heuristic, and is labelled as such everywhere
/// it surfaces: real figures for the paper must come from the target model's own tokenizer
/// (report §8.6 — `tiktoken` is an OpenAI tokenizer and undercounts Claude tokens).
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub source_bytes: usize,
    pub source_lines: usize,
    pub emitted_bytes: usize,
    pub approx_source_tokens: usize,
    pub approx_emitted_tokens: usize,
}

impl Stats {
    pub fn ratio(&self) -> f64 {
        if self.approx_source_tokens == 0 {
            0.0
        } else {
            self.approx_emitted_tokens as f64 / self.approx_source_tokens as f64
        }
    }
}

/// Rough BPE estimate for code-like text (~3.6 chars/token). Order-of-magnitude only.
pub fn approx_tokens(s: &str) -> usize {
    (s.len() as f64 / 3.6).ceil() as usize
}

/// Parse and analyse — used by `guml check`, the LSP, and the repair loop's fast
/// path. Both phases run unconditionally so a single call reports every problem.
pub fn check(src: &str) -> (Program, Diagnostics) {
    let reg = Registry::builtin();
    let parsed = guml_parser::parse(src, &reg);
    let mut diagnostics = parsed.diagnostics;
    sema::analyse(&parsed.program, &reg, &mut diagnostics);
    (parsed.program, diagnostics)
}

pub fn compile(src: &str, opts: &Options) -> CompileResult {
    let (program, mut diagnostics) = check(src);

    let mut files = Vec::new();
    if let Some(b) = guml_codegen::backend(&opts.backend) {
        // Only run codegen when the front end produced something coherent; emitting from a
        // broken AST produces confusing secondary errors.
        if !diagnostics.has_errors() {
            let Emitted { files: f, diagnostics: d } = b.emit(&program);
            files = f;
            diagnostics.extend(d);
        }
    }

    let emitted_bytes: usize = files.iter().map(|f| f.contents.len()).sum();
    let emitted_src: String = files.iter().map(|f| f.contents.as_str()).collect();

    let stats = Stats {
        source_bytes: src.len(),
        source_lines: src.lines().count(),
        emitted_bytes,
        approx_source_tokens: approx_tokens(src),
        approx_emitted_tokens: approx_tokens(&emitted_src),
    };

    CompileResult { program, files, diagnostics, stats }
}

/// Backend list for `--help` output and the CLI's validation.
pub fn resolve_backend(name: &str) -> Option<Box<dyn Backend>> {
    guml_codegen::backend(name)
}
