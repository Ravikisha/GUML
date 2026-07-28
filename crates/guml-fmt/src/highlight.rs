//! Syntax classification: the compiler's own answer to "what colour is this byte".
//!
//! # Why this is in the compiler and not in the editor
//!
//! GUML cannot be highlighted by a regex grammar without lying. Whether the remainder of a
//! line is *structure* or *prose* depends on the tag, which is resolved against the
//! component registry — the same ambiguity that forced the lexer to be line-oriented. A
//! TextMate grammar cannot consult a registry, so it will always colour `p Press the
//! buttons` as a tag followed by four modifiers.
//!
//! So classification runs the real lexer and the real registry, and everything else — the
//! docs site, the playground, the LSP's semantic tokens, the generated TextMate grammar —
//! consumes the result. The site previously kept a hand-ported copy of the tag and modifier
//! lists, which had already drifted (it listed `h3`, which the registry does not define).
//!
//! Output is a flat, ordered, non-overlapping span list, which is what both a `<span>`
//! renderer and the LSP's delta encoding want.

use guml_registry::{Registry, TagKind};
use guml_syntax::{Tok, lex};
use std::fmt::Write as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// A registry tag in tag position.
    Tag,
    /// `page`, `state`, `data`, `type`, …
    Directive,
    /// A word from the closed modifier vocabulary.
    Modifier,
    /// `{…}` binding, type body or mutation body.
    Binding,
    Str,
    Num,
    /// The `name` of `name=value`.
    AttrKey,
    /// Everything from `>` to end of line.
    Action,
    /// Prose: the remainder of a text-tag line, or a `tier`/`faq` content line.
    Prose,
    Comment,
    Route,
    Anchor,
    /// `=` `:` `,` `|`
    Punct,
    /// A word in a position with no more specific meaning: a bare label.
    Text,
}

impl Class {
    /// Stable machine name. Used by the JSON output, the generated CSS classes and the LSP
    /// legend, so it is append-only in the same way diagnostic codes are.
    pub fn name(self) -> &'static str {
        match self {
            Class::Tag => "tag",
            Class::Directive => "directive",
            Class::Modifier => "modifier",
            Class::Binding => "binding",
            Class::Str => "string",
            Class::Num => "number",
            Class::AttrKey => "attr",
            Class::Action => "action",
            Class::Prose => "prose",
            Class::Comment => "comment",
            Class::Route => "route",
            Class::Anchor => "anchor",
            Class::Punct => "punct",
            Class::Text => "text",
        }
    }

    /// The LSP `SemanticTokenType` this maps onto. Editors theme those names, so mapping to
    /// them rather than inventing custom types is what makes GUML look native.
    pub fn lsp_type(self) -> &'static str {
        match self {
            Class::Tag => "type",
            Class::Directive => "keyword",
            Class::Modifier => "modifier",
            Class::Binding => "variable",
            Class::Str => "string",
            Class::Num => "number",
            Class::AttrKey => "property",
            Class::Action => "function",
            Class::Prose | Class::Text => "string",
            Class::Comment => "comment",
            Class::Route | Class::Anchor => "namespace",
            Class::Punct => "operator",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    /// Byte offsets into the source.
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub class: Class,
}

const DIRECTIVES: &[&str] =
    &["page", "type", "data", "state", "store", "route", "auth", "def", "js", "raw"];

