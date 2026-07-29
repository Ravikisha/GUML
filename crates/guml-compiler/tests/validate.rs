//! Static validator tests.
//!
//! Every check gets two cases: one document that should trip it, and one that should not.
//! The second half is the half that matters — a validator that fires on working documents
//! is worse than no validator, because in a generation loop a false error costs a whole
//! model round to "fix" something that was already right.

use guml_compiler::check;
use guml_diagnostics::Severity;

fn codes(src: &str) -> Vec<String> {
    check(src).1.items.iter().map(|d| d.id.to_string()).collect()
}

fn errors(src: &str) -> Vec<String> {
    check(src)
        .1
        .items
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.id.to_string())
        .collect()
}

fn assert_fires(code: &str, src: &str) {
    let found = codes(src);
    assert!(found.contains(&code.to_string()), "expected {code}, got {found:?}\n{src}");
}

fn assert_silent(code: &str, src: &str) {
    let found = codes(src);
    assert!(!found.contains(&code.to_string()), "unexpected {code} in\n{src}\ngot {found:?}");
}

/* ------------------------------------------------- references and types (0061+) */

#[test]
fn unknown_mutation_is_an_error_with_a_suggestion() {
    let src = "page P\ntype T {id, done:bool}\ndata rows:T[] GET /api/rows\n  save PATCH /api/rows/{id} {done}\n\nlist rows\n  text {id}\n  check {done} aria=\"done\" >rows.sve\n";
    assert_fires("GUML0061", src);
    let d = check(src).1.items.into_iter().find(|d| d.id == "GUML0061").unwrap();
    // In `help`, not `suggestion`: the span is the whole element, and `suggestion` is a
    // machine-applicable replacement *for the span*. See `fix::spans_one_token`.
    assert!(d.help.unwrap_or_default().contains("save"), "names the real mutation");
    assert!(d.suggestion.is_none(), "not machine-applicable against a line span");
}

#[test]
fn a_declared_mutation_is_accepted() {
    assert_silent(
        "GUML0061",
        "page P\ntype T {id, done:bool}\ndata rows:T[] GET /api/rows\n  save PATCH /api/rows/{id} {done}\n\nlist rows\n  text {id}\n  check {done} aria=\"d\" >rows.save\n",
    );
}

#[test]
fn an_undeclared_resource_type_is_an_error() {
    assert_fires("GUML0062", "page P\ndata rows:Row[] GET /api/rows\n\nlist rows\n  text {id}\n");
    assert_silent(
        "GUML0062",
        "page P\ntype Row {id}\ndata rows:Row[] GET /api/rows\n\nlist rows\n  text {id}\n",
    );
}

#[test]
fn an_optimistic_mutation_body_must_exist_on_the_type() {
    let src = "page P\ntype T {id, title}\ndata rows:T[] GET /api/rows\n  add POST /api/rows {titel} optimistic:prepend\n\nlist rows\n  text {title}\n";
    assert_fires("GUML0063", src);
    let d = check(src).1.items.into_iter().find(|d| d.id == "GUML0063").unwrap();
    assert!(d.help.unwrap_or_default().contains("title"));
}

#[test]
fn a_plain_request_body_may_carry_fields_the_resource_lacks() {
    // A login sends a password. It is not a field of `Session`, and demanding that it be one
    // rejects a correct document — which is exactly what the first version of this rule did
    // to `bench/phase0/examples/e2-signin.guml`, one of the examples the model learns from.
    assert_silent(
        "GUML0063",
        "page P\ntype Session {token, email}\ndata session:Session[] GET /api/session\n  login POST /api/session {email,password}\n\nlist session\n  text {email}\n",
    );
}

#[test]
fn assignment_targets_must_be_plain_state_names() {
    // The boundary that keeps actions non-Turing-complete, and therefore the security
    // boundary for rendering an untrusted agent's document.
    // A dotted target whose head is a *known* state: the resolver cannot catch this one,
    // because `n` itself resolves. Only the assignment is illegal.
    assert_fires("GUML0064", "page P\nstate n=0\n\nbtn Go >n.length=3\n");
    assert_fires(
        "GUML0064",
        "page P\ntype T {id}\ndata rows:T[] GET /api/rows\n\nbtn Clear >rows=0\n\nlist rows\n  text {id}\n",
    );
    assert_silent("GUML0064", "page P\nstate n=0\n\nbtn Go primary >n++\n");
}

