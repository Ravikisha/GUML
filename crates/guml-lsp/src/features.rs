//! The editor features, as plain functions over text.
//!
//! Separated from the protocol layer on purpose: everything here is testable without a client,
//! a transport or an async runtime. `main.rs` is then only translation between LSP types and
//! these, which is the part that cannot go subtly wrong without failing to compile.
//!
//! Nothing here re-implements the language. Diagnostics come from `guml_compiler::check`,
//! highlighting from `guml_fmt::highlight`, formatting from `guml_fmt::format`, completion and
//! hover from the registry. That is the whole argument for building the server last: by now the
//! compiler already exposes everything an editor needs, so a wrong answer here would have to be
//! a translation bug rather than a second opinion about GUML.

use guml_diagnostics::{Diagnostic, Severity};
use guml_fmt::highlight::{Class, classify};
use guml_registry::{GLOBAL_ATTRS, MODIFIERS, Registry};

/// Zero-based line and character, as the protocol uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// A diagnostic, already converted to zero-based ranges.
#[derive(Debug, Clone)]
pub struct Reported {
    pub start: Position,
    pub end: Position,
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub help: Option<String>,
    /// Set when the compiler's suggestion is a safe replacement for the range, so the editor can
    /// offer it as a one-click fix.
    pub quick_fix: Option<String>,
}

pub fn diagnostics(src: &str) -> Vec<Reported> {
    let (_, diags) = guml_compiler::check(src);
    diags.items.iter().map(|d| convert(src, d)).collect()
}

fn convert(src: &str, d: &Diagnostic) -> Reported {
    Reported {
        start: offset_to_position(src, d.span.start),
        end: offset_to_position(src, d.span.end),
        severity: d.severity,
        code: d.id.clone(),
        message: d.message.clone(),
        help: d.help.clone(),
        // The same rule the batch applier uses: a bare word must not replace a span containing
        // whitespace, and a template with `…` is for a human.
        quick_fix: d.suggestion.clone().filter(|s| {
            !s.contains('…') && {
                let text = src.get(d.span.start..d.span.end).unwrap_or("");
                !text.contains(char::is_whitespace) || s.contains(char::is_whitespace)
            }
        }),
    }
}

/// Byte offset to a zero-based line/character pair.
///
/// Characters are counted in UTF-16 code units because that is what the protocol defaults to —
/// an em dash in a heading would otherwise shift every column after it, which is exactly the
/// class of bug that shredded the highlighter's spans earlier in this project.
pub fn offset_to_position(src: &str, offset: usize) -> Position {
    let offset = offset.min(src.len());
    let before = &src[..offset];
    let line = before.matches('\n').count() as u32;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let character = src[line_start..offset].encode_utf16().count() as u32;
    Position { line, character }
}

/// Semantic tokens, in the protocol's delta encoding.
///
/// Five integers per token: line delta, start delta, length, type index, modifier bitset. The
/// classifier already produces non-overlapping spans in source order, which is precisely the
/// precondition this encoding needs.
pub fn semantic_tokens(src: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for span in classify(src) {
        let start = offset_to_position(src, span.start);
        let end = offset_to_position(src, span.end);
        // A token that wraps a line has no meaning in this encoding; skip rather than emit
        // something the client will misplace.
        if end.line != start.line {
            continue;
        }
        let Some(kind) = token_type_index(span.class) else { continue };

        let line_delta = start.line - prev_line;
        let start_delta =
            if line_delta == 0 { start.character - prev_start } else { start.character };
        out.extend_from_slice(&[line_delta, start_delta, end.character - start.character, kind, 0]);
        prev_line = start.line;
        prev_start = start.character;
    }

    out
}

/// The legend, in the order `semantic_tokens` indexes into.
pub const TOKEN_TYPES: &[&str] = &[
    "type",
    "keyword",
    "modifier",
    "variable",
    "string",
    "number",
    "property",
    "function",
    "comment",
    "namespace",
    "operator",
];

fn token_type_index(class: Class) -> Option<u32> {
    let name = class.lsp_type();
    TOKEN_TYPES.iter().position(|t| *t == name).map(|i| i as u32)
}

/// What can be completed at a position.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub label: String,
    pub detail: String,
    pub kind: CompletionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Tag,
    Modifier,
    Attribute,
    State,
    Resource,
}