pub fn classify(src: &str) -> Vec<Span> {
    let reg = Registry::builtin();
    let lexed = lex(src);
    let infos = crate::layout::analyse(&lexed.lines, &reg);
    let mut out: Vec<Span> = Vec::new();

    // Comments never reach the lexer, so they are found the same way the formatter finds
    // them: by re-reading the source.
    let mut offset = 0usize;
    for (i, raw) in src.split('\n').enumerate() {
        let line_no = (i + 1) as u32;
        let bare = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = bare.trim_start();
        if trimmed.starts_with("//") {
            let start = offset + (bare.len() - trimmed.len());
            out.push(Span {
                start,
                end: start + trimmed.trim_end().len(),
                line: line_no,
                class: Class::Comment,
            });
        }
        offset += raw.len() + 1;
    }

    for (idx, line) in lexed.lines.iter().enumerate() {
        // A `tier`/`faq` content line is prose end to end, including any `|`.
        if infos[idx].raw_text_child {
            push_prose(&mut out, src, line.text_start, line.text_start + line.text.len(), line.line_no);
            continue;
        }

        let tag = line.tokens.first().and_then(|t| t.tok.as_word()).map(str::to_string);
        let is_directive = tag.as_deref().is_some_and(|t| DIRECTIVES.contains(&t));
        let prose_tail = tag.as_deref().is_some_and(|t| reg.kind(t) == Some(TagKind::Text));

        for (i, token) in line.tokens.iter().enumerate() {
            let class = if i == 0 {
                match () {
                    _ if is_directive => Class::Directive,
                    _ if tag.as_deref().is_some_and(|t| reg.get(t).is_some()) => Class::Tag,
                    // An unknown first word is still in tag position: colouring it as a tag
                    // is what makes the typo look like a typo rather than like prose.
                    _ => Class::Tag,
                }
            } else if prose_tail {
                // The whole remainder of a text-tag line is one prose run, so emit it once
                // and stop tokenising for colour purposes.
                push_prose(
                    &mut out,
                    src,
                    token.span.start,
                    line.text_start + line.text.len(),
                    line.line_no,
                );
                break;
            } else if matches!(token.tok, Tok::Pipe) && !is_directive {
                // `card "Ship it" | Describe the page` — everything past the bar is content,
                // exactly as the formatter treats it. Tokenising it would colour ordinary
                // words as modifiers, which is the same lie a regex grammar tells.
                out.push(Span {
                    start: token.span.start,
                    end: token.span.end,
                    line: line.line_no,
                    class: Class::Punct,
                });
                push_prose(
                    &mut out,
                    src,
                    token.span.end,
                    line.text_start + line.text.len(),
                    line.line_no,
                );
                break;
            } else {
                class_of(&token.tok, line.tokens.get(i + 1).map(|t| &t.tok), &reg)
            };
            out.push(Span {
                start: token.span.start,
                end: token.span.end,
                line: line.line_no,
                class,
            });
        }
    }

    out.sort_by_key(|s| s.start);
    out
}

/// Prose is one run *except* for `{…}`, which the compiler interpolates — `head Tasks —
/// {tasks.open.count} open` renders a live number, so colouring it as flat prose would hide
/// the one part of the line that is code.
fn push_prose(out: &mut Vec<Span>, src: &str, start: usize, end: usize, line: u32) {
    // Start at the first non-space byte so a prose run after `|` looks like every other
    // prose run; the gap between tokens is never part of a span.
    let lead = src[start..end].len() - src[start..end].trim_start().len();
    let start = start + lead;
    if start >= end {
        return;
    }
    let text = &src[start..end];
    let mut at = 0usize;
    while let Some(open) = text[at..].find('{') {
        let open = at + open;
        let Some(close) = text[open..].find('}') else { break };
        let close = open + close + 1;
        if open > at {
            out.push(Span { start: start + at, end: start + open, line, class: Class::Prose });
        }
        out.push(Span { start: start + open, end: start + close, line, class: Class::Binding });
        at = close;
    }
    if at < text.len() {
        out.push(Span { start: start + at, end, line, class: Class::Prose });
    }
}

fn class_of(tok: &Tok, next: Option<&Tok>, reg: &Registry) -> Class {
    match tok {
        // `name=` — the word before an `=` is an attribute key, not a label.
        Tok::Word(w) => {
            if matches!(next, Some(Tok::Eq)) {
                Class::AttrKey
            } else if reg.is_modifier(w) {
                Class::Modifier
            } else {
                Class::Text
            }
        }
        Tok::Num(_) => Class::Num,
        Tok::Str(_) => Class::Str,
        Tok::Brace(_) => Class::Binding,
        Tok::Action(_) => Class::Action,
        Tok::Anchor(_) => Class::Anchor,
        Tok::Route(_) => Class::Route,
        Tok::Pipe | Tok::Eq | Tok::Colon | Tok::Comma => Class::Punct,
    }
}

/// Line-grouped rows of `(text, class)`, ready for a `<span>` renderer. Gaps between spans
/// (the spaces) come back as `None`, so a consumer can emit them unstyled without having to
/// re-derive the offsets.
pub fn rows(src: &str) -> Vec<Vec<(String, Option<Class>)>> {
    let spans = classify(src);
    let mut out: Vec<Vec<(String, Option<Class>)>> = Vec::new();
    let mut cursor = 0usize;
    let mut si = 0usize;

    for raw in src.split('\n') {
        let line_start = cursor;
        let line_end = cursor + raw.len();
        cursor = line_end + 1;

        let mut row: Vec<(String, Option<Class>)> = Vec::new();
        let mut at = line_start;
        while si < spans.len() && spans[si].start < line_end {
            let s = &spans[si];
            if s.start > at {
                row.push((src[at..s.start].to_string(), None));
            }
            let end = s.end.min(line_end);
            if end > s.start {
                row.push((src[s.start..end].to_string(), Some(s.class)));
            }
            at = end;
            si += 1;
        }
        if at < line_end {
            row.push((src[at..line_end].to_string(), None));
        }
        out.push(row);
    }

    // `split('\n')` yields a phantom empty line for a trailing newline.
    if out.last().is_some_and(Vec::is_empty) {
        out.pop();
    }
    out
}

