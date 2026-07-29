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
pub mod html;
pub mod json;
pub mod react;
pub mod sourcemap;
pub mod theme;

#[derive(Debug, Clone)]
pub struct OutFile {
    pub path: String,
    pub contents: String,
    /// Line provenance, when the backend recorded it. The driver serialises this to Source Map
    /// v3, because serialising needs the source *text* and only the driver has it.
    ///
    /// Without a map, every stack trace and breakpoint points at generated code the author never
    /// wrote — which the report names as an adoption blocker rather than a nicety (§12.2).
    pub source_map: Option<crate::sourcemap::SourceMap>,
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
        "html" => Some(Box::new(html::HtmlBackend { style: html::Style::Inline })),
        // Same backend, styled by the Tailwind CDN. A separate name rather than a flag, so the
        // preview path is visible in `--help` and never becomes the default by accident.
        "html-cdn" => Some(Box::new(html::HtmlBackend { style: html::Style::Cdn })),
        _ => None,
    }
}

pub fn backend_names() -> &'static [&'static str] {
    &["react", "json", "html", "html-cdn"]
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

/// Invariant 3: a construct a backend cannot lower is *reported*, never silently dropped.
///
/// The backend names itself, because "cannot lower" now means different things in different
/// backends. In React it is a gap to be filled later; in `html` it is permanent and architectural —
/// there is no JavaScript, so there will never be an `onClick`. A message that said "does not yet"
/// about the second case would be telling the reader to wait for something that is not coming.
pub(crate) fn unsupported_in(
    diags: &mut Diagnostics,
    backend: &str,
    span: Span,
    what: impl AsRef<str>,
) {
    let mut d = guml_diagnostics::Diagnostic::warning(
        guml_diagnostics::Code::UnknownTag,
        format!("`{backend}` backend: {}", what.as_ref()),
        span,
    );
    d.severity = Severity::Warning;
    d.help = Some(match backend {
        "html" => "the emitted markup marks the gap with `data-guml-inert`; a construct that needs a runtime cannot work in a no-JavaScript backend".into(),
        _ => "tracked in ROADMAP.md Phase 3; the emitted file marks the gap with a TODO".to_string(),
    });
    diags.push(d);
}

/// The React backend's shorthand, kept so its call sites read unchanged.
pub(crate) fn unsupported(diags: &mut Diagnostics, span: Span, what: impl AsRef<str>) {
    unsupported_in(diags, "react", span, format!("does not yet lower {}", what.as_ref()));
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