/// Completions for a cursor position.
///
/// Context-sensitive by the one rule that matters: at the start of a line you are naming a tag,
/// and after a tag you are choosing modifiers, attributes or a binding. Offering the whole
/// vocabulary everywhere would make the list useless exactly where it should help most.
pub fn completions(src: &str, at: Position) -> Vec<Completion> {
    let reg = Registry::builtin();
    let line = src.lines().nth(at.line as usize).unwrap_or("");
    let before = line
        .char_indices()
        .take_while(|(i, _)| (*i as u32) < at.character)
        .map(|(_, c)| c)
        .collect::<String>();
    let trimmed = before.trim_start();

    // Tag position: nothing but indentation to the left.
    if trimmed.is_empty() || !trimmed.contains(' ') {
        let mut out: Vec<Completion> = reg
            .names()
            .map(|name| Completion {
                label: name.to_string(),
                detail: reg.get(name).map(|c| c.doc.to_string()).unwrap_or_default(),
                kind: CompletionKind::Tag,
            })
            .collect();
        for directive in ["page", "state", "type", "data"] {
            out.push(Completion {
                label: directive.to_string(),
                detail: format!("`{directive}` directive"),
                kind: CompletionKind::Tag,
            });
        }
        return out;
    }

    // Inside a binding: state and resource names.
    if before.rfind('{').is_some_and(|open| !before[open..].contains('}')) {
        let (_, diags) = guml_compiler::check(src);
        let _ = diags;
        return declared_names(src);
    }

    // Otherwise: modifiers, then the attributes this tag accepts.
    let tag = trimmed.split_whitespace().next().unwrap_or("");
    let mut out: Vec<Completion> = MODIFIERS
        .iter()
        .map(|m| Completion {
            label: m.to_string(),
            detail: "modifier".to_string(),
            kind: CompletionKind::Modifier,
        })
        .collect();

    for attr in GLOBAL_ATTRS.iter() {
        out.push(Completion {
            label: format!("{attr}="),
            detail: "global attribute".to_string(),
            kind: CompletionKind::Attribute,
        });
    }
    if let Some(def) = reg.get(tag) {
        for attr in &def.attrs {
            out.push(Completion {
                label: format!("{attr}="),
                detail: format!("`{tag}` attribute"),
                kind: CompletionKind::Attribute,
            });
        }
    }
    out
}

/// State and resource names declared in the document, for binding completion.
fn declared_names(src: &str) -> Vec<Completion> {
    let (program, _) = guml_compiler::check(src);
    let mut out = Vec::new();
    for s in &program.states {
        out.push(Completion {
            label: s.name.clone(),
            detail: if s.domain.is_empty() {
                "state".to_string()
            } else {
                format!("state, one of {}", s.domain.join(" | "))
            },
            kind: CompletionKind::State,
        });
    }
    for r in &program.resources {
        out.push(Completion {
            label: r.name.clone(),
            detail: format!("resource of {}", r.ty),
            kind: CompletionKind::Resource,
        });
    }
    out
}

/// Hover text for a position: what the tag is, and what it costs.
///
/// The token cost is in here because it is the number the whole project is about, and an author
/// choosing between two tags should be able to see it without leaving the editor.
pub fn hover(src: &str, at: Position) -> Option<String> {
    let reg = Registry::builtin();
    let line = src.lines().nth(at.line as usize)?;
    let word = word_at(line, at.character as usize)?;

    if let Some(def) = reg.get(&word) {
        let kind = format!("{:?}", def.kind).to_lowercase();
        let attrs = if def.attrs.is_empty() {
            String::new()
        } else {
            format!("\n\nAttributes: `{}`", def.attrs.join("`, `"))
        };
        let label = if def.requires_label() { "\n\nRequires an accessible name." } else { "" };
        return Some(format!("**{word}** — {kind}\n\n{}{attrs}{label}", def.doc));
    }

    if MODIFIERS.contains(&word.as_str()) {
        return Some(format!(
            "**{word}** — modifier\n\nSemantic, not a utility class: the compiler owns what it \
             looks like, which is what keeps presentation out of the token budget."
        ));
    }

    let (program, _) = guml_compiler::check(src);
    if let Some(state) = program.state(&word) {
        return Some(match state.domain.is_empty() {
            true => format!("**{word}** — state"),
            false => format!("**{word}** — state, one of `{}`", state.domain.join("` | `")),
        });
    }
    if let Some(r) = program.resources.iter().find(|r| r.name == word) {
        let mutations: Vec<&str> = r.mutations.iter().map(|m| m.name.as_str()).collect();
        return Some(format!(
            "**{word}** — resource of `{}`\n\n`{} {}`{}",
            r.ty,
            r.method,
            r.url,
            if mutations.is_empty() {
                String::new()
            } else {
                format!("\n\nMutations: `{}`", mutations.join("`, `"))
            }
        ));
    }

    None
}

