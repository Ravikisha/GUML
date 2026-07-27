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

use guml_ast::{Element, Positional, Program, Value};
use guml_diagnostics::{Diagnostics, Severity, Span};

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
        _ => None,
    }
}

pub fn backend_names() -> &'static [&'static str] {
    &["react"]
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

/// Lower a GUML action body to a JS statement list.
///
/// Supported in v0.1: `x++`, `x--`, `x=expr`, and `;`-sequencing. Resource mutations
/// (`tasks.add{…}`) are recognised and reported as unsupported rather than mangled.
pub(crate) fn lower_action(
    action: &str,
    states: &[String],
    diags: &mut Diagnostics,
    span: Span,
) -> String {
    let mut stmts = Vec::new();
    for raw in action.split(';') {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }
        if let Some(name) = s.strip_suffix("++") {
            let name = name.trim();
            stmts.push(format!("{}({} + 1)", setter(name), name));
        } else if let Some(name) = s.strip_suffix("--") {
            let name = name.trim();
            stmts.push(format!("{}({} - 1)", setter(name), name));
        } else if let Some((lhs, rhs)) = s.split_once('=') {
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if lhs.contains('.') || lhs.contains('(') {
                unsupported(diags, span, format!("action target `{lhs}`"));
                continue;
            }
            stmts.push(format!("{}({})", setter(lhs), rhs));
        } else if s.contains('.') {
            // e.g. `tasks.add{title:draft}` — needs the resource layer (Phase 3).
            unsupported(diags, span, format!("resource mutation `{s}`"));
        } else {
            unsupported(diags, span, format!("action `{s}`"));
        }
    }
    let _ = states; // reserved: unknown-state checking moves here with the resolver
    stmts.join("; ")
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

/// JSX attribute rendering for a GUML attribute value.
pub(crate) fn jsx_attr(name: &str, v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{name}={:?}", s),
        Value::Word(w) => format!("{name}={:?}", w),
        Value::Num(_) | Value::Bool(_) => format!("{name}={{{}}}", v.to_js()),
        Value::Binding(b) => format!("{name}={{{b}}}"),
        Value::Flag => name.to_string(),
    }
}

/// Content may embed `{expr}` bindings. JSX treats those as expressions natively, so v0.1
/// passes them through; a real expression lowering pass lands with the resolver.
pub(crate) fn jsx_text(content: &str) -> String {
    content.to_string()
}

pub(crate) fn state_names(p: &Program) -> Vec<String> {
    p.states.iter().map(|s| s.name.clone()).collect()
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
    use guml_diagnostics::Diagnostics;

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

    #[test]
    fn action_lowering() {
        let mut d = Diagnostics::new();
        let span = Span::point(0, 1, 1);
        let states = vec!["count".to_string()];
        assert_eq!(lower_action("count++", &states, &mut d, span), "setCount(count + 1)");
        assert_eq!(lower_action("count--", &states, &mut d, span), "setCount(count - 1)");
        assert_eq!(lower_action("count=0", &states, &mut d, span), "setCount(0)");
        assert!(!d.has_errors());
    }

    #[test]
    fn unsupported_actions_warn_instead_of_mangling() {
        let mut d = Diagnostics::new();
        let out = lower_action("tasks.add{title:draft}", &[], &mut d, Span::point(0, 1, 1));
        assert!(out.is_empty());
        assert_eq!(d.len(), 1);
        assert!(!d.has_errors(), "unsupported features are warnings, not errors");
    }
}
