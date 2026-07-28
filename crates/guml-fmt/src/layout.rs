//! Nesting analysis: what depth is a line at, and what is it inside.
//!
//! This mirrors the parser exactly, and it has to. The parser's rule is "children are the
//! following lines with a *strictly greater* indent", applied recursively — so `4` then `5`
//! is not two ragged siblings, it is a parent and a child. A formatter that guessed
//! "those look like siblings" and printed them at the same depth would change the tree.
//! Re-indenting is the one thing this crate does that can alter meaning, so the rule is
//! copied rather than approximated.
//!
//! `tier` and `faq` take *content lines* rather than elements, and the parser flattens
//! everything below them into `text_lines`. Those lines are marked here so the printer
//! passes them through verbatim at one level of indent, which is what the AST records.

use guml_registry::Registry;
use guml_syntax::Line;

#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub depth: usize,
    /// The line is a content line of a `tier`/`faq` block: raw text, not an element.
    pub raw_text_child: bool,
    /// Tag of the enclosing block, for context-sensitive printing (mutations under `data`).
    pub parent_tag: Option<String>,
}

struct Frame {
    indent: usize,
    depth: usize,
    tag: Option<String>,
    /// Set when this frame's descendants are content lines rather than elements, to the
    /// single depth they all print at. The parser flattens them into `text_lines`, so any
    /// extra indent the author used carries no meaning and must not be reproduced.
    content_depth: Option<usize>,
}

pub fn analyse(lines: &[Line], reg: &Registry) -> Vec<Info> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut out = Vec::with_capacity(lines.len());

    for line in lines {
        while stack.last().is_some_and(|f| line.indent <= f.indent) {
            stack.pop();
        }

        let parent = stack.last();
        let content_depth = parent.and_then(|f| f.content_depth);
        let depth = match content_depth {
            Some(d) => d,
            None => parent.map_or(0, |f| f.depth + 1),
        };
        let parent_tag = parent.and_then(|f| f.tag.clone());

        let tag = line.tokens.first().and_then(|t| t.tok.as_word()).map(str::to_string);
        // A content line has no tag of its own, so it can never open a block — but it does
        // keep its descendants inside the same content region.
        let opens_content = content_depth.is_none()
            && tag.as_deref().is_some_and(|t| reg.children_are_text(t));

        out.push(Info { depth, raw_text_child: content_depth.is_some(), parent_tag });
        stack.push(Frame {
            indent: line.indent,
            depth,
            tag,
            content_depth: content_depth.or(opens_content.then_some(depth + 1)),
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use guml_syntax::lex;

    fn analysed(src: &str) -> Vec<Info> {
        let lexed = lex(src);
        analyse(&lexed.lines, &Registry::builtin())
    }

    #[test]
    fn depth_comes_from_the_indent_stack_not_from_the_space_count() {
        let info = analysed("page X\ncard A\n    p One\n        p Two\n    p Three\ncard B\n");
        let depths: Vec<_> = info.iter().map(|i| i.depth).collect();
        assert_eq!(depths, vec![0, 0, 1, 2, 1, 0]);
    }

    #[test]
    fn a_one_space_step_is_still_a_child_because_the_parser_says_so() {
        let info = analysed("page X\ncard A\n  p One\n   p Two\n");
        assert_eq!(info[3].depth, 2, "`p Two` is a child of `p One`, ragged as that looks");
    }

    #[test]
    fn tier_and_faq_children_are_marked_as_content_lines() {
        let info = analysed("page X\ntier Pro $24/mo\n  Unlimited projects\n  Custom domains\n");
        assert!(!info[1].raw_text_child);
        assert!(info[2].raw_text_child && info[3].raw_text_child);
    }

    #[test]
    fn content_lines_stay_flat_even_when_the_author_indented_them_further() {
        // The parser flattens everything below `faq` into `text_lines`, so a deeper indent
        // carries no meaning and must not be reproduced as nesting.
        let info = analysed("page X\nfaq open=1\n  Q one | A one\n      Q two | A two\n");
        assert!(info[2].raw_text_child && info[3].raw_text_child);
        assert_eq!(info[2].depth, 1);
    }

    #[test]
    fn a_mutation_knows_it_is_under_data() {
        let info = analysed("page X\ndata tasks:Task[] GET /api/tasks\n  add POST /api/tasks\n");
        assert_eq!(info[2].parent_tag.as_deref(), Some("data"));
    }
}
