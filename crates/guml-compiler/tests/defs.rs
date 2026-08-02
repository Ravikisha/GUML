//! User-defined components: `def`.
//!
//! A `def` is a **compile-time macro**, and the tests below are mostly about the consequences of that
//! choice rather than about substitution mechanics:
//!
//! * Expansion happens before every other pass, so nothing downstream knows `def` exists. That is what
//!   makes a `def` work in the no-JavaScript HTML backend for free, and it is why the resolver and the
//!   accessibility lint apply to expanded markup without a line of new code.
//! * A def's conformance **level is inherited from its body**. There is no `level` on a `def`, because
//!   there is nothing to declare: a body containing an action is app-level by virtue of containing one.
//! * Everything expansion cannot do is an **error**, not a silent omission — children with no slot, a
//!   parameter in an action, a cycle, wrong arity. A macro that quietly drops what it cannot handle is
//!   the worst possible shape for this feature.

use guml_codegen::Backend as _;
use guml_compiler::check;
use guml_registry::Registry;

fn errors(src: &str) -> Vec<String> {
    let (_, d) = check(src);
    d.items
        .iter()
        .filter(|d| d.severity == guml_diagnostics::Severity::Error)
        .map(|d| format!("{} {}", d.id, d.message))
        .collect()
}

const STAT: &str = r#"page Dash

state total=0

def kpi label value
  card sm center
    h {label}
    metric {value}

kpi "Revenue" {total}
kpi "Signups" {total}
"#;

#[test]
fn a_call_expands_to_the_body_with_arguments_substituted() {
    let (program, diags) = check(STAT);
    assert!(!diags.has_errors(), "{:?}", diags.items);

    // Two calls, two expansions, and no trace of the `def` in the tree.
    assert_eq!(program.tree.len(), 2, "expected one card per call: {:?}", program.tree);
    assert!(program.tree.iter().all(|el| el.tag == "card"), "{:?}", program.tree);

    // A string argument becomes text; a binding argument stays a binding.
    let first = &program.tree[0];
    let heading = &first.children[0];
    assert_eq!(heading.content.as_deref(), Some("Revenue"), "a literal should substitute as text");
    let metric = &first.children[1];
    assert_eq!(metric.content.as_deref(), Some("{total}"), "a binding should stay a binding");

    assert_eq!(program.tree[1].children[0].content.as_deref(), Some("Signups"));
}

#[test]
fn the_emitted_output_is_the_same_as_writing_the_body_inline() {
    // The property that makes this a macro rather than a component: the output carries no evidence
    // that a `def` was involved.
    let (with_def, _) = check(STAT);
    let inline = r#"page Dash

state total=0

card sm center
  h Revenue
  metric {total}

card sm center
  h Signups
  metric {total}
"#;
    let (without, _) = check(inline);

    let emit =
        |p: &guml_ast::Program| guml_codegen::react::ReactBackend.emit(p).files[0].contents.clone();
    assert_eq!(
        emit(&with_def),
        emit(&without),
        "expansion should be indistinguishable from inlining"
    );
}

#[test]
fn a_def_works_in_every_backend_including_the_no_javascript_one() {
    // Nothing in `guml-codegen` knows what a `def` is, which is the whole point of expanding early.
    let (program, _) = check(STAT);
    let html = &guml_codegen::html::HtmlBackend::default().emit(&program).files[0].contents;
    assert!(html.contains("Revenue"), "{html}");
    assert!(html.contains("Signups"), "{html}");
    // As an element name specifically: a bare substring search matches `static` in the inlined
    // stylesheet, which says nothing about the tree.
    assert!(!html.contains("<stat"), "the def name should not survive into output:\n{html}");
}

#[test]
fn a_parameter_substitutes_into_prose_and_into_attributes() {
    let src = r#"page P
state count=0
def tile label value
  card
    p Total {value} for {label}
    input draft aria={label}
state draft=""
tile "Q3" {count}
"#;
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let card = &program.tree[0];
    assert_eq!(card.children[0].content.as_deref(), Some("Total {count} for Q3"));
    let aria = card.children[1].attrs.iter().find(|a| a.name == "aria").expect("aria");
    assert_eq!(aria.value.as_text(), Some("Q3"), "a literal argument becomes a literal attribute");
}

