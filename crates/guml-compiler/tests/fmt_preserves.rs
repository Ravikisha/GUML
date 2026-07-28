//! The formatter's one hard invariant: it never changes meaning.
//!
//! `guml-fmt` sits below the parser, so it cannot test this itself — the parser is only
//! reachable from here. Everything else the formatter does is cosmetic and covered by unit
//! tests; this file covers the part that could silently corrupt a document.
//!
//! Spans are excluded from the comparison because they *must* change: re-indenting moves
//! bytes. Everything else — tags, positionals, modifiers, attributes, actions, prose,
//! content lines, nesting — must be identical.

use guml_fmt::{Options, format};
use guml_parser::parse;
use guml_registry::Registry;
use serde_json::Value;

/// Ugly-but-valid inputs. Each one exercises a rule that could go wrong in a way a
/// round-trip would hide.
const CASES: &[(&str, &str)] = &[
    ("ragged indent", "page X\ncard A\n    p One\n        p Two\n    p Three\n"),
    ("tabs", "page X\ncard A\n\tp One\n"),
    ("wide gaps", "page X\nbtn    Add     primary   disabled={!count}\n"),
    ("spaced attrs", "page X\ninput draft placeholder = \"Add a task…\" required\n"),
    ("spaced enum", "page X\nstate filter = all | open | done\n"),
    ("spaced type", "page X\ntype Task {id ,title , done:bool}\n"),
    ("action padding", "page X\nbtn Add primary >   count++  \n"),
    ("multi-statement action", "page X\nform >tasks.add{title:draft} ;  draft=\"\"\n"),
    ("content pipe", "page X\ncard \"Ship it\"|Describe the page, get a build.\n"),
    (
        "content lines",
        "page X\ntier Pro $24/mo \"For devs\" cta=\"Go Pro\" /signup featured\n      Unlimited projects\n      Custom domains\n",
    ),
    ("faq pairs", "page X\nfaq open=1\n  Can I export? | Yes, plain source.\n"),
    ("prose spacing", "page X\np Two  spaces   and a  tab-free  line\n"),
    ("comments everywhere", "// top\npage X\n// about state\nstate count=0\n\ncard A\n  // inner\n  p One\n"),
    (
        "directives out of order",
        "page X\ncard A\n  p One\nstate draft=\"\"\ntype Task {id, done:bool}\ndata tasks:Task[] GET /api/tasks\n  add POST /api/tasks {title} optimistic:prepend\n",
    ),
    ("quoted single word", "page X\nbtn \"Add\" primary\n"),
    ("no trailing newline", "page X\ncard A"),
    ("crlf", "page X\r\ncard A\r\n  p One\r\n"),
];

fn ast_json(src: &str) -> Value {
    let reg = Registry::builtin();
    let parsed = parse(src, &reg);
    let mut v: Value = serde_json::to_value(&parsed.program).expect("AST serialises");
    strip_spans(&mut v);
    v
}

/// Byte offsets and line numbers move when the text moves; that is the formatter working,
/// not the formatter breaking.
fn strip_spans(v: &mut Value) {
    match v {
        Value::Object(map) => {
            map.remove("span");
            for (_, child) in map.iter_mut() {
                strip_spans(child);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(strip_spans),
        _ => {}
    }
}

#[test]
fn formatting_preserves_the_ast() {
    for (name, src) in CASES {
        for opts in [Options::default(), Options::canonical()] {
            let out = format(src, opts).text;
            if opts.canonical {
                // Canonical mode reorders declarations on purpose, so compare the tree and
                // the declaration *set* rather than declaration order.
                let (a, b) = (ast_json(src), ast_json(&out));
                assert_eq!(a["tree"], b["tree"], "canonical changed the element tree for {name}\n{out}");
                for key in ["states", "resources", "types", "page"] {
                    assert_eq!(
                        sorted(&a[key]),
                        sorted(&b[key]),
                        "canonical changed `{key}` for {name}\n{out}"
                    );
                }
            } else {
                assert_eq!(ast_json(src), ast_json(&out), "format changed the AST for {name}\n{out}");
            }
        }
    }
}

fn sorted(v: &Value) -> Vec<String> {
    match v {
        Value::Array(items) => {
            let mut s: Vec<String> = items.iter().map(|i| i.to_string()).collect();
            s.sort();
            s
        }
        other => vec![other.to_string()],
    }
}

#[test]
fn formatting_preserves_the_ast_of_every_fixture() {
    for name in ["a.guml", "b.guml", "c.guml"] {
        let src = std::fs::read_to_string(format!("../../fixtures/{name}")).expect("fixture");
        for opts in [Options::default(), Options::canonical()] {
            let out = format(&src, opts).text;
            let (a, b) = (ast_json(&src), ast_json(&out));
            assert_eq!(a["tree"], b["tree"], "{name} tree changed\n{out}");
        }
    }
}

#[test]
fn formatting_does_not_introduce_diagnostics() {
    let reg = Registry::builtin();
    for name in ["a.guml", "b.guml", "c.guml"] {
        let src = std::fs::read_to_string(format!("../../fixtures/{name}")).expect("fixture");
        let before = parse(&src, &reg).diagnostics.len();
        let after = parse(&format(&src, Options::default()).text, &reg).diagnostics.len();
        assert_eq!(before, after, "{name}: formatting changed the diagnostic count");
    }
}

#[test]
fn the_fixtures_are_already_formatted() {
    // The published token counts are measured on these files. If the formatter disagrees
    // with them, one of the two is wrong and the numbers stop meaning anything.
    for name in ["a.guml", "b.guml", "c.guml"] {
        let src = std::fs::read_to_string(format!("../../fixtures/{name}")).expect("fixture");
        let out = format(&src, Options::default());
        assert!(!out.changed, "{name} is not in formatted form:\n{}", out.text);
    }
}

#[test]
fn canonical_form_is_stable_across_cosmetic_rewrites() {
    // The property the benchmark depends on: two generations that differ only in layout
    // must canonicalise to the same bytes, or dedup and consistency scoring are noise.
    let a = "page X\nstate count=0\n\ncard A\n  p One\n";
    let b = "// a note\npage X\n\n\nstate count=0\ncard A\n      p One\n";
    assert_eq!(format(a, Options::canonical()).text, format(b, Options::canonical()).text);
}
