//! `js` and `raw` escape hatches.
//!
//! Two independent things are being pinned down here, and they pull in opposite directions:
//!
//! 1. **The hatch works.** Emitted code contains the block verbatim — no reformatting, no escaping,
//!    no validation. If a hatch silently mangles its contents it is not a hatch, and the
//!    escape-hatch rate stops being a measurable quantity because people work around it instead.
//! 2. **The hatch does not travel.** The JSON backend feeds the browser runtime, which renders
//!    documents that may have come from an untrusted agent. So the block *body* is dropped there
//!    rather than passed through. The security boundary is that GUML actions are not
//!    Turing-complete; a `js` block that reached a client-side `eval` would erase it.
//!
//! Every block also gets a `GUML0090` note, so the rate is countable from `check --format json`
//! rather than by grepping.

use guml_codegen::Backend as _;
use guml_compiler::check;

const DOC: &str = r#"page P
card Hi
  p x
raw react
  <SomeChart data={rows} />
js
  const fmt = (n) => n.toFixed(2);
"#;

#[test]
fn escape_blocks_are_a_note_not_an_error() {
    let (_, diags) = check(DOC);
    assert!(!diags.has_errors(), "an escape hatch must not fail the build: {:?}", diags.items);

    let notes: Vec<_> = diags.items.iter().filter(|d| d.id == "GUML0090").collect();
    assert_eq!(notes.len(), 2, "one note per block, got {:?}", diags.items);
    // The count is in the message so the escape-hatch rate can be weighted by size, not just by
    // block count — a one-line `raw` and a 200-line `js` are not the same admission.
    assert!(notes.iter().all(|d| d.message.contains("1 line")), "{notes:?}");
}

#[test]
fn escape_tags_are_outside_the_component_vocabulary() {
    // `js`/`raw` are recognised before the registry lookup, so they are deliberately not
    // components: no attributes, no design-system classes, nothing the registry promises.
    let registry = guml_registry::Registry::builtin();
    for tag in ["js", "raw"] {
        assert!(registry.get(tag).is_none(), "`{tag}` must not be a registry component");
        // And a typo near them must not be steered into a hatch: `guml fix` would then turn a
        // misspelled component into unchecked code.
        assert_ne!(registry.suggest("jsx"), Some(tag));
    }
}

#[test]
fn react_emits_the_block_verbatim() {
    let (program, diags) = check(DOC);
    assert!(!diags.has_errors());
    let src = &guml_codegen::react::ReactBackend.emit(&program).files[0].contents;

    // Verbatim, including the arrow function and the JSX the compiler would never generate.
    assert!(src.contains("const fmt = (n) => n.toFixed(2);"), "{src}");
    assert!(src.contains("<SomeChart data={rows} />"), "{src}");

    // `js` is component-body code, so it belongs above the return; `raw` belongs where it sits in
    // the tree. Getting these the wrong way round produces code that parses and does nothing.
    //
    // Anchored on the *component's* return. A document using an escape hatch also gets an error boundary
    // (that is precisely what the boundary is for), and the boundary's own `render()` has a `return (`
    // before the component — so an unanchored search finds the wrong one and the assertion inverts.
    let component_at = src.find("export default function").expect("the component");
    let js_at = src.find("const fmt").expect("js block");
    let return_at = component_at + src[component_at..].find("return (").expect("return");
    let raw_at = src.find("<SomeChart").expect("raw block");
    assert!(js_at < return_at, "js block must be hoisted above the return\n{src}");
    assert!(raw_at > return_at, "raw block must stay in the tree\n{src}");
}

#[test]
fn a_raw_block_for_another_backend_is_skipped_not_an_error() {
    // A document can carry blocks for several targets. Emitting a Svelte block into React would be
    // a syntax error in the output; failing the build would make multi-target documents impossible.
    let (program, diags) = check("page P\nraw svelte\n  {#if x}<p>y</p>{/if}\n");
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let src = &guml_codegen::react::ReactBackend.emit(&program).files[0].contents;
    assert!(!src.contains("{#if x}"), "a svelte block must not land in a React file\n{src}");
}

#[test]
fn the_json_backend_never_ships_the_code() {
    let (program, _) = check(DOC);
    let json = &guml_codegen::json::JsonBackend.emit(&program).files[0].contents;

    // The load-bearing assertion of this file. The browser runtime consumes this tree.
    assert!(!json.contains("toFixed"), "js body reached the render tree\n{json}");
    assert!(!json.contains("SomeChart"), "raw body reached the render tree\n{json}");

    // A placeholder is still emitted, because invariant 3 says never silently drop: the preview
    // shows a gap where the block is rather than pretending the document is complete.
    assert!(json.contains("js-placeholder"), "{json}");
    assert!(json.contains("raw-placeholder"), "{json}");
    assert!(json.contains("not run in the preview"), "{json}");
}

