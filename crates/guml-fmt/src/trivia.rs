//! Recovering what the lexer throws away.
//!
//! `guml_syntax::lex` drops blank lines and `//` comments before they ever become tokens —
//! correct for a compiler, fatal for a formatter. This module re-reads the source and
//! rebuilds the full document as an ordered list of items, so a comment can be re-emitted
//! at the indentation of the code it belongs to.
//!
//! The lexer is left alone deliberately: it runs on every keystroke through the LSP and on
//! every repair-loop round, and it should not carry a cost that only the formatter pays.

use guml_syntax::Lexed;

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// A significant line; the index is into `Lexed::lines`.
    Code(usize),
    /// A `//` line, already trimmed of surrounding whitespace.
    Comment(String),
    Blank,
}

pub fn split(src: &str, lexed: &Lexed) -> Vec<Item> {
    // `lex` numbers lines from 1 over `split('\n')`, so the mapping back is exact.
    let mut by_line = vec![usize::MAX; src.split('\n').count() + 2];
    for (i, line) in lexed.lines.iter().enumerate() {
        if let Some(slot) = by_line.get_mut(line.line_no as usize) {
            *slot = i;
        }
    }

    let mut items = Vec::new();
    for (idx, raw) in src.split('\n').enumerate() {
        let line_no = idx + 1;
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = raw.trim();

        match by_line.get(line_no).copied() {
            Some(i) if i != usize::MAX => items.push(Item::Code(i)),
            _ if trimmed.starts_with("//") => items.push(Item::Comment(trimmed.to_string())),
            _ if trimmed.is_empty() => items.push(Item::Blank),
            // A non-empty line the lexer skipped can only be the phantom final element of
            // `split('\n')` on a file that ends in a newline.
            _ => {}
        }
    }

    // Drop the trailing blank produced by the final newline; the printer adds exactly one.
    while items.last() == Some(&Item::Blank) {
        items.pop();
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use guml_syntax::lex;

    #[test]
    fn interleaves_comments_blanks_and_code_in_source_order() {
        let src = "// top\npage X\n\ncard Hi\n";
        let items = split(src, &lex(src));
        assert_eq!(
            items,
            vec![
                Item::Comment("// top".into()),
                Item::Code(0),
                Item::Blank,
                Item::Code(1),
            ]
        );
    }

    #[test]
    fn an_indented_comment_is_captured_without_its_indent() {
        let src = "page X\n    // inner\n";
        let items = split(src, &lex(src));
        assert_eq!(items[1], Item::Comment("// inner".into()));
    }

    #[test]
    fn trailing_blank_lines_are_dropped() {
        let src = "page X\n\n\n";
        assert_eq!(split(src, &lex(src)), vec![Item::Code(0)]);
    }
}
