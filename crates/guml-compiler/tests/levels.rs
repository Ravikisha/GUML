//! Conformance levels: `core` is markup, `app` adds behaviour.
//!
//! GUML is one language with two levels, the way CommonMark and GFM are one language with two levels.
//! The split is what makes the language embeddable:
//!
//! * A **core** document has no I/O, no state and no behaviour, so a host can render one that arrived
//!   from an untrusted agent — there is nothing in it to run.
//! * An **app** document declares network requests and mutations on the host's behalf, which is a
//!   categorically different thing to accept from a stranger.
//!
//! The level is carried by the *registry*, not by a separate flag, so a host that asked for markup
//! cannot accidentally get behaviour because one call site forgot to thread an option through.
//!
//! Every rejection here is an **error**, not a filter. A host that receives an app-level document and
//! asked for core has to be told, rather than handed a page with the fetch silently removed —
//! invariant 3 applies to levels exactly as it applies to codegen.

use guml_compiler::check_with;
use guml_registry::{Level, Registry};

fn core_errors(src: &str) -> Vec<String> {
    let (_, diags) = check_with(src, &Registry::core());
    diags.items.iter().filter(|d| d.id == "GUML0091").map(|d| d.message.clone()).collect()
}

#[test]
fn a_pure_markup_document_compiles_at_the_core_level() {
    // The landing fixture is real, published markup: sections, prose, pricing tiers, an FAQ. If the
    // core level could not compile this, the level would be useless.
    let src = std::fs::read_to_string("../../fixtures/c.guml").expect("c.guml");
    let (_, diags) = check_with(&src, &Registry::core());
    assert!(
        !diags.has_errors(),
        "a content page must be valid core markup: {:?}",
        diags
            .items
            .iter()
            .filter(|d| d.severity == guml_diagnostics::Severity::Error)
            .collect::<Vec<_>>()
    );
}

#[test]
fn state_and_data_are_rejected_at_the_core_level() {
    let src = std::fs::read_to_string("../../fixtures/b.guml").expect("b.guml");
    let messages = core_errors(&src);
    assert!(
        messages.iter().any(|m| m.contains("`data`")),
        "a resource declares a network request and must be rejected: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("`state`")),
        "mutable state needs a runtime: {messages:?}"
    );
}

#[test]
fn an_action_is_rejected_at_the_core_level() {
    // `>` runs code on an event, which is the definition of behaviour.
    let messages = core_errors("page P\ncard Hi\n  btn Go primary >count++\n");
    assert!(messages.iter().any(|m| m.contains("an action")), "{messages:?}");
}

#[test]
fn a_js_block_is_rejected_at_the_core_level() {
    // The escape hatch is arbitrary code by construction, so it cannot exist in a level whose whole
    // promise is that there is nothing to run.
    let messages = core_errors("page P\njs\n  fetch(\"/steal\");\ncard Hi\n");
    assert!(messages.iter().any(|m| m.contains("`js`")), "{messages:?}");
}

#[test]
fn raw_markup_is_still_allowed_at_the_core_level() {
    // `raw html` is markup, not behaviour — the same call a Markdown renderer makes when it decides
    // inline HTML is in scope. It is still reported as an escape hatch so the rate stays measurable.
    let src = "page P\nraw html\n  <hr class=\"my-4\" />\n";
    let (_, diags) = check_with(src, &Registry::core());
    assert!(!diags.has_errors(), "`raw` is markup: {:?}", diags.items);
    assert!(
        diags.items.iter().any(|d| d.id == "GUML0090"),
        "an escape hatch should still be counted: {:?}",
        diags.items
    );
}

#[test]
fn a_repeater_is_app_level_because_it_iterates_a_resource() {
    // `list` has nothing to iterate without `data`, so it belongs to the app level. It fails as an
    // unknown tag, because at the core level it genuinely is not in the vocabulary — enforcement by
    // absence rather than by a downstream check somebody could forget.
    let (_, diags) = check_with("page P\nlist rows\n  text {id}\n", &Registry::core());
    assert!(
        diags.items.iter().any(|d| d.id == "GUML0030" && d.message.contains("list")),
        "{:?}",
        diags.items
    );
    // And it is perfectly fine one level up.
    assert!(Registry::builtin().get("list").is_some());
}

#[test]
fn the_rejection_is_an_error_rather_than_a_silent_filter() {
    // The failure this guards against is the tempting one: strip the `data` line and compile the rest,
    // handing the host a page that looks complete and fetches nothing.
    let (program, diags) = check_with("page P\nstate count=0\nmetric {count}\n", &Registry::core());
    assert!(diags.has_errors(), "the document must not compile");
    // The declaration is still parsed, so diagnostics downstream stay coherent rather than cascading
    // into "unknown reference `count`".
    assert_eq!(program.states.len(), 1, "the state should still be in the AST");
    assert!(
        !diags.items.iter().any(|d| d.id == "GUML0033"),
        "rejecting the level must not also produce a phantom unresolved reference: {:?}",
        diags.items
    );
}

#[test]
fn the_app_level_is_the_default() {
    // Nothing changes for an existing document: `check` without a registry compiles at the app level,
    // so the split is additive rather than a breaking change.
    let src = std::fs::read_to_string("../../fixtures/b.guml").expect("b.guml");
    let (_, diags) = guml_compiler::check(&src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    assert_eq!(Registry::builtin().level(), Level::App);
    assert_eq!(Registry::default().level(), Level::App);
}
