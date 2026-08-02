//! Rendering one document.
//!
//! Everything here is a choice about what is *discretionary*. Indentation, spacing between
//! tokens and attribute punctuation are the formatter's to decide. Prose, content lines and
//! action bodies are the author's, and are copied byte for byte — a formatter that respaced
//! prose would be editing the product's copy, and `p Two  spaces` is a content change, not a
//! style one.

use crate::layout::Info;
use crate::{INDENT, Options};
use guml_registry::{Registry, TagKind};
use guml_syntax::{Lexed, Line, Tok, Token};

use crate::trivia::Item;

/// Directives are declarations, so their order carries no meaning and canonical mode may
/// sort them. Elements are the document, and are never reordered.
/// Top-level directives, in canonical order.
///
/// `on` was missing, and two things went wrong because of it. In canonical form an `on` effect sorted into
/// `rest` — after the element tree — so a document's effects moved below its markup. And in normal
/// formatting an `on` line looked like the first line of the *tree*, so the blank line marking the
/// declaration/tree seam was inserted in front of it, and a comment introducing the effect was left stranded
/// above that blank with nothing under it.
///
/// Ranked after `state`/`store`, because an effect's trigger reads them.
const DIRECTIVES: &[&str] =
    &["page", "type", "data", "state", "store", "on", "route", "auth", "def"];

fn directive_rank(tag: &str) -> Option<usize> {
    DIRECTIVES.iter().position(|d| *d == tag)
}

fn tag_of(line: &Line) -> Option<&str> {
    line.tokens.first().and_then(|t| t.tok.as_word())
}

pub fn render(
    doc: &[Item],
    lexed: &Lexed,
    infos: &[Info],
    broken: &[u32],
    reg: &Registry,
    opts: Options,
) -> String {
    let lines = &lexed.lines;
    let aligned = align_mutation_columns(lines, infos, reg, opts);

    let mut out = String::new();
    if opts.canonical {
        for idx in canonical_order(lines, infos) {
            push_line(&mut out, lines, infos, &aligned, idx, broken, reg, opts);
        }
        return out;
    }

    let mut pending_blank = false;
    let mut seen_code = false;
    let mut directives_done = false;

    for (pos, item) in doc.iter().enumerate() {
        match item {
            Item::Blank => {
                if seen_code {
                    pending_blank = true;
                }
            }
            Item::Comment(text) => {
                // A comment belongs to what it introduces, so it takes the indentation of
                // the next code line rather than the one it happens to follow.
                let depth = next_code_depth(doc, pos, infos).unwrap_or(0);
                // And the seam blank goes *above* it, for the same reason. If the code line this comment
                // introduces is the one that ends the declaration block, inserting the blank when that line
                // is reached puts it between the comment and its subject — the comment ends up attached to
                // nothing, which is worse than the missing blank it was fixing.
                if seen_code
                    && !directives_done
                    && next_code_is_tree(doc, pos, lines, infos, directives_done)
                {
                    directives_done = true;
                    pending_blank = true;
                }
                flush_blank(&mut out, &mut pending_blank);
                out.push_str(&" ".repeat(depth * INDENT));
                out.push_str(text);
                out.push('\n');
                seen_code = true;
            }
            Item::Code(idx) => {
                let is_directive = infos[*idx].depth == 0
                    && tag_of(&lines[*idx]).and_then(directive_rank).is_some();
                let child_of_directive = infos[*idx].depth > 0 && !directives_done;

                // One blank line marks the seam between the declarations and the tree.
                if seen_code && !is_directive && !child_of_directive && !directives_done {
                    directives_done = true;
                    pending_blank = true;
                }

                flush_blank(&mut out, &mut pending_blank);
                push_line(&mut out, lines, infos, &aligned, *idx, broken, reg, opts);
                seen_code = true;
            }
        }
    }

    out
}

fn flush_blank(out: &mut String, pending: &mut bool) {
    if *pending {
        out.push('\n');
        *pending = false;
    }
}

