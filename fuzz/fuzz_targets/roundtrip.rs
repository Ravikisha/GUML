//! Formatting must not change what a document means.
//!
//! The property: for any document that parses, `ast(fmt(x)) == ast(x)`. Compared structurally rather
//! than by span, because re-indenting moves every byte offset.
//!
//! This is the most valuable target of the four. A formatter that silently alters meaning is a compiler
//! bug wearing a cosmetic disguise, and it would be applied to every file in a repository by a
//! format-on-save hook before anyone noticed.
#![no_main]

use libfuzzer_sys::fuzz_target;

fn shape(program: &guml_ast::Program) -> String {
    fn walk(el: &guml_ast::Element, depth: usize, out: &mut String) {
        out.push_str(&format!(
            "{}{} p={} a={:?} c={:?} t={:?}
",
            "  ".repeat(depth),
            el.tag,
            el.positionals.len(),
            el.actions,
            el.content,
            el.text_lines
        ));
        for child in &el.children {
            walk(child, depth + 1, out);
        }
    }

    let mut out = String::new();
    out.push_str(&format!("page={:?}
", program.page.as_ref().map(|p| &p.name)));
    for t in &program.types {
        out.push_str(&format!("type {} {}
", t.name, t.fields.len()));
    }
    for s in &program.states {
        out.push_str(&format!("state {} {:?}
", s.name, s.domain));
    }
    for r in &program.resources {
        out.push_str(&format!("data {} {} {} {}
", r.name, r.ty, r.method, r.url));
    }
    for d in &program.defs {
        out.push_str(&format!("def {} {:?}
", d.name, d.params));
    }
    for el in &program.tree {
        walk(el, 0, &mut out);
    }
    out
}

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };

    let (before, diags) = guml_compiler::check(src);
    if diags.has_errors() {
        return;
    }

    let formatted = guml_fmt::format(src, guml_fmt::Options::default()).text;
    let (after, _) = guml_compiler::check(&formatted);

    assert_eq!(shape(&before), shape(&after), "formatting changed the document
--- before
{src}
--- after
{formatted}");
});
