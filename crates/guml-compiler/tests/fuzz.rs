//! Robustness under hostile and malformed input.
//!
//! # Why in-repo rather than `cargo-fuzz`
//!
//! `cargo-fuzz` needs a nightly toolchain and libFuzzer, so it cannot run in the same CI job as
//! everything else and in practice would run rarely or never. This is a seeded generator plus
//! property assertions: no new dependency, deterministic, and it runs on every push. It finds
//! shallower bugs than coverage-guided fuzzing, but it finds them every time rather than when
//! somebody remembers.
//!
//! # What is asserted
//!
//! Not "the output is correct" — for garbage input there is no correct output. The properties are
//! the ones a *host* depends on:
//!
//! 1. **No panic.** A GUML document may come from an untrusted agent. A panic in the browser wasm
//!    build takes the page down; in the language server it takes the editor's diagnostics with it.
//! 2. **Termination.** Every parser here is hand-written, and a loop that fails to consume on an
//!    unexpected token is a hang rather than a crash — which is worse, because it looks like
//!    slowness.
//! 3. **Spans stay inside the source.** A diagnostic that points past the end of the file makes an
//!    editor throw when it tries to highlight it, and makes `guml fix` slice out of bounds.
//! 4. **The formatter never invents meaning.** Format twice, get the same thing; and if the input
//!    parsed, the AST survives.

use guml_codegen::Backend as _;
use guml_compiler::check;
use guml_fmt::{Options, format};

/// A small deterministic PRNG. `rand` is not a dependency, and a fixed algorithm means a failure
/// found in CI reproduces exactly on a laptop.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

/// Fragments drawn from real GUML, plus the things that break it.
const PIECES: &[&str] = &[
    "page P",
    "state count=0",
    "state filter=all|open|done",
    "type T {id, done:bool}",
    "data rows:T[] GET /api/rows",
    "  add POST /api/rows {id} optimistic:prepend",
    "card Hi",
    "  p prose text",
    "btn Go primary >count++",
    "list rows",
    "  text {id}",
    "  empty nothing",
    "tabs filter",
    "faq open=1",
    "  Q | A",
    "tier Pro $9/mo",
    "section #a Title cols=3",
    "link X #a",
    "input draft aria=\"D\"",
    "metric {count}",
    "head Total {rows.open.count}",
    "toggle {done} aria=\"T\"",
    // Malformed: unbalanced, wrong types, hostile expressions, control characters.
    "\t tab indented",
    "  \t mixed",
    "crad typo",
    "btn \"unterminated",
    "metric {unclosed",
    "state x=",
    "data z:Missing[] FETCH nopath",
    "btn X >window.location=\"/x\"",
    "metric {a ? b : c}",
    "metric {fetch(url)}",
    "p {{{{{{",
    "}}}}}}",
    "===",
    ">>>",
    "state count=0|",
    "|",
    "#",
    "/",
    ">",
    "\"",
    "{",
    "}",
    "…",
    "🙂 emoji",
    "\u{0}\u{1}",
    "                                                  deep indent",
    "p ",
];

fn document(rng: &mut Rng, lines: usize) -> String {
    let mut out = String::new();
    for _ in 0..lines {
        out.push_str(rng.pick(PIECES));
        out.push('\n');
    }
    out
}

/// Panics and hangs, over a corpus generated from a fixed seed.
#[test]
fn nothing_panics_on_generated_documents() {
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    // 4,000 documents rather than a million: this runs on every push, and the marginal document
    // from a non-guided generator finds very little. The million-iteration gate in the roadmap is
    // for a coverage-guided run, which is a separate job.
    for i in 0..4_000 {
        let lines = 1 + rng.below(12);
        let src = document(&mut rng, lines);

        let (program, diags) = check(&src);

        // Spans have to be inside the source, or an editor throws when it highlights one and
        // `guml fix` slices out of bounds.
        for d in &diags.items {
            assert!(
                d.span.start <= d.span.end && d.span.end <= src.len(),
                "iteration {i}: {} has span {}..{} in a {}-byte document\n{src:?}",
                d.id,
                d.span.start,
                d.span.end,
                src.len()
            );
            assert!(d.span.line >= 1, "iteration {i}: {} reports line 0", d.id);
            // And a span must land on a character boundary, or slicing it panics.
            assert!(
                src.is_char_boundary(d.span.start) && src.is_char_boundary(d.span.end),
                "iteration {i}: {} span is not on a char boundary\n{src:?}",
                d.id
            );
        }

        // The formatter runs on invalid input by design — that is its main caller.
        let once = format(&src, Options::default());
        let twice = format(&once.text, Options::default());
        assert_eq!(twice.text, once.text, "iteration {i}: formatting is not idempotent\n{src:?}");

        let canonical = format(&src, Options::canonical());
        let canonical_twice = format(&canonical.text, Options::canonical());
        assert_eq!(
            canonical_twice.text, canonical.text,
            "iteration {i}: canonical form is not idempotent\n{src:?}"
        );

        // Codegen only runs on documents that parsed, which mirrors the driver.
        if !diags.has_errors() {
            let out = guml_codegen::react::ReactBackend.emit(&program);
            assert!(!out.files.is_empty(), "iteration {i}: no output for a valid document");
        }
    }
}

