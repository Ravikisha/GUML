//! GUML lexer.
//!
//! # Why line-oriented rather than a token stream with INDENT/DEDENT
//!
//! GUML has a genuine ambiguity that no lexer can resolve alone:
//!
//! ```text
//! btn Decrement ghost >count--     // `Decrement` is a label, `ghost` is a modifier
//! p Press the buttons to change.   // the whole remainder is prose content
//! ```
//!
//! Whether the remainder of a line is *structured* or *prose* depends on the tag, which is
//! resolved against the component registry — i.e. it is a parser/resolver concern. So the
//! lexer produces, per logical line, both a structured token list **and** the raw text, and
//! lets the parser choose. `Line::rest_from` is the bridge.
//!
//! This is also why prose costs almost nothing in GUML (report §1.5, the "content floor"):
//! text is never escaped or quoted, because the lexer never has to interpret it.
//!
//! Layout (nesting) is derived from the `indent` field by the parser's indent stack, not by
//! synthesised INDENT/DEDENT tokens. Blank lines and `//` comments never affect layout.

pub mod expr;

pub use guml_diagnostics::{Code as DiagCode, Diagnostic as Diag};
use guml_diagnostics::{Code, Diagnostic, Diagnostics, Span};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Tok {
    /// Bare word: tag names, modifiers, labels, numbers-with-units (`$24/mo`).
    Word(String),
    /// Numeric literal.
    Num(String),
    /// Quoted string, escapes resolved.
    Str(String),
    /// Balanced `{...}` group, braces stripped. The parser interprets it as a binding
    /// expression (`{count}`), a type body (`{id, title:bool}`), or a mutation body.
    Brace(String),
    /// Everything after `>` to end of line. Actions terminate a line by construction, which
    /// is what makes them lexable in one pass.
    Action(String),
    /// `#features`
    Anchor(String),
    /// `/signup`
    Route(String),
    Pipe,
    Eq,
    Colon,
    Comma,
}

impl Tok {
    pub fn as_word(&self) -> Option<&str> {
        match self {
            Tok::Word(w) => Some(w),
            _ => None,
        }
    }

