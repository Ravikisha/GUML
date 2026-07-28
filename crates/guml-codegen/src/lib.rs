//! Code generation.
//!
//! The generator is where "convention as compression" is cashed out (report §4.C.4): the
//! model never emits a Tailwind class string, a `useState` setter, an `aria-label`, or a
//! loading state — the compiler owns all of it, which is simultaneously a token saving and a
//! correctness guarantee.
//!
//! **v0.1 scope is deliberately a vertical slice.** The React backend lowers containers, text,
//! and controls end to end (fixture A compiles and runs). Repeaters, forms, resources and
//! optimistic mutations are *reported as unsupported* rather than silently mis-lowered — an
//! honest partial compiler is useful; a quietly wrong one is not.

use guml_ast::{Element, Positional, Program};
use guml_diagnostics::{Diagnostics, Severity, Span};

pub mod expr;
pub mod json;
pub mod react;

#[derive(Debug, Clone)]
pub struct OutFile {
    pub path: String,
    pub contents: String,
}

#[derive(Debug, Clone, Default)]
pub struct Emitted {
    pub files: Vec<OutFile>,
    pub diagnostics: Diagnostics,
}

pub trait Backend {
    fn name(&self) -> &'static str;
    fn emit(&self, program: &Program) -> Emitted;
}

/// Backends registered in v0.1.
pub fn backend(name: &str) -> Option<Box<dyn Backend>> {
    match name {
        "react" => Some(Box::new(react::ReactBackend)),
        "json" => Some(Box::new(json::JsonBackend)),
        _ => None,
    }
}

pub fn backend_names() -> &'static [&'static str] {
    &["react", "json"]
}

// ------------------------------------------------------------------ shared helpers

/// `count` -> `setCount`
pub(crate) fn setter(name: &str) -> String {
    let mut c = name.chars();
    match c.next() {
        Some(f) => format!("set{}{}", f.to_uppercase(), c.as_str()),
        None => "set".to_string(),
    }
}

/// PascalCase component name from a page name.
pub(crate) fn component_name(raw: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for ch in raw.chars() {
        if ch.is_alphanumeric() {
            if upper {
                out.extend(ch.to_uppercase());
                upper = false;
            } else {
                out.push(ch);
            }
        } else {
            upper = true;
        }
    }
    if out.is_empty() || out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("Page{out}")
    } else {
        out
    }
}


pub(crate) fn unsupported(diags: &mut Diagnostics, span: Span, what: impl AsRef<str>) {
    let mut d = guml_diagnostics::Diagnostic::warning(
        guml_diagnostics::Code::UnknownTag,
        format!("v0.1 React backend does not yet lower {}", what.as_ref()),
        span,
    );
    d.severity = Severity::Warning;
    d.help =
        Some("tracked in ROADMAP.md Phase 3; the emitted file marks the gap with a TODO".into());
    diags.push(d);
}




pub(crate) fn modifiers_of(el: &Element) -> Vec<&str> {
    el.positionals
        .iter()
        .filter_map(|p| match p {
            Positional::Modifier(m) => Some(m.as_str()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setter_naming() {
        assert_eq!(setter("count"), "setCount");
        assert_eq!(setter("draft"), "setDraft");
    }

    #[test]
    fn component_naming() {
        assert_eq!(component_name("Counter"), "Counter");
        assert_eq!(component_name("task list"), "TaskList");
        assert_eq!(component_name("landing-page"), "LandingPage");
    }


}
