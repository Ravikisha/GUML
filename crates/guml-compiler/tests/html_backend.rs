//! The static HTML backend.
//!
//! Two things are being held down here.
//!
//! **The IR claim.** "GUML is an IR with several backends" is a claim about the language, and one
//! backend cannot support it. What makes a second backend evidence rather than a coincidence is that
//! it shares the design-system table with the first: the same GUML produces the same class strings,
//! so the compiler — not the backend — owns presentation. A separate table would have made this file
//! a test of two hand-written string lists agreeing.
//!
//! **Invariant 3, in its hardest form.** This backend emits no JavaScript, so state, actions and
//! fetch are not "not yet" — they are permanently impossible. Every one of them has to be reported
//! and marked in the output. A no-JS backend that emitted a page of buttons which look live and do
//! nothing is the worst possible output for a compiler whose pitch is reliability.

use guml_codegen::Backend as _;
use guml_compiler::check;

fn html(src: &str) -> (String, Vec<String>) {
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let out = guml_codegen::html::HtmlBackend::default().emit(&program);
    let messages = out.diagnostics.items.iter().map(|d| d.message.clone()).collect();
    (out.files[0].contents.clone(), messages)
}

#[test]
fn a_content_page_lowers_completely() {
    // A landing page is the category this backend is *right* for: no runtime needed, so no warning
    // should be produced at all.
    let src = std::fs::read_to_string("../../fixtures/c.guml").expect("c.guml");
    let (out, warnings) = html(&src);

    assert!(out.starts_with("<!doctype html>"), "{}", &out[..60]);
    assert!(out.contains("<title>Landing</title>"), "no title");
    // `faq` is interactive without script, so it lowers fully rather than being marked inert.
    assert!(out.contains("<details"), "faq did not become a disclosure element");
    assert!(out.contains("<summary"), "faq entries have no summary");
    assert!(!out.contains("data-guml-inert=\"no runtime\"") || out.contains("<button"), "{out}");
    // Anchors the nav links point at have to exist, exactly as in the React output.
    for id in ["features", "pricing", "faq"] {
        assert!(out.contains(&format!("id=\"{id}\"")), "missing anchor target #{id}");
    }
    // The only warnings a pure-content page may produce are about its call-to-action buttons.
    for w in &warnings {
        assert!(w.contains("has an action"), "unexpected warning on a content page: {w}");
    }
}

#[test]
fn the_design_system_is_shared_with_the_react_backend() {
    // The load-bearing test of the whole backend. Not "the classes look right" — "the classes are
    // the *same table*", which is what makes presentation a compiler concern rather than a per-
    // backend opinion.
    let src = "page P\ncard sm center\n  h Clicks\n  p Press the buttons.\n";
    let (program, _) = check(src);
    let html_out = &guml_codegen::html::HtmlBackend::default().emit(&program).files[0].contents;
    let react_out = &guml_codegen::react::ReactBackend.emit(&program).files[0].contents;

    // Pull every class string out of the React output and require the HTML to carry it too.
    let mut checked = 0;
    for chunk in react_out.split("className=\"").skip(1) {
        let Some(classes) = chunk.split('"').next() else { continue };
        if classes.is_empty() {
            continue;
        }
        checked += 1;
        assert!(
            html_out.contains(classes),
            "the `html` backend does not share the React class string {classes:?}\n{html_out}"
        );
    }
    assert!(checked >= 3, "expected several class strings to compare, got {checked}");
}

#[test]
fn no_javascript_is_emitted_for_a_stateful_document() {
    let src = std::fs::read_to_string("../../fixtures/a.guml").expect("a.guml");
    let (out, _) = html(&src);
    // Not one script tag, not even the Tailwind CDN. The default styling path inlines the theme's
    // stylesheet, so an emitted document has no runtime dependency on anything at all.
    assert_eq!(
        out.matches("<script").count(),
        0,
        "a no-JavaScript backend emitted a script:
{out}"
    );
    assert!(
        out.contains("<style>"),
        "the theme stylesheet was not inlined:
{out}"
    );
    assert!(
        !out.contains("cdn.tailwindcss.com"),
        "the CDN must be opt-in:
{out}"
    );
    for banned in ["onclick", "onClick", "addEventListener", "useState"] {
        assert!(!out.contains(banned), "`{banned}` reached a no-JavaScript backend:\n{out}");
    }
}