    /// Text value of a token when used in a positional/content slot.
    pub fn text(&self) -> Option<&str> {
        match self {
            Tok::Word(w) | Tok::Num(w) | Tok::Str(w) => Some(w),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

/// One significant source line: blank lines and comments are dropped by the lexer.
#[derive(Debug, Clone, Serialize)]
pub struct Line {
    /// Leading-space count. Tabs are an error (`GUML0001`).
    pub indent: usize,
    pub line_no: u32,
    /// Byte offset of the start of the line in the source.
    pub start: usize,
    /// The line, without the trailing newline and without the leading indent.
    pub text: String,
    /// Byte offset where `text` begins (i.e. `start + indent`).
    pub text_start: usize,
    pub tokens: Vec<Token>,
}

impl Line {
    /// Raw source from token `idx` to end of line. This is how prose content and free-form
    /// values are recovered without the lexer having to interpret them.
    pub fn rest_from(&self, idx: usize) -> &str {
        match self.tokens.get(idx) {
            Some(t) => {
                let rel = t.span.start.saturating_sub(self.text_start);
                self.text.get(rel..).unwrap_or("").trim_end()
            }
            None => "",
        }
    }

    pub fn span(&self) -> Span {
        Span::new(
            self.text_start,
            self.text_start + self.text.len(),
            self.line_no,
            (self.indent + 1) as u32,
        )
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Lexed {
    pub lines: Vec<Line>,
    #[serde(skip)]
    pub diagnostics: Diagnostics,
}

/// Lex a GUML source file. Never fails: lexical problems are reported as diagnostics and the
/// lexer recovers so that later errors are still surfaced in the same pass.
pub fn lex(src: &str) -> Lexed {
    let mut out = Lexed::default();
    let mut offset = 0usize;
    let mut line_no = 0u32;
    // Indent of the `js`/`raw` header whose body we are inside. Nothing below it is GUML: not the
    // tokens, not `//` (a JavaScript comment), not a tab (legal indentation in most other
    // languages). Lexing a body as GUML reported errors against code the compiler had just
    // promised not to look at, and silently deleted every `//` line in it.
    let mut escape_indent: Option<usize> = None;

    for raw in src.split('\n') {
        line_no += 1;
        let line_start = offset;
        offset += raw.len() + 1;

        let raw = raw.strip_suffix('\r').unwrap_or(raw);

        // Indent, and whether a tab was used for it. Measured before the escape test, because the
        // test needs the indent — but the tab is only *reported* once we know the line is GUML, so
        // the positions are recovered by re-walking the prefix below rather than collected here:
        // this loop runs for every line of every compile, and a `Vec` per line cost ~4x on the
        // 200-line `check` budget.
        let mut indent = 0usize;
        let mut byte_idx = 0usize;
        let mut has_tab = false;
        for ch in raw.chars() {
            match ch {
                ' ' => {
                    indent += 1;
                    byte_idx += 1;
                }
                '\t' => {
                    has_tab = true;
                    indent += 2; // recover: keep parsing this line at a plausible level
                    byte_idx += 1;
                }
                _ => break,
            }
        }

        let text = &raw[byte_idx..];
        let trimmed_end = text.trim_end();

        // A blank line neither opens nor closes a block, so it must not clear the escape state.
        // It is still dropped, as everywhere else — the one content this does not preserve, which
        // matters only inside a multi-line string literal.
        if trimmed_end.is_empty() {
            continue;
        }

        let in_escape = escape_indent.is_some_and(|base| indent > base);
        if !in_escape {
            escape_indent = None;
            if trimmed_end.starts_with("//") {
                continue; // comments never affect layout
            }
            for t in tab_positions(&raw[..byte_idx], has_tab) {
                out.diagnostics.push(
                    Diagnostic::error(
                        Code::TabIndent,
                        "tabs are not allowed for indentation",
                        Span::new(line_start + t, line_start + t + 1, line_no, (t + 1) as u32),
                    )
                    .with_help("GUML indentation is spaces only, 2 per level")
                    .with_suggestion("  "),
                );
            }
        }

        let text_start = line_start + byte_idx;
        // A body line carries no tokens at all: the parser takes its text verbatim, and giving it
        // tokens would let a later pass mistake it for structure.
        let tokens = if in_escape {
            Vec::new()
        } else {
            lex_line(trimmed_end, text_start, indent, line_no, &mut out.diagnostics)
        };
        if !in_escape && matches!(tokens.first().and_then(|t| t.tok.as_word()), Some("js" | "raw"))
        {
            escape_indent = Some(indent);
        }

        out.lines.push(Line {
            indent,
            line_no,
            start: line_start,
            text: trimmed_end.to_string(),
            text_start,
            tokens,
        });
    }

    out
}

/// Byte offsets of the tabs in an indent prefix. `has_tab` short-circuits the common case so the
/// prefix is not re-walked for the overwhelming majority of lines that are indented with spaces.
fn tab_positions(prefix: &str, has_tab: bool) -> impl Iterator<Item = usize> + '_ {
    let prefix = if has_tab { prefix } else { "" };
    prefix.bytes().enumerate().filter(|(_, b)| *b == b'\t').map(|(i, _)| i)
}

fn lex_line(
    text: &str,
    base: usize,
    col_base: usize,
    line_no: u32,
    diags: &mut Diagnostics,
) -> Vec<Token> {
    let bytes = text.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0usize;

    // Columns are absolute within the source line (1-based), so an editor or a patch tool can
    // use them directly.
    let span_at = |start: usize, end: usize| -> Span {
        Span::new(base + start, base + end, line_no, (start + col_base + 1) as u32)
    };

    while i < bytes.len() {
        let c = bytes[i];

        if c == b' ' {
            i += 1;
            continue;
        }

        // `>` swallows the rest of the line as an action body.
        if c == b'>' {
            let body = text[i + 1..].trim().to_string();
            toks.push(Token { tok: Tok::Action(body), span: span_at(i, text.len()) });
            break;
        }

        // Quoted string.
        if c == b'"' {
            let start = i;
            i += 1;
            let mut value = String::new();
            let mut closed = false;
            while i < bytes.len() {
                match bytes[i] {
                    b'\\' if i + 1 < bytes.len() => {
                        // Keep escape handling minimal and predictable: \" and \\ only.
                        let next = bytes[i + 1];
                        if next == b'"' || next == b'\\' {
                            value.push(next as char);
                            i += 2;
                        } else {
                            value.push('\\');
                            i += 1;
                        }
                    }
                    b'"' => {
                        i += 1;
                        closed = true;
                        break;
                    }
                    _ => {
                        // Copy the whole UTF-8 char, not the byte.
                        let ch = text[i..].chars().next().unwrap();
                        value.push(ch);
                        i += ch.len_utf8();
                    }
                }
            }
            if !closed {
                diags.push(
                    Diagnostic::error(
                        Code::UnterminatedString,
                        "unterminated string literal",
                        span_at(start, i),
                    )
                    .with_help("add a closing `\"`"),
                );
            }
            toks.push(Token { tok: Tok::Str(value), span: span_at(start, i) });
            continue;
        }

        // Balanced brace group: binding, type body, or mutation body.
        if c == b'{' {
            let start = i;
            let mut depth = 0i32;
            let mut in_str = false;
            while i < bytes.len() {
                match bytes[i] {
                    b'"' => in_str = !in_str,
                    b'{' if !in_str => depth += 1,
                    b'}' if !in_str => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if depth != 0 {
                diags.push(
                    Diagnostic::error(
                        Code::UnterminatedBrace,
                        "unterminated `{` group",
                        span_at(start, i),
                    )
                    .with_help("add a closing `}`"),
                );
            }
            let inner_end = i.saturating_sub(1).max(start + 1);
            let inner = text[start + 1..inner_end.min(text.len())].to_string();
            toks.push(Token { tok: Tok::Brace(inner.trim().to_string()), span: span_at(start, i) });
            continue;
        }

        // Anchor `#id`, only at token start.
        if c == b'#' {
            let start = i;
            i += 1;
            while i < bytes.len() && is_word_byte(bytes[i]) {
                i += 1;
            }
            toks.push(Token {
                tok: Tok::Anchor(text[start + 1..i].to_string()),
                span: span_at(start, i),
            });
            continue;
        }

        // Route `/path`, only at token start (so `$24/mo` stays a single word) — or an absolute
        // `http(s)://host/path`.
        //
        // # Why the absolute case is here and not left to fall through
        //
        // It did fall through, and lost the scheme. `https://api.example.com/rows` lexed as the word
        // `https`, a `:` (the type-annotation separator), and then a route `//api.example.com/rows` —
        // so the emitted code fetched a *protocol-relative* URL. It happened to work in a browser,
        // which is why nobody noticed, and it was wrong in three ways: the scheme the author wrote was
        // silently discarded, the emitted request did not match the document, and `validate::check_url`'s
        // `starts_with("http")` branch was unreachable dead code claiming support for a form that never
        // arrived intact.
        //
        // Only `http` and `https`. A general "scheme followed by `://`" rule would make `javascript:` and
        // `data:` lexable as request targets, and a URL a document can name is a URL the compiler will
        // emit a fetch to — so the allow-list is the security boundary, not a convenience.
        let absolute = (c == b'h')
            && ["https://", "http://"].iter().any(|scheme| text[i..].starts_with(scheme));
        if c == b'/' || absolute {
            let start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            toks.push(Token {
                tok: Tok::Route(text[start..i].to_string()),
                span: span_at(start, i),
            });
            continue;
        }

        // Single-character punctuation.
        let punct = match c {
            b'|' => Some(Tok::Pipe),
            b'=' => Some(Tok::Eq),
            b':' => Some(Tok::Colon),
            b',' => Some(Tok::Comma),
            _ => None,
        };
        if let Some(p) = punct {
            toks.push(Token { tok: p, span: span_at(i, i + 1) });
            i += 1;
            continue;
        }

        // Number.
        if c.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            // A digit run followed immediately by word bytes is a word (`3px`, `2xl`).
            if i < bytes.len() && is_word_byte(bytes[i]) {
                while i < bytes.len() && is_word_byte(bytes[i]) {
                    i += 1;
                }
                toks.push(Token {
                    tok: Tok::Word(text[start..i].to_string()),
                    span: span_at(start, i),
                });
            } else {
                toks.push(Token {
                    tok: Tok::Num(text[start..i].to_string()),
                    span: span_at(start, i),
                });
            }
            continue;
        }

        // Bare word.
        let start = i;
        while i < bytes.len() && is_word_byte(bytes[i]) {
            i += 1;
        }
        if i == start {
            // Unrecognised byte: report once and skip the whole char so we make progress.
            let ch = text[i..].chars().next().unwrap();
            diags.push(Diagnostic::error(
                Code::UnexpectedChar,
                format!("unexpected character `{ch}`"),
                span_at(start, start + ch.len_utf8()),
            ));
            i += ch.len_utf8();
            continue;
        }
        toks.push(Token { tok: Tok::Word(text[start..i].to_string()), span: span_at(start, i) });
    }

    toks
}

/// Bytes that may appear inside a bare word. Deliberately permissive so that things like
/// `$24/mo`, `text-sm`, `tasks.open.count`, and `Task[]` lex as one token.
fn is_word_byte(b: u8) -> bool {
    !matches!(b, b' ' | b'\t' | b'"' | b'{' | b'}' | b'|' | b'=' | b':' | b',' | b'>' | b'#')
        && !b.is_ascii_control()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(line: &str) -> Vec<Tok> {
        let lexed = lex(line);
        assert!(!lexed.diagnostics.has_errors(), "{:?}", lexed.diagnostics.items);
        lexed.lines[0].tokens.iter().map(|t| t.tok.clone()).collect()
    }

    #[test]
    fn lexes_a_simple_element() {
        assert_eq!(
            toks("btn Decrement ghost"),
            vec![Tok::Word("btn".into()), Tok::Word("Decrement".into()), Tok::Word("ghost".into())]
        );
    }

    #[test]
    fn action_swallows_rest_of_line() {
        let t = toks("btn Increment primary >count++");
        assert_eq!(t.last(), Some(&Tok::Action("count++".into())));
        assert_eq!(t.len(), 4);
    }

    #[test]
    fn action_keeps_sequenced_statements_intact() {
        let t = toks(r#"form >tasks.add{title:draft}; draft="""#);
        assert_eq!(t.last(), Some(&Tok::Action(r#"tasks.add{title:draft}; draft="""#.into())));
    }

    #[test]
    fn brace_groups_are_captured_verbatim() {
        assert_eq!(
            toks("btn Add primary disabled={!draft.trim()}"),
            vec![
                Tok::Word("btn".into()),
                Tok::Word("Add".into()),
                Tok::Word("primary".into()),
                Tok::Word("disabled".into()),
                Tok::Eq,
                Tok::Brace("!draft.trim()".into()),
            ]
        );
    }

    #[test]
    fn nested_braces_balance() {
        assert_eq!(toks("x {a{b}c}"), vec![Tok::Word("x".into()), Tok::Brace("a{b}c".into())]);
    }

    #[test]
    fn strings_handle_escapes_and_unicode() {
        assert_eq!(
            toks(r#"input draft placeholder="Add a task…""#).last(),
            Some(&Tok::Str("Add a task…".into()))
        );
        assert_eq!(toks(r#"x "a\"b""#).last(), Some(&Tok::Str("a\"b".into())));
    }

    #[test]
    fn anchors_and_routes() {
        assert_eq!(
            toks("link Features #features"),
            vec![
                Tok::Word("link".into()),
                Tok::Word("Features".into()),
                Tok::Anchor("features".into())
            ]
        );
        assert_eq!(
            toks(r#"btn "Get started" primary /signup"#).last(),
            Some(&Tok::Route("/signup".into()))
        );
    }

    #[test]
    fn slash_inside_a_word_is_not_a_route() {
        assert_eq!(
            toks("tier Pro $24/mo"),
            vec![Tok::Word("tier".into()), Tok::Word("Pro".into()), Tok::Word("$24/mo".into())]
        );
    }

    #[test]
    fn type_bodies_lex_as_a_brace_group() {
        assert_eq!(
            toks("type Task {id, title, done:bool}"),
            vec![
                Tok::Word("type".into()),
                Tok::Word("Task".into()),
                Tok::Brace("id, title, done:bool".into()),
            ]
        );
    }

    #[test]
    fn state_domain_uses_pipes() {
        assert_eq!(
            toks("state filter=all|open|done"),
            vec![
                Tok::Word("state".into()),
                Tok::Word("filter".into()),
                Tok::Eq,
                Tok::Word("all".into()),
                Tok::Pipe,
                Tok::Word("open".into()),
                Tok::Pipe,
                Tok::Word("done".into()),
            ]
        );
    }

    #[test]
    fn indentation_and_blank_lines() {
        let src = "page P\n\n// a comment\ncard\n  h Title\n    p Deep\n";
        let lexed = lex(src);
        assert!(!lexed.diagnostics.has_errors());
        let levels: Vec<usize> = lexed.lines.iter().map(|l| l.indent).collect();
        assert_eq!(levels, vec![0, 0, 2, 4]);
        assert_eq!(lexed.lines.len(), 4, "blank lines and comments are dropped");
    }

    #[test]
    fn tabs_are_an_error_but_recover() {
        let lexed = lex("card\n\th Title\n");
        assert!(lexed.diagnostics.has_errors());
        assert_eq!(lexed.diagnostics.items[0].code, Code::TabIndent);
        assert_eq!(lexed.lines.len(), 2, "lexing continues after the error");
    }

    #[test]
    fn rest_from_recovers_raw_prose() {
        let lexed = lex("p Press the buttons to change the value.");
        let line = &lexed.lines[0];
        assert_eq!(line.rest_from(1), "Press the buttons to change the value.");
        assert_eq!(line.rest_from(0), "p Press the buttons to change the value.");
    }

    #[test]
    fn unterminated_string_reports_once_and_recovers() {
        let lexed = lex("btn \"oops");
        assert_eq!(lexed.diagnostics.error_count(), 1);
        assert_eq!(lexed.diagnostics.items[0].code, Code::UnterminatedString);
    }

    #[test]
    fn spans_point_at_the_right_line_and_column() {
        let lexed = lex("page P\ncard\n  btn Go\n");
        let line = &lexed.lines[2];
        let tok = &line.tokens[1]; // `Go`
        assert_eq!(tok.span.line, 3);
        assert_eq!(tok.span.col, 7); // absolute column within the line, 1-based
        assert_eq!(
            &lexed_src_slice("page P\ncard\n  btn Go\n", tok.span.start, tok.span.end),
            "Go"
        );
    }

    fn lexed_src_slice(src: &str, start: usize, end: usize) -> String {
        src[start..end].to_string()
    }
}