/// Whether the next code line after `from` is the first line of the element tree.
///
/// The same test the `Code` arm applies, factored out so the comment above a line and the line itself
/// cannot disagree about where the seam is.
fn next_code_is_tree(
    doc: &[Item],
    from: usize,
    lines: &[Line],
    infos: &[Info],
    directives_done: bool,
) -> bool {
    let Some(idx) = doc[from + 1..].iter().find_map(|i| match i {
        Item::Code(idx) => Some(*idx),
        _ => None,
    }) else {
        return false;
    };
    let is_directive =
        infos[idx].depth == 0 && tag_of(&lines[idx]).and_then(directive_rank).is_some();
    let child_of_directive = infos[idx].depth > 0 && !directives_done;
    !is_directive && !child_of_directive
}

fn next_code_depth(doc: &[Item], from: usize, infos: &[Info]) -> Option<usize> {
    doc[from + 1..].iter().find_map(|i| match i {
        Item::Code(idx) => Some(infos[*idx].depth),
        _ => None,
    })
}

/// Canonical order: every top-level directive block, sorted by kind and stable within it,
/// then the element tree untouched. A `data` block travels with its mutations.
fn canonical_order(lines: &[Line], infos: &[Info]) -> Vec<usize> {
    let mut blocks: Vec<(usize, usize, Vec<usize>)> = Vec::new(); // (rank, seq, indices)
    let mut rest: Vec<usize> = Vec::new();
    let mut i = 0usize;
    let mut seq = 0usize;

    while i < lines.len() {
        let rank =
            (infos[i].depth == 0).then(|| tag_of(&lines[i]).and_then(directive_rank)).flatten();
        match rank {
            Some(rank) => {
                let mut group = vec![i];
                i += 1;
                while i < lines.len() && infos[i].depth > 0 {
                    group.push(i);
                    i += 1;
                }
                blocks.push((rank, seq, group));
                seq += 1;
            }
            None => {
                rest.push(i);
                i += 1;
            }
        }
    }

    blocks.sort_by_key(|(rank, seq, _)| (*rank, *seq));
    blocks.into_iter().flat_map(|(_, _, g)| g).chain(rest).collect()
}

#[allow(clippy::too_many_arguments)]
fn push_line(
    out: &mut String,
    lines: &[Line],
    infos: &[Info],
    aligned: &[Option<String>],
    idx: usize,
    broken: &[u32],
    reg: &Registry,
    opts: Options,
) {
    let line = &lines[idx];
    let info = &infos[idx];
    // `extra_indent` is non-zero only inside a `js`/`raw` body, where the nesting belongs to
    // another language and is reproduced rather than normalised.
    out.push_str(&" ".repeat(info.depth * INDENT + info.extra_indent));
    out.push_str(&render_line(line, info, aligned[idx].as_deref(), broken, reg, opts));
    out.push('\n');
}

fn render_line(
    line: &Line,
    info: &Info,
    aligned: Option<&str>,
    broken: &[u32],
    reg: &Registry,
    opts: Options,
) -> String {
    // A line the lexer could not read is returned as written. Reprinting from a token list
    // that is known to be wrong turns a small error into lost work.
    if broken.contains(&line.line_no) {
        return line.text.trim_end().to_string();
    }
    // Content lines under `tier`/`faq` are stored verbatim in the AST, so touching them —
    // even normalising the spaces around `|` — would change the tree.
    if info.raw_text_child {
        return line.text.trim_end().to_string();
    }
    if let Some(text) = aligned {
        return text.to_string();
    }

    let Some(tag) = tag_of(line) else {
        return line.text.trim_end().to_string();
    };

    if directive_rank(tag).is_some() {
        return render_directive(tag, line, opts);
    }
    // Text tags take the whole remainder as prose: never re-tokenised, never respaced.
    if reg.kind(tag) == Some(TagKind::Text) {
        let rest = line.rest_from(1).trim_end();
        return if rest.is_empty() { tag.to_string() } else { format!("{tag} {rest}") };
    }

    render_tokens(&line.tokens, line, reg, opts)
}