#[test]
fn a_binding_that_is_not_a_parameter_is_left_alone() {
    // Substitution must not capture the surrounding document's bindings, or a def body could not refer
    // to page state at all.
    let src = r#"page P
state count=0
state other=0
def tile label
  card
    h {label}
    metric {count}
tile "Hits"
"#;
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let card = &program.tree[0];
    assert_eq!(card.children[0].content.as_deref(), Some("Hits"));
    assert_eq!(card.children[1].content.as_deref(), Some("{count}"), "page state should survive");
}

#[test]
fn wrong_arity_is_an_error_naming_the_signature() {
    let e = errors("page P\ndef box a\n  card {a}\nbox\n");
    assert!(e.iter().any(|m| m.starts_with("GUML0094") && m.contains("takes 1 argument")), "{e:?}");
    let e = errors("page P\ndef box a\n  card {a}\nbox \"x\" \"y\"\n");
    assert!(e.iter().any(|m| m.contains("2 were given")), "{e:?}");
}

#[test]
fn a_def_may_not_shadow_a_builtin_or_another_def() {
    let e = errors("page P\ndef card x\n  p {x}\n");
    assert!(e.iter().any(|m| m.starts_with("GUML0093")), "shadowing a builtin: {e:?}");

    let e = errors("page P\ndef box a\n  card {a}\ndef box b\n  card {b}\n");
    assert!(e.iter().any(|m| m.contains("defined more than once")), "{e:?}");
}

#[test]
fn a_cycle_is_reported_with_its_path_and_does_not_hang() {
    // Direct.
    let e = errors("page P\ndef loop x\n  card\n    loop {x}\nloop \"a\"\n");
    assert!(e.iter().any(|m| m.starts_with("GUML0095")), "{e:?}");

    // Mutual, which a naive depth check would miss.
    let e =
        errors("page P\ndef a x\n  card {x}\n    b {x}\ndef b y\n  card {y}\n    a {y}\na \"z\"\n");
    let cycle = e.iter().find(|m| m.starts_with("GUML0095")).expect("a cycle should be reported");
    assert!(cycle.contains("a → b → a") || cycle.contains("b → a → b"), "{cycle}");
}

#[test]
fn an_empty_def_is_an_error_rather_than_a_call_that_vanishes() {
    let e = errors("page P\ndef nope x\n");
    assert!(e.iter().any(|m| m.starts_with("GUML0096")), "{e:?}");
}

#[test]
fn a_parameter_in_an_action_is_rejected_rather_than_guessed() {
    // Substituting here means deciding whether the argument is a variable reference or a literal, and
    // the call site does not answer that. Guessing would produce JavaScript that compiles and does the
    // wrong thing.
    let e = errors(
        "page P\nstate count=0\ndef stepper amount\n  btn Up primary >count=count+amount\nstepper 1\n",
    );
    assert!(e.iter().any(|m| m.starts_with("GUML0097")), "{e:?}");
}

#[test]
fn a_slot_receives_the_call_children() {
    // What lets a `def` *wrap* content rather than only produce it.
    let src = "page P\ndef panel title\n  card {title}\n    slot\npanel \"Settings\"\n  p First.\n  p Second.\n";
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);

    let card = &program.tree[0];
    assert_eq!(card.tag, "card");
    // The slot element itself is gone, replaced by the children in order.
    assert_eq!(card.children.len(), 2, "{:?}", card.children);
    assert_eq!(card.children[0].content.as_deref(), Some("First."));
    assert_eq!(card.children[1].content.as_deref(), Some("Second."));
    assert!(!format!("{:?}", program.tree).contains("\"slot\""), "a slot survived expansion");
}

#[test]
fn children_are_resolved_in_the_callers_scope_not_the_defs() {
    // Macro hygiene. A binding in the children must mean what it meant where the call was written; if a
    // slot captured the def's parameters, `{title}` below would silently become the def's `title`.
    let src = "page P\nstate title=\"page\"\ndef panel title\n  card {title}\n    slot\npanel \"def scope\"\n  metric {title}\n";
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let card = &program.tree[0];
    // The def's own use of `title` became the argument…
    assert_eq!(card.positionals.len(), 1);
    // …and the child's use of `title` is still the page state.
    assert_eq!(card.children[0].content.as_deref(), Some("{title}"));
}