#[test]
fn state_and_actions_are_reported_not_dropped() {
    let src = std::fs::read_to_string("../../fixtures/a.guml").expect("a.guml");
    let (out, warnings) = html(&src);

    assert!(
        warnings.iter().any(|w| w.contains("`state` needs a runtime")),
        "state was dropped silently: {warnings:?}"
    );
    // One warning per button that had an action — three in the counter fixture.
    let actions = warnings.iter().filter(|w| w.contains("has an action")).count();
    assert_eq!(actions, 3, "expected a warning per action, got {actions}: {warnings:?}");

    // And the output marks the gap where a reader can see it, not only in the diagnostics.
    assert_eq!(out.matches("data-guml-inert").count(), 3, "{out}");
    assert_eq!(
        out.matches("disabled data-guml-inert").count(),
        3,
        "buttons should be inert:\n{out}"
    );
}

#[test]
fn a_binding_renders_its_initial_value_or_an_em_dash() {
    // `metric {count}` with `state count=0` has a knowable value before any script runs. An
    // expression does not, and leaving `{count + 1}` on screen for a visitor to read would be worse
    // than admitting the gap.
    let (out, warnings) = html("page P\nstate count=7\nmetric {count}\nmetric {count + 1}\n");
    assert!(out.contains(">7</p>"), "initial value not rendered:\n{out}");
    assert!(out.contains(">—</p>"), "an unknowable binding should render an em dash:\n{out}");
    assert!(
        warnings.iter().any(|w| w.contains("no value at build time")),
        "the unknowable binding was not reported: {warnings:?}"
    );
}

#[test]
fn a_repeater_renders_its_empty_state() {
    // There is no data at build time, so the honest render is the state a first-time visitor sees —
    // and the author's own `empty` message, not a generic one.
    let src = "page P\ntype Row {id, title}\ndata rows:Row[] GET /api/rows\nlist rows\n  text {title}\n  empty No rows yet.\n";
    let (out, warnings) = html(src);
    assert!(out.contains("No rows yet."), "the author's empty message was not used:\n{out}");
    assert!(out.contains("data-guml-inert=\"no data at build time\""), "{out}");
    assert!(
        warnings.iter().any(|w| w.contains("`data` needs a runtime")),
        "the resource was dropped silently: {warnings:?}"
    );
}

#[test]
fn prose_is_escaped_even_though_it_was_never_quoted() {
    // GUML prose reaches the backend verbatim — that is why it costs so few tokens, and it means
    // this backend is the first thing in the pipeline that has to escape it. Missing that would be
    // an HTML injection in any document containing a `<`.
    let (out, _) = html("page P\np 5 < 6 & \"quoted\" <img src=x onerror=alert(1)\n");
    assert!(out.contains("5 &lt; 6 &amp; &quot;quoted&quot;"), "{out}");
    assert!(!out.contains("<img"), "prose was not escaped:\n{out}");
    assert!(out.contains("&lt;img"), "{out}");
    assert_eq!(out.matches("<script").count(), 0, "{out}");
}

#[test]
fn a_closing_tag_cannot_be_written_in_prose_at_all() {
    // Not an escaping property but worth pinning, because it is why the obvious injection payload
    // cannot even be authored: `>` takes the rest of the line as an action, so
    // `p <script>alert(1)</script>` parses as an action calling `alert`, and the resolver rejects it
    // as an undeclared reference before any backend sees it.
    let (_, diags) = check("page P\np <script>alert(1)</script>\n");
    assert!(
        diags.items.iter().any(|d| d.id == "GUML0033" && d.message.contains("alert")),
        "expected the action resolver to reject it: {:?}",
        diags.items
    );
}

