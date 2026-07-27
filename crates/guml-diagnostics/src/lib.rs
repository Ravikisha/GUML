//! Diagnostics for the GUML compiler.
//!
//! Design constraint from the research report (§6.3, §12.2): the *producer* of GUML source
//! is usually an LLM, so diagnostics are a machine interface first and a human interface
//! second. That means:
//!
//! 1. **Complete in one pass** — never stop at the first error. Every extra round trip is a
//!    full LLM generation, so a parser that reports one error at a time turns a 1-iteration
//!    repair loop into an N-iteration one.
//! 2. **Span-accurate** — byte offsets plus line/col, so a patch can be applied mechanically.
//! 3. **Machine-actionable** — a stable `code`, and where possible a concrete `suggestion`
//!    string that is a drop-in replacement for the offending span.
//! 4. **Serializable** — `--format json` is the format the repair loop consumes.

use serde::{Deserialize, Serialize};

/// A byte range in the source, with precomputed line/column for display.
///
/// `line` and `col` are 1-based (editor convention). `start`/`end` are 0-based byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, col: u32) -> Self {
        Self { start, end, line, col }
    }

    /// A zero-width span, used for "expected something here" diagnostics.
    pub fn point(at: usize, line: u32, col: u32) -> Self {
        Self { start: at, end: at, line, col }
    }

    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line,
            col: self.col,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
}

/// Stable diagnostic codes. These are part of the compiler's public contract: the repair
/// loop and the eventual benchmark harness key on them, so codes are append-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    // Lexical
    TabIndent,
    UnterminatedString,
    UnterminatedBrace,
    UnexpectedChar,
    // Layout
    InconsistentDedent,
    UnexpectedIndent,
    // Syntax
    ExpectedTag,
    ExpectedValue,
    TrailingTokensAfterAction,
    // Resolution
    UnknownTag,
    UnknownModifier,
    UnknownAttr,
    UnknownState,
    // Semantics
    DuplicateState,
    MissingPageDirective,
    // Accessibility (hard errors by design — see report §6.4)
    IconControlWithoutLabel,
    InputWithoutLabel,
}

impl Code {
    /// Short stable string, e.g. `GUML0007`, for logs and test snapshots.
    pub fn id(self) -> &'static str {
        match self {
            Code::TabIndent => "GUML0001",
            Code::UnterminatedString => "GUML0002",
            Code::UnterminatedBrace => "GUML0003",
            Code::UnexpectedChar => "GUML0004",
            Code::InconsistentDedent => "GUML0010",
            Code::UnexpectedIndent => "GUML0011",
            Code::ExpectedTag => "GUML0020",
            Code::ExpectedValue => "GUML0021",
            Code::TrailingTokensAfterAction => "GUML0022",
            Code::UnknownTag => "GUML0030",
            Code::UnknownModifier => "GUML0031",
            Code::UnknownAttr => "GUML0032",
            Code::UnknownState => "GUML0033",
            Code::DuplicateState => "GUML0040",
            Code::MissingPageDirective => "GUML0041",
            Code::IconControlWithoutLabel => "GUML0050",
            Code::InputWithoutLabel => "GUML0051",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: Code,
    pub id: String,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    /// Human/model-facing explanation of *how* to fix it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Literal replacement text for `span`. Present only when the fix is unambiguous, so
    /// the repair loop can apply it without another model call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn error(code: Code, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            id: code.id().to_string(),
            severity: Severity::Error,
            message: message.into(),
            span,
            help: None,
            suggestion: None,
        }
    }

    pub fn warning(code: Code, message: impl Into<String>, span: Span) -> Self {
        Self { severity: Severity::Warning, ..Self::error(code, message, span) }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, d: Diagnostic) {
        self.items.push(d);
    }

    pub fn extend(&mut self, other: Diagnostics) {
        self.items.extend(other.items);
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.items.iter().filter(|d| d.severity == Severity::Error).count()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.items).unwrap_or_else(|_| "[]".into())
    }

    /// Terminal rendering with a source excerpt and caret. Deliberately dependency-free for
    /// now; swap in `miette`/`ariadne` once the diagnostic set stabilises (see TECH-STACK.md).
    pub fn render(&self, src: &str, path: &str) -> String {
        let lines: Vec<&str> = src.lines().collect();
        let mut out = String::new();
        for d in &self.items {
            let sev = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Note => "note",
            };
            out.push_str(&format!("{sev}[{}]: {}\n", d.id, d.message));
            out.push_str(&format!("  --> {path}:{}:{}\n", d.span.line, d.span.col));
            if let Some(text) = lines.get(d.span.line.saturating_sub(1) as usize) {
                out.push_str(&format!("   |\n {:>2} | {text}\n", d.span.line));
                let pad = " ".repeat(d.span.col.saturating_sub(1) as usize);
                let width = (d.span.end.saturating_sub(d.span.start)).max(1);
                out.push_str(&format!("   | {pad}{}\n", "^".repeat(width)));
            }
            if let Some(h) = &d.help {
                out.push_str(&format!("   = help: {h}\n"));
            }
            if let Some(s) = &d.suggestion {
                out.push_str(&format!("   = suggestion: {s}\n"));
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique() {
        let codes = [
            Code::TabIndent,
            Code::UnterminatedString,
            Code::UnterminatedBrace,
            Code::UnexpectedChar,
            Code::InconsistentDedent,
            Code::UnexpectedIndent,
            Code::ExpectedTag,
            Code::ExpectedValue,
            Code::TrailingTokensAfterAction,
            Code::UnknownTag,
            Code::UnknownModifier,
            Code::UnknownAttr,
            Code::UnknownState,
            Code::DuplicateState,
            Code::MissingPageDirective,
            Code::IconControlWithoutLabel,
            Code::InputWithoutLabel,
        ];
        let mut ids: Vec<&str> = codes.iter().map(|c| c.id()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "diagnostic codes must be unique");
    }

    #[test]
    fn json_round_trips() {
        let mut ds = Diagnostics::new();
        ds.push(
            Diagnostic::error(Code::UnknownTag, "unknown tag `buton`", Span::new(0, 5, 1, 1))
                .with_suggestion("btn"),
        );
        let json = ds.to_json();
        assert!(json.contains("GUML0030"));
        assert!(json.contains("\"suggestion\": \"btn\""));
        assert!(ds.has_errors());
    }
}