/// Formatting a document that parses must not change what it means.
#[test]
fn formatting_preserves_meaning_on_generated_documents() {
    let mut rng = Rng(0xC0FF_EE00_1234_5678);
    let mut checked = 0;

    for i in 0..2_000 {
        let lines = 1 + rng.below(10);
        let src = document(&mut rng, lines);
        let (before, diags) = check(&src);
        if diags.has_errors() {
            continue;
        }
        checked += 1;

        let formatted = format(&src, Options::default()).text;
        let (after, _) = check(&formatted);

        // Compared structurally rather than by span, since re-indenting moves every offset.
        assert_eq!(
            shape(&before),
            shape(&after),
            "iteration {i}: formatting changed the document\n--- before\n{src}\n--- after\n{formatted}"
        );
    }

    // A generator that never produces a valid document would make this test vacuous while still
    // passing, which is the failure mode of every property test written from fragments.
    assert!(checked > 50, "only {checked} generated documents parsed — the corpus is too hostile");
}

/// A structural fingerprint: tags, nesting, declarations, actions. Everything except position.
fn shape(program: &guml_ast::Program) -> String {
    fn walk(el: &guml_ast::Element, depth: usize, out: &mut String) {
        out.push_str(&format!(
            "{}{} p={} a={:?} c={:?} t={:?}\n",
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
    out.push_str(&format!("page={:?}\n", program.page.as_ref().map(|p| &p.name)));
    for t in &program.types {
        out.push_str(&format!(
            "type {} {:?}\n",
            t.name,
            t.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
        ));
    }
    for s in &program.states {
        out.push_str(&format!("state {} {:?} {:?}\n", s.name, s.init, s.domain));
    }
    for r in &program.resources {
        out.push_str(&format!("data {} {} {} {}\n", r.name, r.ty, r.method, r.url));
    }
    for el in &program.tree {
        walk(el, 0, &mut out);
    }
    out
}

/// The expression parser, separately: it is the newest and it consumes the least structured input.
#[test]
fn the_expression_parser_terminates_on_hostile_input() {
    let mut rng = Rng(0xDEAD_BEEF_0000_0001);
    const ATOMS: &[&str] = &[
        "count",
        "tasks.open.count",
        "draft.trim()",
        "!",
        "-",
        "(",
        ")",
        "&&",
        "||",
        "==",
        "!=",
        "<",
        ">=",
        "+",
        "*",
        "/",
        "\"str",
        "\"",
        "1",
        "1.2.3",
        ".",
        "..",
        "a.b.c.d.e",
        "{",
        "}",
        "true",
        "false",
        "…",
        "\u{0}",
    ];

    for _ in 0..20_000 {
        let mut expr = String::new();
        for _ in 0..rng.below(12) {
            expr.push_str(rng.pick(ATOMS));
            if rng.below(3) == 0 {
                expr.push(' ');
            }
        }
        // Both entry points: the tree, and the JavaScript lowering that consumes it.
        let parsed = guml_syntax::expr::parse(&expr);
        let _ = guml_codegen::expr::lower(&expr);
        // `idents` and `head_ident` walk the tree; a cycle or a stack overflow would show here.
        let _ = parsed.idents();
        let _ = parsed.head_ident();
        let _ = parsed.is_computed();
    }
}

/// Deep nesting, long lines, and long documents — the shapes that overflow a recursive descent
/// parser or a recursive printer.
#[test]
fn extreme_shapes_do_not_overflow() {
    // 200 levels of nesting. The parser recurses per level, and so does the React renderer.
    let mut deep = String::from("page P\n");
    for i in 0..200 {
        deep.push_str(&"  ".repeat(i));
        deep.push_str("card X\n");
    }
    let (program, _) = check(&deep);
    let _ = guml_codegen::react::ReactBackend.emit(&program);
    let _ = format(&deep, Options::default());

    // One very long line.
    let long = format!("page P\np {}\n", "word ".repeat(20_000));
    let (program, _) = check(&long);
    let _ = guml_codegen::react::ReactBackend.emit(&program);
    let _ = format(&long, Options::default());

    // Many short lines.
    let many = format!("page P\n{}", "card X\n  p y\n".repeat(5_000));
    let (program, diags) = check(&many);
    assert!(!diags.has_errors(), "10,000 valid lines should still compile");
    let _ = guml_codegen::react::ReactBackend.emit(&program);

    // A deeply parenthesised expression, which is where a precedence-climbing parser recurses.
    let nested = format!("page P\nmetric {{{}count{}}}\n", "(".repeat(500), ")".repeat(500));
    let _ = check(&nested);
}