#[test]
fn an_escape_block_reaches_only_its_own_backend() {
    let src =
        "page P\nraw html\n  <hr class=\"my-4\" />\nraw react\n  <Chart />\njs\n  const x = 1;\n";
    let (out, warnings) = html(src);
    assert!(out.contains("<hr class=\"my-4\" />"), "`raw html` was not emitted:\n{out}");
    assert!(!out.contains("<Chart />"), "a React block leaked into HTML output:\n{out}");
    assert!(!out.contains("const x = 1"), "a `js` block reached a no-JavaScript backend:\n{out}");
    assert!(
        warnings.iter().any(|w| w.contains("`js` block cannot run")),
        "the dropped `js` block was not reported: {warnings:?}"
    );
}

#[test]
fn the_stylesheet_is_inlined_rather_than_fetched() {
    // The production path. A document that depends on a third-party script at render time is not a
    // static artifact — it is an outage waiting for someone else's CDN, on a backend whose entire
    // selling point is needing nothing.
    let (out, _) = html(
        "page P
card Hi
  p body
",
    );
    assert!(out.contains("<style>"), "{out}");
    // The stylesheet implements the classes the document actually uses.
    assert!(out.contains(".rounded-xl"), "{out}");
    assert!(
        out.contains("box-sizing"),
        "no reset in the stylesheet:
{out}"
    );
}

#[test]
fn the_cdn_path_is_a_separate_opt_in_backend() {
    // Kept for previews, named so it cannot be reached by accident.
    use guml_codegen::Backend as _;
    let (program, _) = check(
        "page P
card Hi
  p body
",
    );
    let cdn = guml_codegen::html::HtmlBackend { style: guml_codegen::html::Style::Cdn };
    let out = &cdn.emit(&program).files[0].contents;
    assert!(out.contains("cdn.tailwindcss.com"), "{out}");
    assert!(
        out.contains("not a production artifact"),
        "the caveat should be in the output:
{out}"
    );
    assert!(
        !out.contains("<style>"),
        "the CDN path should not also inline:
{out}"
    );

    // And it is reachable by name, so `--backend html-cdn` works.
    assert!(guml_compiler::backend_names().contains(&"html-cdn"));
}

#[test]
fn the_host_can_take_over_styling_entirely() {
    // For a host whose own pipeline processes the classes, emitting either a stylesheet or a script
    // would be wrong.
    use guml_codegen::Backend as _;
    let (program, _) = check(
        "page P
card Hi
",
    );
    let bare = guml_codegen::html::HtmlBackend { style: guml_codegen::html::Style::None };
    let out = &bare.emit(&program).files[0].contents;
    assert!(!out.contains("<style>"), "{out}");
    assert_eq!(out.matches("<script").count(), 0, "{out}");
    // The classes are still there for the host to process.
    assert!(out.contains("rounded-xl"), "{out}");
}

#[test]
fn document_metadata_reaches_the_document() {
    // `page Name` names a component. A document needs a title, a language and a direction, and
    // without `lang` assistive technology guesses pronunciation.
    let (out, _) = html(
        "page Docs title=\"Read the docs\" description=\"How it works.\" lang=en-GB dir=rtl
p Body.
",
    );
    assert!(out.contains(r#"<html lang="en-GB" dir="rtl">"#), "{out}");
    assert!(out.contains("<title>Read the docs</title>"), "{out}");
    assert!(out.contains(r#"<meta name="description" content="How it works." />"#), "{out}");
}

#[test]
fn a_document_without_metadata_still_declares_a_language() {
    // The default has to be a real value: `<html>` with no `lang` is the single most common
    // accessibility failure in generated markup.
    let (out, _) = html(
        "page P
p Body.
",
    );
    assert!(out.contains(r#"<html lang="en">"#), "{out}");
    assert!(
        out.contains("<title>P</title>"),
        "the page name is the fallback title:
{out}"
    );
}
