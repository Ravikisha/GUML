//! The Web Components backend.
//!
//! Four backends is where "GUML is an IR" stops being arguable. React has hooks, Svelte has runes, static
//! HTML has no runtime at all, and this has manual DOM updates — four genuinely different reactivity
//! models from one AST, sharing one element table, one theme and one expression lowering.
//!
//! What these tests hold down is the part a `.js` file cannot be trusted about from Rust: that the
//! *generated JavaScript is the right JavaScript*. `scripts/check-wc.mjs` runs it; this checks the
//! decisions that produce it, because a test that only asserted "the string contains `class Counter`"
//! would have passed on every bug this backend shipped in its first hour.

use guml_codegen::Backend as _;
use guml_compiler::check;

fn wc(src: &str) -> (String, Vec<String>) {
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let out = guml_codegen::wc::WcBackend.emit(&program);
    let messages = out.diagnostics.items.iter().map(|d| d.message.clone()).collect();
    (out.files[0].contents.clone(), messages)
}

const TASKS: &str = r#"page Tasks

type Task {id, title, done:bool}
data tasks:Task[] GET /api/tasks
  add  POST   /api/tasks      {title} optimistic:prepend
  save PATCH  /api/tasks/{id} {done}  optimistic

state draft=""
state filter=all|open|done

head Tasks — {tasks.open.count} open

form >tasks.add{title:draft}; draft=""
  input draft aria="New task"
  btn Add primary disabled={!draft.trim()}

tabs filter

list tasks where={filter}
  check {done} >tasks.save
  text {title} strike={done}
  btn Delete quiet aria="Delete {title}" >tasks.save
  empty Nothing here yet.
"#;