fn render_directive(tag: &str, line: &Line, opts: Options) -> String {
    match tag {
        // `type Task {id, title, done:bool}` — the brace body is a field list, so commas
        // get one space after and none before.
        "type" => {
            let name = line.tokens.get(1).and_then(|t| t.tok.as_word()).unwrap_or("");
            let body = line
                .tokens
                .iter()
                .find_map(|t| match &t.tok {
                    Tok::Brace(inner) => Some(normalise_fields(inner)),
                    _ => None,
                })
                .unwrap_or_default();
            format!("type {name} {{{body}}}")
        }
        // `state filter=all|open|done` — here `|` separates the members of an enumerated
        // domain, not content from structure, so it stays tight.
        "state" | "store" => {
            let mut out = String::new();
            let mut glue = false;
            for t in &line.tokens {
                match &t.tok {
                    Tok::Eq | Tok::Pipe | Tok::Colon => {
                        out.push_str(match t.tok {
                            Tok::Eq => "=",
                            Tok::Pipe => "|",
                            _ => ":",
                        });
                        glue = true;
                    }
                    _ => {
                        if !out.is_empty() && !glue {
                            out.push(' ');
                        }
                        out.push_str(&token_text(&t.tok, false, opts));
                        glue = false;
                    }
                }
            }
            out
        }
        _ => render_tokens(&line.tokens, line, &Registry::builtin(), opts),
    }
}

/// General structured line: positionals, modifiers, attributes, optional `| content`, and a
/// trailing action.
fn render_tokens(toks: &[Token], line: &Line, reg: &Registry, opts: Options) -> String {
    let mut out = String::new();
    let mut glue = false;
    let mut positional = true; // until the first attribute or content marker

    for (i, t) in toks.iter().enumerate() {
        match &t.tok {
            // Everything after `|` is content: raw, one space each side of the bar.
            Tok::Pipe => {
                let rest = line.rest_from(i + 1).trim();
                out.push_str(" | ");
                out.push_str(rest);
                return out;
            }
            Tok::Action(body) => {
                out.push_str(" >");
                out.push_str(&normalise_action(body, opts));
                return out;
            }
            Tok::Eq => {
                out.push('=');
                glue = true;
                positional = false;
            }
            Tok::Colon => {
                out.push(':');
                glue = true;
            }
            Tok::Comma => {
                out.push(',');
                glue = false;
            }
            _ => {
                if !out.is_empty() && !glue {
                    out.push(' ');
                }
                out.push_str(&token_text(&t.tok, positional, opts));
                glue = false;
            }
        }
    }

    let _ = reg;
    out
}

fn token_text(tok: &Tok, positional: bool, opts: Options) -> String {
    match tok {
        Tok::Word(w) | Tok::Num(w) => w.clone(),
        Tok::Str(s) => {
            if opts.canonical && positional && can_drop_quotes(s) {
                s.clone()
            } else {
                format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            }
        }
        Tok::Brace(inner) => format!("{{{inner}}}"),
        Tok::Anchor(a) => format!("#{a}"),
        Tok::Route(r) => r.clone(),
        Tok::Pipe => "|".into(),
        Tok::Eq => "=".into(),
        Tok::Colon => ":".into(),
        Tok::Comma => ",".into(),
        // Reached only if an action appears somewhere the line printer did not already
        // consume it; `>` swallows the rest of the line, so it is always terminal.
        Tok::Action(body) => format!(">{body}"),
    }
}

/// Quotes come off only when the bare form lexes back to the same value *and* means the same
/// thing. A one-word label that collides with a modifier or a tag name keeps them, because
/// `btn "primary"` and `btn primary` are different documents.
fn can_drop_quotes(s: &str) -> bool {
    let reg = Registry::builtin();
    !s.is_empty()
        && s.chars().all(|c| {
            !c.is_whitespace()
                && !matches!(c, '"' | '{' | '}' | '|' | '=' | ':' | ',' | '>' | '#' | '\\')
        })
        && !s.starts_with('/')
        && !s.chars().next().is_some_and(|c| c.is_ascii_digit())
        && !reg.is_modifier(s)
        && reg.get(s).is_none()
        && directive_rank(s).is_none()
}

fn normalise_fields(inner: &str) -> String {
    inner.split(',').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ")
}