#[test]
fn an_assignment_must_match_the_states_type() {
    assert_fires("GUML0065", "page P\nstate n=0\n\nbtn Go >n=\"text\"\n");
    assert_fires("GUML0065", "page P\nstate name=\"\"\n\nbtn Go >name=3\n");
    // An enumerated state may only take a member of its domain.
    assert_fires("GUML0065", "page P\nstate filter=all|open\n\nbtn Go >filter=closed\n");
    assert_silent("GUML0065", "page P\nstate filter=all|open\n\nbtn Go >filter=open\n");
    assert_silent("GUML0065", "page P\nstate n=0\n\nbtn Go >n=42\n");
    assert_silent("GUML0065", "page P\nstate draft=\"\"\n\nbtn Go >draft=\"\"\n");
}

/* ------------------------------------------------------------- structure (0070+) */

#[test]
fn duplicate_and_dangling_anchors() {
    assert_fires("GUML0070", "page P\n\nsection #a One\n  p x\nsection #a Two\n  p y\n");
    let src = "page P\n\nnav Brand\n  link Pricing #pricing\n\nsection #features Features\n  p x\n";
    assert_fires("GUML0071", src);
    let d = check(src).1.items.into_iter().find(|d| d.id == "GUML0071").unwrap();
    assert!(d.message.contains("pricing"));
}

#[test]
fn several_links_may_point_at_the_same_section() {
    // A nav link and a hero link both targeting `#work` is ordinary page structure. An
    // earlier version counted the *link* as defining the id, so it reported the section as a
    // duplicate and then as dangling — on a page that was completely correct.
    assert_silent(
        "GUML0070",
        "page P

nav Brand
  link Work #work

hero
  h1 Hi
  link \"See the work\" #work

section #work Work
  p x
",
    );
    assert_silent(
        "GUML0071",
        "page P

nav Brand
  link Work #work

hero
  h1 Hi
  link \"See the work\" #work

section #work Work
  p x
",
    );
}

#[test]
fn an_anchor_that_exists_is_accepted() {
    assert_silent(
        "GUML0071",
        "page P\n\nnav Brand\n  link Features #features\n\nsection #features Features\n  p x\n",
    );
}

#[test]
fn a_repeater_without_an_item_template_is_a_warning() {
    assert_fires("GUML0072", "page P\ntype T {id}\ndata rows:T[] GET /api/rows\n\nlist rows\n");
    assert_silent(
        "GUML0072",
        "page P\ntype T {id}\ndata rows:T[] GET /api/rows\n\nlist rows\n  text {id}\n",
    );
}

#[test]
fn a_second_h1_is_a_warning_not_an_error() {
    let src = "page P\n\nh1 One\nh1 Two\n";
    assert_fires("GUML0073", src);
    assert!(!errors(src).contains(&"GUML0073".to_string()), "structure smell, not a hard error");
}

#[test]
fn unused_declarations_are_reported() {
    assert_fires("GUML0074", "page P\nstate unusedThing=0\n\ncard Hi\n  p x\n");
    assert_fires(
        "GUML0075",
        "page P\ntype T {id}\ndata rows:T[] GET /api/rows\n\ncard Hi\n  p x\n",
    );
    // Used through prose interpolation, an action body, and a `where=` — all count.
    assert_silent("GUML0074", "page P\nstate count=0\n\nhead Total {count}\n");
    assert_silent("GUML0074", "page P\nstate draft=\"\"\n\ninput draft aria=\"Draft\"\n");
}

/* ------------------------------------------- domains, attributes, requests (0080+) */

#[test]
fn tabs_and_select_need_an_enumerated_state() {
    assert_fires("GUML0080", "page P\nstate filter=\"\"\n\ntabs filter\n");
    assert_silent("GUML0080", "page P\nstate filter=all|open|done\n\ntabs filter\n");
}

#[test]
fn numeric_attributes_reject_words() {
    assert_fires("GUML0081", "page P\n\nsection Features cols=three\n  p x\n");
    assert_silent("GUML0081", "page P\n\nsection Features cols=3\n  p x\n");
}

#[test]
fn a_repeated_attribute_is_a_warning() {
    assert_fires(
        "GUML0082",
        "page P\nstate d=\"\"\n\ninput d placeholder=\"a\" placeholder=\"b\" aria=\"D\"\n",
    );
}

#[test]
fn bad_methods_and_urls_are_errors() {
    assert_fires(
        "GUML0083",
        "page P\ntype T {id}\ndata rows:T[] FETCH /api/rows\n\nlist rows\n  text {id}\n",
    );
    assert_fires(
        "GUML0084",
        "page P\ntype T {id}\ndata rows:T[] GET api/rows\n\nlist rows\n  text {id}\n",
    );
    assert_silent(
        "GUML0084",
        "page P\ntype T {id}\ndata rows:T[] GET https://example.com/rows\n\nlist rows\n  text {id}\n",
    );
}