/// JSON for the CLI and the wasm boundary. Hand-written rather than via serde so the crate
/// stays dependency-light — this is a flat array of five-field objects.
pub fn to_json(src: &str) -> String {
    let mut out = String::from("[");
    for (i, s) in classify(src).iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"start":{},"end":{},"line":{},"class":"{}","lsp":"{}"}}"#,
            s.start,
            s.end,
            s.line,
            s.class.name(),
            s.class.lsp_type()
        );
    }
    out.push(']');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(src: &str) -> Vec<(&'static str, String)> {
        classify(src).into_iter().map(|s| (s.class.name(), src[s.start..s.end].to_string())).collect()
    }

    #[test]
    fn a_tag_is_a_tag_and_a_modifier_is_a_modifier() {
        assert_eq!(
            classes("btn Add primary\n"),
            vec![
                ("tag", "btn".into()),
                ("text", "Add".into()),
                ("modifier", "primary".into())
            ]
        );
    }

    #[test]
    fn prose_after_a_text_tag_is_one_run_not_a_pile_of_modifiers() {
        // The reason a regex grammar cannot do this: `center` is a real modifier, and here
        // it is just a word in a sentence.
        assert_eq!(
            classes("p Press the center button\n"),
            vec![("tag", "p".into()), ("prose", "Press the center button".into())]
        );
    }

    #[test]
    fn attribute_keys_are_distinguished_from_labels() {
        let c = classes("input draft placeholder=\"Add…\"\n");
        assert_eq!(c[1], ("text", "draft".into()));
        assert_eq!(c[2], ("attr", "placeholder".into()));
        assert_eq!(c[3], ("punct", "=".into()));
        assert_eq!(c[4], ("string", "\"Add…\"".into()));
    }

    #[test]
    fn directives_bindings_actions_routes_and_anchors() {
        let c = classes("state count=0\n");
        assert_eq!(c[0], ("directive", "state".into()));
        assert_eq!(classes("btn Go >count++\n")[2], ("action", ">count++".into()));
        assert_eq!(classes("metric {count}\n")[1], ("binding", "{count}".into()));
        assert_eq!(classes("link Pricing #pricing\n")[2], ("anchor", "#pricing".into()));
        assert_eq!(classes("btn Start /signup\n")[2], ("route", "/signup".into()));
    }

    #[test]
    fn a_binding_inside_prose_is_still_a_binding() {
        // `head Tasks — {tasks.open.count} open` renders a live count. Flattening it into
        // prose would hide the only executable part of the line.
        assert_eq!(
            classes("head Tasks — {tasks.open.count} open
"),
            vec![
                ("tag", "head".into()),
                ("prose", "Tasks — ".into()),
                ("binding", "{tasks.open.count}".into()),
                ("prose", " open".into()),
            ]
        );
        assert_eq!(classes("metric {count}
")[1], ("binding", "{count}".into()));
    }

    #[test]
    fn content_after_a_bar_is_prose_not_structure() {
        let c = classes("card \"Ship it\" | Describe the page, get a full build.
");
        assert_eq!(c[0], ("tag", "card".into()));
        assert_eq!(c[1], ("string", "\"Ship it\"".into()));
        assert_eq!(c[2], ("punct", "|".into()));
        // One run: `full` is a real modifier and must not be coloured as one here.
        assert_eq!(c[3], ("prose", "Describe the page, get a full build.".into()));
    }

    #[test]
    fn comments_are_classified_even_though_the_lexer_drops_them() {
        assert_eq!(classes("// a note\npage X\n")[0], ("comment", "// a note".into()));
    }

    #[test]
    fn content_lines_under_tier_are_prose_including_the_bar() {
        let c = classes("faq open=1\n  Can I export? | Yes.\n");
        assert!(c.contains(&("prose", "Can I export? | Yes.".to_string())), "{c:?}");
    }

    #[test]
    fn rows_cover_every_byte_of_the_line_including_the_gaps() {
        let src = "btn Add primary\n";
        let joined: String =
            rows(src).into_iter().flatten().map(|(t, _)| t).collect::<Vec<_>>().join("");
        assert_eq!(joined, "btn Add primary");
    }

    #[test]
    fn rows_are_one_per_source_line() {
        assert_eq!(rows("page X\n\ncard A\n").len(), 3);
    }

    #[test]
    fn json_is_well_formed_and_carries_the_lsp_mapping() {
        let json = to_json("btn Add primary\n");
        assert!(json.starts_with(r#"[{"start":0,"end":3,"line":1,"class":"tag","lsp":"type"}"#), "{json}");
    }

    #[test]
    fn an_unknown_tag_is_still_coloured_as_a_tag() {
        // Colouring a typo as prose would hide it; the diagnostic says what is wrong, the
        // highlighter's job is only to keep it looking like the thing it was meant to be.
        assert_eq!(classes("crad Hi\n")[0], ("tag", "crad".into()));
    }
}