/// Action bodies are stored in the AST *verbatim*, so the formatter may only trim the
/// ends. Normalising the spacing around `;` looked safe and is not: it rewrites the exact
/// string the tree records, and the AST-preservation test caught it. When the expression
/// parser lands (Phase 2) actions become structure rather than text, and canonical mode can
/// format them properly.
fn normalise_action(body: &str, _opts: Options) -> String {
    body.trim().to_string()
}

/// Mutation lines under a `data` block are a table, and the fixtures already write them as
/// one by hand. Aligning the columns is the formatter agreeing with the author rather than
/// overruling them. Canonical mode collapses the padding, because alignment is discretionary
/// whitespace and two documents that differ only in it must produce the same bytes.
fn align_mutation_columns(
    lines: &[Line],
    infos: &[Info],
    reg: &Registry,
    opts: Options,
) -> Vec<Option<String>> {
    let mut out = vec![None; lines.len()];
    if opts.canonical {
        return out;
    }

    let mut i = 0usize;
    while i < lines.len() {
        if infos[i].parent_tag.as_deref() != Some("data") || infos[i].raw_text_child {
            i += 1;
            continue;
        }
        let start = i;
        let depth = infos[i].depth;
        while i < lines.len()
            && infos[i].parent_tag.as_deref() == Some("data")
            && infos[i].depth == depth
        {
            i += 1;
        }

        let rows: Vec<Option<Vec<String>>> =
            lines[start..i].iter().map(|l| mutation_fields(l, reg, opts)).collect();
        if rows.iter().any(Option::is_none) {
            // One odd line and the whole table is off; fall back for the group.
            continue;
        }
        let rows: Vec<Vec<String>> = rows.into_iter().flatten().collect();
        // Pad every column except the last one present on each row.
        let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
        let widths: Vec<usize> = (0..cols)
            .map(|c| {
                rows.iter().filter_map(|r| r.get(c)).map(|s| s.chars().count()).max().unwrap_or(0)
            })
            .collect();

        for (row, slot) in rows.iter().zip(out[start..i].iter_mut()) {
            let mut text = String::new();
            for (c, field) in row.iter().enumerate() {
                if c > 0 {
                    text.push(' ');
                }
                if c + 1 < row.len() {
                    let pad = widths[c].saturating_sub(field.chars().count());
                    text.push_str(field);
                    text.push_str(&" ".repeat(pad));
                } else {
                    text.push_str(field);
                }
            }
            *slot = Some(text);
        }
    }

    out
}

/// `add POST /api/tasks {title} optimistic:prepend` → five columns: name, method, url,
/// body, flags. Columns are classified by token kind rather than by position, so a line
/// that does not fit the shape (no route, say) returns `None` and is printed normally
/// instead of being forced into a table it does not belong in.
///
/// The body column is padded even when empty, because a mutation without a body still has
/// to keep `optimistic` under the other rows' flags — which is exactly what the fixtures do
/// by hand.
fn mutation_fields(line: &Line, reg: &Registry, opts: Options) -> Option<Vec<String>> {
    let mut name = None;
    let mut method = None;
    let mut url = None;
    let mut body = None;
    let mut flags: Vec<String> = Vec::new();
    let mut glue = false;

    for t in &line.tokens {
        match &t.tok {
            Tok::Word(w) if name.is_none() => name = Some(w.clone()),
            Tok::Word(w) if method.is_none() && w.chars().all(|c| c.is_ascii_uppercase()) => {
                method = Some(w.clone());
            }
            Tok::Route(r) if url.is_none() => url = Some(r.clone()),
            Tok::Brace(inner) if body.is_none() => body = Some(format!("{{{inner}}}")),
            Tok::Colon => {
                flags.last_mut()?.push(':');
                glue = true;
            }
            other => {
                let text = token_text(other, false, opts);
                if glue {
                    flags.last_mut()?.push_str(&text);
                    glue = false;
                } else {
                    flags.push(text);
                }
            }
        }
    }

    let (name, method, url) = (name?, method?, url?);
    let _ = reg;
    let mut fields = vec![name, method, url, body.unwrap_or_default()];
    if !flags.is_empty() {
        fields.push(flags.join(" "));
    }
    // A trailing empty body column would leave dangling spaces at end of line.
    while fields.last().is_some_and(String::is_empty) {
        fields.pop();
    }
    Some(fields)
}