#[test]
fn a_reference_from_inside_a_block_counts_as_a_use() {
    // A body is another language, so a use need not look like a `{binding}`. Reporting `month` as
    // dead here would be wrong on its own terms, and actively dangerous once anything elides dead
    // declarations — the emitted code would stop compiling.
    let src = "page P\nstate month=all|q1\njs\n  const isQ1 = month === \"q1\";\n";
    let (_, diags) = check(src);
    assert!(
        !diags.items.iter().any(|d| d.id == "GUML0074"),
        "`month` is used by the block: {:?}",
        diags.items
    );

    // The control: with the block gone, it really is unused.
    let (_, diags) = check("page P\nstate month=all|q1\ncard Hi\n");
    assert!(diags.items.iter().any(|d| d.id == "GUML0074"), "{:?}", diags.items);
}

#[test]
fn a_body_is_not_lexed_as_guml() {
    // Every one of these is an error, a dropped line or a rewrite if the body is treated as GUML:
    // `}` is `GUML0004`, `//` is a comment (silently deleted), a tab is `GUML0001`.
    let src = "page P\njs\n  // keep me\n  if (a) { b(); }\n\tconst t = 1;\n";
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "body reported as GUML: {:?}", diags.items);

    let lines = &program.tree[0].text_lines;
    assert!(lines.iter().any(|l| l.contains("// keep me")), "comment dropped: {lines:?}");
    assert!(lines.iter().any(|l| l.contains("if (a) { b(); }")), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("const t = 1;")), "tab-indented line lost: {lines:?}");

    // And the emitted code still contains them, which is the point of the hatch.
    let out = &guml_codegen::react::ReactBackend.emit(&program).files[0].contents;
    assert!(out.contains("// keep me") && out.contains("if (a) { b(); }"), "{out}");
}

#[test]
fn a_block_ends_at_the_first_line_that_dedents() {
    // A blank line inside a body must not end the block, but a real dedent must — otherwise the
    // rest of the document would be swallowed into unchecked code.
    let src = "page P\njs\n  const a = 1;\n\n  const b = 2;\ncrad typo\n";
    let (program, diags) = check(src);
    assert_eq!(program.tree[0].text_lines.len(), 2, "blank line ended the block early");
    assert!(
        diags.items.iter().any(|d| d.id == "GUML0010" || d.message.contains("crad")),
        "the line after the block is GUML again and must be checked: {:?}",
        diags.items
    );
}

#[test]
fn nesting_inside_a_block_survives_the_round_trip() {
    // Unlike `tier`/`faq` content — which the parser flattens, so its extra indent means nothing —
    // a block body is another language. Flattening it would change the value of a template
    // literal, and would make the emitted JS unreadable besides.
    let src = "page P\njs\n    const f = () => {\n      return `a\n        b`;\n    }\n";
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let lines = &program.tree[0].text_lines;
    // Measured from the *body's* first line, so a body indented by 4 still starts flush.
    assert_eq!(lines[0], "const f = () => {");
    assert_eq!(lines[1], "  return `a");
    assert_eq!(lines[2], "    b`;");

    let out = guml_fmt::format(src, guml_fmt::Options::default()).text;
    assert!(out.contains("\n  const f"), "body should start at one level:\n{out}");
    assert!(out.contains("\n    return `a"), "relative nesting lost:\n{out}");
    assert!(out.contains("\n      b`;"), "template literal indent lost:\n{out}");
    assert_eq!(
        guml_fmt::format(&out, guml_fmt::Options::default()).text,
        out,
        "not idempotent over a nested block"
    );
}

#[test]
fn the_formatter_leaves_block_contents_alone() {
    // The formatter re-indents structure. Inside a hatch it must not, because the contents are not
    // GUML and its idea of canonical indentation is meaningless there.
    let src = "page P\njs\n      const x   =   1;\n        if (x) {}\n";
    let out = guml_fmt::format(src, guml_fmt::Options::default()).text;
    assert!(out.contains("const x   =   1;"), "formatter rewrote block contents:\n{out}");
    let twice = guml_fmt::format(&out, guml_fmt::Options::default()).text;
    assert_eq!(twice, out, "not idempotent over a block");
}
