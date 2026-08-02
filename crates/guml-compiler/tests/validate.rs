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

/// The React output, for the cases where a *type* rule and its *lowering* must agree. Keeping them in
/// one test is the point: they were separately consistent and jointly wrong before.
fn react(src: &str) -> String {
    use guml_codegen::Backend as _;
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    guml_codegen::react::ReactBackend.emit(&program).files[0].contents.clone()
}

/// Diagnostic *messages*, where the wording is the thing being tested — a code alone does not tell an
/// author which two fields collided.
fn messages(src: &str) -> Vec<String> {
    check(src).1.items.iter().map(|d| d.message.clone()).collect()
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

/// A comparison against a value an enumerated state can never hold.
///
/// Assignment was already checked (`>filter="opne"`). Comparison is the more dangerous half: it is not
/// a type error, it is **dead code** — the branch never runs, the page silently renders the wrong
/// thing, and nothing else in the pipeline has an opinion. A closed set of values is only worth having
/// if the compiler holds comparisons to it.
#[test]
fn a_comparison_outside_an_enumerated_domain_is_an_error() {
    let cases = [
        // In a binding positional.
        "page P\nstate filter=all|open|done\nmetric {filter == \"opne\"}\n",
        // In prose.
        "page P\nstate filter=all|open|done\np Showing {filter == \"opne\"} items\n",
        // In an attribute.
        "page P\nstate filter=all|open|done\ncard hidden={filter != \"opne\"}\n",
        // Reversed operands.
        "page P\nstate filter=all|open|done\nmetric {\"opne\" == filter}\n",
    ];
    for src in cases {
        let (_, diags) = guml_compiler::check(src);
        assert!(
            diags.items.iter().any(|d| d.id == "GUML0080" && d.message.contains("never equal")),
            "not caught: {src:?} → {:?}",
            diags.items
        );
    }
}

#[test]
fn a_comparison_inside_the_domain_is_silent() {
    for value in ["all", "open", "done"] {
        let src = format!("page P\nstate filter=all|open|done\nmetric {{filter == \"{value}\"}}\n");
        let (_, diags) = guml_compiler::check(&src);
        assert!(
            !diags.items.iter().any(|d| d.id == "GUML0080"),
            "`{value}` is in the domain but was rejected: {:?}",
            diags.items
        );
    }
    // And a state with no domain is not subject to the check at all.
    let (_, diags) =
        guml_compiler::check("page P\nstate draft=\"\"\nmetric {draft == \"anything\"}\n");
    assert!(!diags.items.iter().any(|d| d.id == "GUML0080"), "{:?}", diags.items);
}

/// `.open`/`.done` name the *state*, not a field called `done`.
///
/// This block exists because the original rule was "the row type must have a field named `done`", and
/// the lowering was a hardcoded `!it.done`. Both agreed with each other and both were wrong: a Phase 0
/// example modelling invoices with `paid:bool` passed the check and compiled to
/// `invoices.filter((it) => !it.done).length` — always zero, no diagnostic. Nothing tested a boolean
/// field with any other name, which is why two layers could be consistently wrong for months.
mod open_and_done_resolve_the_state_field {
    use super::*;

    const INVOICES: &str = "page P\ntype Invoice {id, amount:number, paid:bool}\n\
                            data invoices:Invoice[] GET /api/invoices\n";

    #[test]
    fn a_boolean_field_of_any_name_is_the_state() {
        let src = format!("{INVOICES}head {{invoices.open.count}} awaiting");
        assert_eq!(codes(&src), Vec::<String>::new());
    }

    #[test]
    fn and_the_backend_filters_on_that_field_not_on_done() {
        let src = format!("{INVOICES}head {{invoices.open.count}} awaiting");
        let js = react(&src);
        assert!(js.contains("!it.paid"), "should filter on the declared field, got: {js}");
        assert!(!js.contains("it.done"), "must not invent a `done` field: {js}");
    }

    #[test]
    fn no_boolean_field_means_there_is_no_state_to_filter() {
        let src = "page P\ntype Row {id, amount:number}\ndata rows:Row[] GET /api/rows\n\
                   head {rows.open.count}";
        assert_eq!(codes(src), vec!["GUML0065"]);
        assert!(messages(src)[0].contains("no boolean field"), "{:?}", messages(src));
    }

    #[test]
    fn two_boolean_fields_are_ambiguous_rather_than_a_coin_flip() {
        let src = "page P\ntype Invoice {id, paid:bool, overdue:bool}\n\
                   data invoices:Invoice[] GET /api/invoices\nhead {invoices.open.count}";
        assert_eq!(codes(src), vec!["GUML0065"]);
        let msg = &messages(src)[0];
        // Both names are listed, sorted, because the author has to be told which two collided.
        assert!(msg.contains("overdue, paid"), "{msg}");
        assert!(msg.contains("ambiguous"), "{msg}");
    }

    #[test]
    fn done_still_works_and_is_not_special_cased() {
        let src = "page P\ntype Task {id, done:bool}\ndata tasks:Task[] GET /api/tasks\n\
                   head {tasks.done.count} of {tasks.count}";
        assert_eq!(codes(src), Vec::<String>::new());
        assert!(react(src).contains("it.done"));
    }
}

mod an_escape_hatch_declares_names_the_document_can_read {
    use super::*;

    /// The case that forced it: summing a *computed* per-row value. An aggregate applies to a field, not
    /// to an expression, so `Σ unitPrice × quantity` — a cart subtotal — has no binding form. The spec's
    /// answer is "drop into `js` and let it be counted", and that did not work either: the block could
    /// compute the value and nothing could read it.
    const CART: &str = "page P\n\
                        type Line {id, product, unitPrice:number, quantity:number}\n\
                        data cart:Line[] GET /api/cart\n\
                        js\n\
                        \x20 const subtotal = cart.reduce((a, l) => a + l.unitPrice * l.quantity, 0);\n\
                        card \"Totals\"\n\
                        \x20 metric {subtotal}\n";

    #[test]
    fn a_js_const_is_in_scope_for_a_binding() {
        // `GUML0090` for the hatch itself is expected and is the point — the escape is *counted*, not
        // waved through. What must not be there is `GUML0033`.
        assert_eq!(codes(CART), vec!["GUML0090"]);
        let out = react(CART);
        assert!(out.contains("const subtotal = cart.reduce"), "{out}");
        assert!(out.contains(">{subtotal}<"), "the binding did not read the js value: {out}");
    }

    #[test]
    fn a_name_the_js_block_does_not_declare_is_still_undeclared() {
        // The rule extends scope; it does not disable the check. A typo in the binding is still an error,
        // which is what stops this from being "any identifier is fine if the document contains any `js`".
        let src = CART.replace("{subtotal}", "{subtotl}");
        assert!(codes(&src).contains(&"GUML0033".to_string()), "{:?}", codes(&src));
    }

    #[test]
    fn only_a_top_level_declaration_escapes_the_block() {
        // Conservative on purpose: a `const` inside a function body is not in the component's scope, and
        // putting it there would let a binding read a name that does not exist at that point.
        let src = "page P\n\
                   js\n\
                   \x20 function total() {\n\
                   \x20   const inner = 1;\n\
                   \x20   return inner;\n\
                   \x20 }\n\
                   metric {inner}\n";
        assert!(codes(src).contains(&"GUML0033".to_string()), "{:?}", codes(src));

        // The function itself *is* top level, so calling it is fine.
        let ok = src.replace("metric {inner}", "metric {total()}");
        assert_eq!(codes(&ok), vec!["GUML0090"]);
    }

    #[test]
    fn a_raw_block_declares_nothing() {
        // A `raw` body is markup for one backend and every other backend drops it, so a binding that
        // depended on a name from it would compile in one target and be undefined in the rest.
        let src = "page P\nraw react\n  const x = 1;\nmetric {x}\n";
        assert!(codes(src).contains(&"GUML0033".to_string()), "{:?}", codes(src));
    }
}