fn word_at(line: &str, character: usize) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    if character > chars.len() {
        return None;
    }
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    // Sitting just past the end of a word still counts as being on it, which is where a cursor
    // usually is when someone asks.
    let mut start = character.min(chars.len().saturating_sub(1));
    if !chars.get(start).copied().is_some_and(is_word) && start > 0 {
        start -= 1;
    }
    if !chars.get(start).copied().is_some_and(is_word) {
        return None;
    }
    let mut end = start;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    while end + 1 < chars.len() && is_word(chars[end + 1]) {
        end += 1;
    }
    Some(chars[start..=end].iter().collect())
}

/// Format the whole document. `None` when nothing would change, so the editor does not record an
/// undo step for a no-op.
pub fn format(src: &str) -> Option<String> {
    let out = guml_fmt::format(src, guml_fmt::Options::default());
    out.changed.then_some(out.text)
}

/// Document symbols: the declarations and top-level sections, for the outline view.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub detail: String,
    pub line: u32,
}

pub fn symbols(src: &str) -> Vec<Symbol> {
    let (program, _) = guml_compiler::check(src);
    let mut out = Vec::new();

    if let Some(page) = &program.page {
        out.push(Symbol {
            name: page.name.clone(),
            detail: "page".into(),
            line: page.span.line.saturating_sub(1),
        });
    }
    for t in &program.types {
        out.push(Symbol {
            name: t.name.clone(),
            detail: format!("type, {} fields", t.fields.len()),
            line: t.span.line.saturating_sub(1),
        });
    }
    for r in &program.resources {
        out.push(Symbol {
            name: r.name.clone(),
            detail: format!("data {}", r.ty),
            line: r.span.line.saturating_sub(1),
        });
    }
    for s in &program.states {
        out.push(Symbol {
            name: s.name.clone(),
            detail: "state".into(),
            line: s.span.line.saturating_sub(1),
        });
    }
    for el in &program.tree {
        if let Some(anchor) = el.anchor() {
            out.push(Symbol {
                name: format!("#{anchor}"),
                detail: el.tag.clone(),
                line: el.span.line.saturating_sub(1),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "page Tasks\n\
        type Task {id, title, done:bool}\n\
        data tasks:Task[] GET /api/tasks\n\
        \x20 save PATCH /api/tasks/{id} {done} optimistic\n\
        state filter=all|open|done\n\
        \n\
        list tasks where={filter}\n\
        \x20 text {title}\n";

    #[test]
    fn diagnostics_come_from_the_compiler_with_zero_based_ranges() {
        let reported = diagnostics("page P\n\ncrad Hello\n");
        let d = reported.iter().find(|d| d.code == "GUML0030").expect("unknown tag");
        // Line 3 in the compiler's 1-based world is line 2 here.
        assert_eq!(d.start.line, 2);
        assert_eq!(d.start.character, 0);
        assert_eq!(d.quick_fix.as_deref(), Some("card"), "a token rename is offerable");
    }

    #[test]
    fn a_template_suggestion_is_not_offered_as_a_quick_fix() {
        // `toggle aria="…"` is a shape for a human; applying it would put an ellipsis in the
        // accessible name.
        let reported = diagnostics("page P\nstate on=false\n\ntoggle {on}\n");
        let d = reported.iter().find(|d| d.code == "GUML0050").expect("missing name");
        assert!(d.quick_fix.is_none());
    }

    #[test]
    fn positions_count_utf16_units_like_the_protocol() {
        // An em dash is one char, three bytes, one UTF-16 unit. Counting bytes would put every
        // column after it in the wrong place.
        let src = "head Tasks — {tasks.open.count} open\n";
        let pos = offset_to_position(src, src.find('{').unwrap());
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, "head Tasks — ".chars().count() as u32);
    }

    #[test]
    fn semantic_tokens_are_five_ints_each_and_monotonic() {
        let data = semantic_tokens(DOC);
        assert_eq!(data.len() % 5, 0, "five integers per token");
        assert!(!data.is_empty());

        // Deltas must never be negative, which in this encoding means the source order the
        // classifier produced has to be preserved.
        let mut line = 0u32;
        for chunk in data.chunks(5) {
            line += chunk[0];
            assert!(chunk[2] > 0, "zero-length token");
            assert!((chunk[3] as usize) < TOKEN_TYPES.len(), "type index in legend");
        }
        assert!(line > 0, "tokens span more than one line");
    }

    #[test]
    fn completion_at_the_start_of_a_line_offers_tags_and_directives() {
        let items = completions(DOC, Position { line: 6, character: 0 });
        assert!(items.iter().any(|c| c.label == "list" && c.kind == CompletionKind::Tag));
        assert!(items.iter().any(|c| c.label == "state"));
        // Not modifiers: a line cannot start with one.
        assert!(!items.iter().any(|c| c.kind == CompletionKind::Modifier));
    }

    #[test]
    fn completion_after_a_tag_offers_modifiers_and_that_tags_attributes() {
        let items = completions("page P\n\nbtn Add ", Position { line: 2, character: 8 });
        assert!(items.iter().any(|c| c.label == "primary"));
        assert!(items.iter().any(|c| c.label == "busy="), "`busy` is a `btn` attribute");
        assert!(items.iter().any(|c| c.label == "aria="), "global attributes too");
    }

    #[test]
    fn completion_inside_a_binding_offers_declared_names() {
        let items = completions(
            "page P\nstate draft=\"\"\n\nbtn Go disabled={",
            Position { line: 3, character: 17 },
        );
        assert!(items.iter().any(|c| c.label == "draft" && c.kind == CompletionKind::State));
        assert!(!items.iter().any(|c| c.kind == CompletionKind::Modifier));
    }

    #[test]
    fn hover_explains_a_tag_a_modifier_and_a_declaration() {
        let tag = hover(DOC, Position { line: 6, character: 1 }).expect("hover on `list`");
        assert!(tag.contains("**list**"), "{tag}");
        assert!(tag.to_lowercase().contains("repeater"), "{tag}");

        let modifier = hover("page P\n\nbtn Add primary\n", Position { line: 2, character: 9 })
            .expect("hover on `primary`");
        assert!(modifier.contains("modifier"), "{modifier}");

        let state = hover(DOC, Position { line: 4, character: 7 }).expect("hover on `filter`");
        assert!(state.contains("all") && state.contains("done"), "{state}");
    }

    #[test]
    fn hover_on_a_resource_lists_its_mutations() {
        let text = hover(DOC, Position { line: 2, character: 6 }).expect("hover on `tasks`");
        assert!(text.contains("save"), "{text}");
    }

    #[test]
    fn hover_on_nothing_is_none() {
        assert!(hover(DOC, Position { line: 5, character: 0 }).is_none(), "blank line");
    }

    #[test]
    fn formatting_returns_none_when_nothing_changes() {
        let tidy = "page P\n\ncard Hi\n  p x\n";
        assert!(format(tidy).is_none(), "no edit means no undo step");
        assert!(format("page P\ncard Hi\n    p x\n").is_some());
    }

    #[test]
    fn symbols_list_the_declarations_and_anchored_sections() {
        let syms = symbols(DOC);
        assert!(syms.iter().any(|s| s.name == "Tasks" && s.detail == "page"));
        assert!(syms.iter().any(|s| s.name == "Task" && s.detail.starts_with("type")));
        assert!(syms.iter().any(|s| s.name == "tasks" && s.detail.starts_with("data")));
        assert!(syms.iter().any(|s| s.name == "filter" && s.detail == "state"));

        let anchored = symbols("page P\n\nsection #work Work\n  p x\n");
        assert!(anchored.iter().any(|s| s.name == "#work"));
    }

    #[test]
    fn nothing_panics_on_a_position_past_the_end_of_the_document() {
        // An editor will ask about a stale position while the user is typing.
        let far = Position { line: 999, character: 999 };
        assert!(hover(DOC, far).is_none());
        assert!(!completions(DOC, far).is_empty(), "falls back to tag position");
        let _ = offset_to_position(DOC, DOC.len() + 100);
    }
}