#[test]
fn children_with_no_slot_are_an_error() {
    // Dropping them silently is the failure invariant 3 exists to prevent.
    let e = errors("page P\ndef box a\n  card {a}\nbox \"x\"\n  p inner\n");
    assert!(e.iter().any(|m| m.starts_with("GUML0097") && m.contains("no `slot`")), "{e:?}");
}

#[test]
fn a_body_may_have_at_most_one_slot() {
    // A second slot would duplicate the children, which is far likelier a mistake than an intention.
    let e = errors("page P\ndef box a\n  card {a}\n    slot\n    slot\nbox \"x\"\n  p y\n");
    assert!(e.iter().any(|m| m.contains("at most one")), "{e:?}");
}

#[test]
fn a_slot_with_no_children_is_a_warning() {
    // Not an error: a wrapper called without content still renders. But it is almost certainly a
    // mistake, and it costs nothing to say so.
    let (_, diags) = check("page P\ndef box a\n  card {a}\n    slot\nbox \"x\"\n");
    assert!(
        diags
            .items
            .iter()
            .any(|d| d.id == "GUML0097" && d.message.contains("supplies no children")),
        "{:?}",
        diags.items
    );
    assert!(!diags.has_errors(), "an empty slot should not fail the build");
}

#[test]
fn an_unused_parameter_is_a_warning() {
    // Same reasoning as `GUML0074` for an unused `state`: free to notice, almost always a mistake.
    let (_, diags) = check("page P\ndef box a b\n  card {a}\nbox \"x\" \"y\"\n");
    assert!(
        diags.items.iter().any(|d| d.id == "GUML0097" && d.message.contains("never uses")),
        "{:?}",
        diags.items
    );
    assert!(!diags.has_errors(), "an unused parameter should not fail the build");
}

#[test]
fn a_def_inherits_its_conformance_level_from_its_body() {
    // There is no `level` on a `def` because there is nothing to declare. A body of markup is core; a
    // body containing an action is app, by virtue of containing one.
    let markup = "page P\ndef panel title\n  card {title}\n    p Body.\npanel \"Hi\"\n";
    let (_, diags) = guml_compiler::check_with(markup, &Registry::core());
    assert!(!diags.has_errors(), "a markup-only def is core: {:?}", diags.items);

    let behaviour = "page P\ndef clicker label\n  btn {label} primary >count++\nclicker \"Go\"\n";
    let (_, diags) = guml_compiler::check_with(behaviour, &Registry::core());
    assert!(
        diags.items.iter().any(|d| d.id == "GUML0091"),
        "a def containing an action needs the app level: {:?}",
        diags.items
    );
}

#[test]
fn a_def_may_call_another_def() {
    let src = r#"page P
def label text
  h {text}
def panel title
  card
    label {title}
    p Body.
panel "Hello"
"#;
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    // Both levels expanded: the card contains an `h`, not a `label`.
    let card = &program.tree[0];
    assert_eq!(card.tag, "card");
    assert_eq!(card.children[0].tag, "h", "the inner def should also be expanded");
    assert_eq!(card.children[0].content.as_deref(), Some("Hello"));
}

#[test]
fn downstream_passes_see_expanded_markup() {
    // The accessibility lint knows nothing about `def`, so this proves expansion happens first: an
    // input labelled only by its placeholder is `GUML0051` whether it was written inline or produced
    // by a macro. A warning rather than an error, because a placeholder is at least *something* — the
    // hard error is reserved for a control with no name at all.
    let (_, diags) = check(
        "page P\nstate draft=\"\"\ndef field hint\n  input draft placeholder={hint}\nfield \"Type here\"\n",
    );
    assert!(
        diags.items.iter().any(|d| d.id == "GUML0051"),
        "the accessibility lint should apply to expanded markup: {:?}",
        diags.items
    );

    // And the resolver too: a binding inside a def body that names nothing is still `GUML0033`.
    let e = errors("page P\ndef tile x\n  card {x}\n    metric {nosuchthing}\ntile \"a\"\n");
    assert!(e.iter().any(|m| m.starts_with("GUML0033")), "{e:?}");
}