#[test]
fn a_custom_element_is_registered_under_a_hyphenated_name() {
    let (out, warnings) = wc("page Counter\nstate count=0\n\nmetric {count}\n");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(out.contains("class Counter extends HTMLElement"), "{out}");
    // The standard requires a hyphen precisely so a custom element cannot collide with a future
    // built-in. `guml-` also keeps two compiled documents on one page apart.
    assert!(out.contains(r#"customElements.define("guml-counter", Counter)"#), "{out}");
}

#[test]
fn the_design_system_is_shared_with_the_react_backend() {
    // The IR claim, checked the way the html and svelte suites check it: the same document must produce
    // the same class strings from a backend with a completely different reactivity model. A second table
    // would make four backends agreeing a coincidence.
    let src = "page P\ncard sm center\n  btn Go primary\n";
    let (out, _) = wc(src);
    let (program, _) = check(src);
    let react = guml_codegen::react::ReactBackend.emit(&program).files[0].contents.clone();
    // Read from the *theme* rather than written out. This listed `bg-slate-900` and `focus:ring-2`, which
    // pinned one palette into a test about two backends agreeing — so changing the default theme broke it for
    // a reason unrelated to what it checks. Asking the theme what a `btn primary` is makes it hold for any
    // theme, which is the actual claim.
    let theme = guml_codegen::theme::active();
    let groups = [
        theme.classes("card", &["sm", "center"]),
        theme.classes("btn", &["primary"]),
        theme.contract.focus_visible.clone(),
    ];
    for group in &groups {
        assert!(!group.trim().is_empty(), "the theme produced nothing to compare");
        for class in group.split_whitespace() {
            assert!(out.contains(class), "wc output is missing `{class}`:\n{out}");
            assert!(react.contains(class), "react output is missing `{class}` — the test is wrong");
        }
    }
}

#[test]
fn state_reads_are_qualified_and_string_literals_are_not() {
    // The bug that shaped the whole backend. Prefixing state reads by rewriting the *lowered string*
    // cannot distinguish an identifier from the contents of a string, the literal text of a template, or
    // a lambda's own parameter — and got all three wrong simultaneously:
    //
    //   `s.Invoices — ${…} s.awaiting s.payment`
    //   (s.a, s.b) => s.a + Number(s.b)
    //   s.view === "s.all"
    //
    // The prefix is now applied by `Ctx::with_scope` during lowering, where the tree says "path head".
    let (out, _) = wc(TASKS);
    assert!(out.contains("`Tasks — ${s.tasks"), "prose was rewritten:\n{out}");
    assert!(out.contains("open`"), "the literal tail was rewritten:\n{out}");
    assert!(out.contains(r#"s.filter === "all""#), "a string literal was rewritten:\n{out}");
    assert!(!out.contains("s.awaiting"), "prose words were treated as state:\n{out}");
    assert!(!out.contains("(s.a, s.b)"), "lambda parameters were treated as state:\n{out}");
}

#[test]
fn a_row_renders_its_bindings() {
    // `text {title} strike={done}` is *structured*, not prose — it has an `=` and `strike` is a registry
    // attribute on `text` — so `content` and `label` are both `None`. Reading only those two emitted an
    // empty element, and every row of the task list was a blank line.
    let (out, _) = wc(TASKS);
    assert!(out.contains(">${item.title}</span>"), "the row's title did not render:\n{out}");
    assert!(out.contains("${item.id}"), "the row key is not the row's id:\n{out}");
}

#[test]
fn a_class_toggle_is_merged_into_the_existing_class_attribute() {
    // Emitting a second `class` produced `class="${…}" class="flex-1 …"`. HTML keeps the first and
    // discards the rest, so the element lost every theme class *and* the strike never appeared — one
    // mistake breaking both halves.
    let (out, _) = wc(TASKS);
    let row = out.lines().find(|l| l.contains("rows.map")).expect("a row template");
    let spans: Vec<&str> = row.match_indices("<span").map(|(i, _)| &row[i..]).collect();
    for span in spans {
        let end = span.find('>').unwrap_or(span.len());
        let open_tag = &span[..end];
        assert_eq!(
            open_tag.matches("class=").count(),
            1,
            "a row element has two `class` attributes, so HTML keeps only one:\n{open_tag}"
        );
    }
    assert!(out.contains("line-through text-slate-400"), "{out}");
}

#[test]
fn an_interpolated_accessible_name_is_interpolated() {
    // `aria="Delete {title}"` was emitted verbatim, so a screen reader announced "Delete {title}". The
    // accessible name is the guarantee this compiler makes hardest — a brace in it is not a cosmetic bug.
    let (out, _) = wc(TASKS);
    assert!(out.contains(r#"aria-label="Delete ${item.title}""#), "{out}");
    assert!(!out.contains("Delete {title}"), "the binding was left literal:\n{out}");
}

#[test]
fn a_bound_field_carries_the_marker_the_dispatcher_matches_on() {
    // Without `data-g-field` the input was written once at first paint and never read back, so typing
    // changed nothing. The delegated dispatcher has no other way to know the input owns a state name.
    let (out, _) = wc(TASKS);
    assert!(out.contains(r#"data-g-field="draft""#), "{out}");
    assert!(out.contains("this.#set({ [bound.dataset.gField]: bound.value })"), "{out}");
    // And its value is written exactly once — at first paint, never in `#update`. Writing it back on
    // every keystroke is what moves the cursor to the end.
    let update = out.split("#update()").nth(1).unwrap_or_default();
    assert!(
        !update.contains("el.value ="),
        "a bound field's value is written during update, which moves the cursor:\n{update}"
    );
}

#[test]
fn every_action_has_exactly_one_handler() {
    // The two failure modes of a delegated dispatcher: an element whose index has no case (a dead
    // control), and a case no element triggers (dead code that looks live).
    let (out, _) = wc(TASKS);
    let on_elements: std::collections::BTreeSet<&str> =
        out.match_indices("data-g-act=\"").map(|(i, _)| slice_until(&out[i + 12..], '"')).collect();
    let in_switch: std::collections::BTreeSet<&str> = out
        .match_indices("case \"")
        .filter_map(|(i, _)| out[i + 6..].split_once(':').map(|(_, rest)| slice_until(rest, '"')))
        .collect();
    assert!(!on_elements.is_empty(), "no actions were emitted at all");
    assert_eq!(on_elements, in_switch, "elements and handlers disagree");
}

fn slice_until(s: &str, stop: char) -> &str {
    &s[..s.find(stop).unwrap_or(s.len())]
}

#[test]
fn a_second_connect_does_not_rebuild_the_markup() {
    // A custom element is connected again whenever it moves in the DOM. Rebuilding would discard
    // whatever the user had typed, so the guard is not an optimisation.
    let (out, _) = wc(TASKS);
    assert!(out.contains("if (this.#painted) return;"), "{out}");
}

#[test]
fn an_optimistic_mutation_rolls_back() {
    // The half every hand-written optimistic update forgets, which is exactly why the compiler owns it.
    let (out, _) = wc(TASKS);
    assert!(out.contains("const snapshot = this.#state.tasks;"), "no snapshot taken:\n{out}");
    assert!(out.contains("tasks: snapshot,"), "no rollback on failure:\n{out}");
    // `prepend` is the author's declaration, not a guess.
    assert!(out.contains("...snapshot]"), "prepend strategy not honoured:\n{out}");
    // A path parameter reads the row rather than being sent literally.
    assert!(out.contains("`/api/tasks/${item.id}`"), "{out}");
}

#[test]
fn a_repeater_renders_loading_empty_and_error_without_being_asked() {
    let (out, _) = wc(TASKS);
    assert!(out.contains("s.tasksError"), "no error branch:\n{out}");
    assert!(out.contains("animate-pulse"), "no loading skeleton:\n{out}");
    assert!(out.contains("Nothing here yet."), "the empty slot did not render:\n{out}");
}

#[test]
fn a_host_component_is_reported_rather_than_emitted() {
    // A registry package may declare `"element": "Callout"` — a framework component. There is no
    // framework here to resolve it, and emitting `<Callout>` into innerHTML would produce an unknown
    // element that silently renders nothing.
    let json = r#"{"components":[
        {"name":"callout","kind":"container","doc":"Aside.","element":"Callout","import":"@acme/ds"}
    ]}"#;
    let reg = guml_registry::Registry::from_json(json).expect("loads");
    let (program, diags) = guml_compiler::check_with("page P\ncallout \"Note\"\n  p Body.\n", &reg);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    // The backend reads the *active* vocabulary, which this test cannot install (it is write-once per
    // process and other tests share it). What is checked here is that the front end accepts the document
    // — the reporting path is covered by `scripts/check-wc.mjs` over a project that installs one.
    let out = guml_codegen::wc::WcBackend.emit(&program);
    assert!(
        !out.files[0].contents.contains("<Callout"),
        "a framework component reached a framework-free backend:\n{}",
        out.files[0].contents
    );
}

#[test]
fn markup_cannot_end_the_template_literal_it_lives_in() {
    // A backtick in prose would close the literal the markup lives in, and the emitted module would not
    // parse at all. Prose is verbatim by contract, so this is reachable from a valid document — and it
    // *is* in this repository: `invoices.guml` has one, which is how the escaping got tested for real.
    let (out, _) = wc("page P\np A backtick ` in prose.\n");
    assert!(out.contains("\\`"), "a backtick was not escaped:\n{out}");

    // A literal `${` is deliberately not tested from a document, because no valid document can contain
    // one: in prose a `{` always opens a binding, so `${x}` is an interpolation and `${` alone is
    // `GUML0003`. The escaping exists as a backstop and is unit-tested in
    // `guml_codegen::wc::tests::a_template_literal_cannot_be_ended_early_by_the_markup`, where the input
    // does not have to be a legal document.
    //
    // A brace that *is* a binding interpolates rather than escaping, which is the reachable case:
    let (out, _) = wc("page P\nstate total=0\n\np Costs {total} today.\n");
    assert!(out.contains("`Costs ${s.total} today.`"), "{out}");
}