/* ------------------------------------------------------------------ no false alarms */

#[test]
fn the_fixtures_validate_without_new_errors() {
    // The published fixtures are the reference documents. If validation flags them, either
    // they are wrong or the validator is — and either way the token figures measured from
    // them stop meaning anything.
    for name in ["a.guml", "b.guml", "c.guml"] {
        let src = std::fs::read_to_string(format!("../../fixtures/{name}")).expect("fixture");
        let errs = errors(&src);
        assert!(errs.is_empty(), "{name} should validate clean, got {errs:?}");
    }
}

#[test]
fn the_phase0_examples_validate_without_new_errors() {
    // These go into the model's context as examples of correct GUML. An example that does
    // not validate teaches the model to write documents the validator rejects.
    for name in ["e1-counter.guml", "e2-signin.guml", "e3-invoices.guml"] {
        let path = format!("../../bench/phase0/examples/{name}");
        let src = std::fs::read_to_string(&path).expect("example");
        let errs = errors(&src);
        assert!(errs.is_empty(), "{name} should validate clean, got {errs:?}");
    }
}

#[test]
fn validation_reports_everything_in_one_pass() {
    // Invariant 1: each repair round is a full generation, so a document with four distinct
    // problems must come back with four diagnostics, not the first one.
    let src =
        "page P\nstate n=0\nstate spare=1\n\nsection #dup A cols=x\n  p y\nsection #dup B\n  p z\n";
    let found = codes(src);
    for code in ["GUML0070", "GUML0081", "GUML0074"] {
        assert!(found.contains(&code.to_string()), "missing {code} in {found:?}");
    }
}

#[test]
fn syntax_outside_the_expression_grammar_is_rejected_not_forwarded() {
    // This used to pass straight through into emitted JavaScript, where a ternary happened to
    // work and a call happened to run. Actions and bindings are deliberately not
    // Turing-complete, and that boundary is also the security boundary for rendering a document
    // an untrusted agent produced — so "not in the grammar" has to mean rejected.
    assert_fires("GUML0023", "page P\nstate a=1\nstate b=2\nstate c=3\n\nhead T {a ? b : c}\n");
    assert_fires("GUML0023", "page P\nstate url=\"\"\n\nbtn Go disabled={fetch(url)}\n");
    // …and the ordinary expressions stay silent.
    for src in [
        "page P\nstate count=0\n\nmetric {count}\n",
        "page P\nstate draft=\"\"\n\nbtn Go disabled={!draft.trim()}\n",
        "page P\ntype T {id, done:bool}\ndata rows:T[] GET /api/rows\n\nhead {rows.open.count} left\n\nlist rows\n  text {id}\n",
    ] {
        assert_silent("GUML0023", src);
    }
}

/// Prose containing `=` stays prose.
///
/// The rule used to be "any `=` on the line means this is structured", so `p Set x=1 to enable the
/// flag.` parsed as the positional `Set`, an attribute `x=1`, and four discarded words. The emitted
/// React was `<p x={1}>Set</p>`: most of the sentence deleted, an invalid DOM prop added, and the
/// build exited ok with one warning about the attribute.
///
/// Prose being taken verbatim is the content-floor claim. A rule that drops words from it is data
/// loss, not compression.
#[test]
fn prose_containing_an_equals_sign_is_not_read_as_an_attribute() {
    let (program, diags) = guml_compiler::check("page P\np Set x=1 to enable the flag.\n");
    let el = &program.tree[0];
    assert_eq!(el.content.as_deref(), Some("Set x=1 to enable the flag."));
    assert!(el.attrs.is_empty(), "prose became an attribute: {:?}", el.attrs);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    assert!(
        !diags.items.iter().any(|d| d.id == "GUML0032"),
        "no attribute was written, so none should be rejected: {:?}",
        diags.items
    );
}

/// The other direction: a text tag's *real* attributes still parse as attributes.
#[test]
fn a_registry_attribute_on_a_text_tag_is_still_structured() {
    // `strike` is a `text` attribute, so this line is structured even though prose would also be a
    // plausible reading. The registry is what distinguishes the two cases.
    let src = "page P\ntype T {id, title, done:bool}\ndata rows:T[] GET /api/r\nlist rows\n  text {title} strike={done}\n";
    let (program, diags) = guml_compiler::check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let text = &program.tree[0].children[0];
    assert_eq!(text.tag, "text");
    assert!(text.attrs.iter().any(|a| a.name == "strike"), "attrs: {:?}", text.attrs);
    assert!(text.content.is_none(), "the line should not have become prose: {:?}", text.content);
}
