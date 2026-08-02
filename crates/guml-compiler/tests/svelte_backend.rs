//! The Svelte 5 backend.
//!
//! Four backends now, and this one earns its place by exercising a part of the AST the others do not.
//! React has reactivity expressed imperatively (hooks, setters, dependency arrays); static HTML has
//! none at all. Svelte has reactivity expressed *declaratively*, so it is the case that catches
//! anything the compiler was accidentally lowering in a React-shaped way.
//!
//! Most of what is asserted here is agreement: the same theme, the same expression lowering, the same
//! dead-declaration elimination, the same optimistic-update semantics. What differs is only what the
//! target genuinely does differently.

use guml_codegen::Backend as _;
use guml_compiler::check;

fn svelte(src: &str) -> String {
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    guml_codegen::svelte::SvelteBackend.emit(&program).files[0].contents.clone()
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("../../fixtures/{name}")).expect(name)
}

#[test]
fn state_becomes_a_rune_and_an_action_assigns_it_directly() {
    // The compile-away-the-framework story in miniature: no setter, no hook, no import. A rune is a
    // variable, so `count++` is `count++`.
    let out = svelte(&fixture("a.guml"));
    assert!(out.contains("let count = $state(0);"), "{out}");
    assert!(out.contains("onclick={() => { count++; }}"), "{out}");
    assert!(!out.contains("useState"), "React machinery leaked into Svelte output:\n{out}");
    assert!(!out.contains("import"), "runes need no imports:\n{out}");
}

#[test]
fn a_resource_becomes_an_effect_with_cancellation() {
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains("let tasks = $state([]);"), "{out}");
    // The fetch is a named function so `>tasks.list` can re-run it; `$effect` takes it directly,
    // because it returns its own teardown. This assertion used to pin the inline `$effect(() => {`
    // form, which said nothing about the property that matters — that the fetch runs and aborts.
    assert!(out.contains("function tasksList() {"), "{out}");
    assert!(out.contains("$effect(tasksList);"), "{out}");
    // The part hand-written effects get wrong, and the reason the compiler owns it.
    assert!(out.contains("new AbortController()"), "{out}");
    assert!(out.contains("return () => controller.abort();"), "{out}");
}

#[test]
fn a_where_filter_is_derived_rather_than_memoised() {
    // The clearest place the two runtimes differ: React needs `useMemo` plus a hand-built dependency
    // array for this; Svelte tracks it, so the same GUML produces less code.
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains("const visibleTasks = $derived("), "{out}");
    assert!(!out.contains("useMemo"), "{out}");
    assert!(!out.contains("[tasks, filter]"), "no dependency array should be emitted:\n{out}");
}

#[test]
fn optimistic_updates_and_rollback_match_the_react_backend() {
    // The semantics are the compiler's, not the target's. If these drifted apart, one of the two
    // backends would be quietly wrong about what `optimistic:prepend` means.
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains("const snapshot = tasks;"), "{out}");
    assert!(out.contains("tasks = [{ ...body }, ...tasks];"), "prepend strategy:\n{out}");
    assert!(out.contains("tasks = tasks.filter((it) => it !== item);"), "delete strategy:\n{out}");
    assert!(out.contains("tasks = snapshot;"), "rollback:\n{out}");
}

#[test]
fn a_repeater_becomes_each_with_the_loading_and_empty_branches() {
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains("{#each visibleTasks as item (item.id)}"), "keyed each:\n{out}");
    assert!(out.contains("{#if tasksError}"), "error branch:\n{out}");
    assert!(out.contains("{:else if tasksLoading}"), "loading branch:\n{out}");
    assert!(out.contains("animate-pulse"), "the skeleton is still compiled in:\n{out}");
    assert!(out.contains("Nothing here yet."), "the author's empty message:\n{out}");
}

#[test]
fn two_way_binding_uses_a_bind_directive() {
    // The one place Svelte is meaningfully terser rather than merely different: a value/handler pair
    // becomes one directive.
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains("bind:value={draft}"), "{out}");
    assert!(out.contains("bind:checked={item.done}"), "{out}");
}

#[test]
fn a_checkbox_toggle_posts_the_negated_field() {
    // `check {done} >tasks.save` is a toggle. Without deriving the body from the binding the mutation
    // would post `{}` and silently save nothing — the exact class of bug that looks like it works.
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains("tasksSave(item, { done: !item.done })"), "{out}");
}

#[test]
fn a_form_submit_button_is_a_submit_button() {
    // A `btn` inside a form with no action of its own *is* the submit control. `type="button"` there
    // produces a form that cannot be submitted from the keyboard.
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains(r#"type="submit">Add</button>"#), "{out}");
    assert!(out.contains("onsubmit={(e) => { e.preventDefault();"), "{out}");
}

#[test]
fn an_attribute_interpolation_is_qualified_to_the_row() {
    // `aria="Delete {title}"` inside a repeater must become `item.title`, or the template references a
    // name that is not in scope. A plain string stays a plain string.
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains("aria-label={`Delete ${item.title}`}"), "{out}");
    assert!(
        out.contains(r#"aria-label="New task""#),
        "a literal should not become a template:\n{out}"
    );
}

#[test]
fn the_theme_is_the_same_one_the_react_backend_uses() {
    // What makes a fourth backend evidence rather than a coincidence. Not "the classes look right" —
    // "the classes are the same table".
    let src = "page P\ncard sm center\n  h Clicks\n  p Press the buttons.\n";
    let (program, _) = check(src);
    let sv = &guml_codegen::svelte::SvelteBackend.emit(&program).files[0].contents;
    let react = &guml_codegen::react::ReactBackend.emit(&program).files[0].contents;

    let mut checked = 0;
    for chunk in react.split("className=\"").skip(1) {
        let Some(classes) = chunk.split('"').next() else { continue };
        if classes.is_empty() {
            continue;
        }
        checked += 1;
        assert!(sv.contains(classes), "svelte does not share the class string {classes:?}\n{sv}");
    }
    assert!(checked >= 3, "expected several class strings to compare, got {checked}");
}

#[test]
fn dead_declarations_are_eliminated_here_too() {
    let out = svelte("page P\nstate used=0\nstate dead=\"\"\nmetric {used}\n");
    assert!(out.contains("let used = $state(0);"), "{out}");
    assert!(!out.contains("dead"), "a dead state was emitted:\n{out}");
}

#[test]
fn escape_blocks_reach_only_their_own_backend() {
    // `d.guml` carries a `raw react` block and a `raw svelte` block. This is the backend where the
    // second one is the live path and the first must not appear.
    let out = svelte(&fixture("d.guml"));
    assert!(out.contains("Svelte total"), "the svelte block was not emitted:\n{out}");
    assert!(!out.contains("className"), "a React block leaked into Svelte output:\n{out}");
    // A `js` block is component-body code here exactly as in React.
    assert!(
        out.contains("const currency ="),
        "the js block should be hoisted into <script>:\n{out}"
    );
}

#[test]
fn a_content_page_needs_no_script_at_all() {
    // The landing fixture has no state and no resources, so the emitted component is pure markup —
    // which is the same conclusion the static-HTML backend reaches, by a different route.
    let out = svelte(&fixture("c.guml"));
    assert!(!out.contains("<script>"), "a stateless page should emit no script block:\n{out}");
    assert!(out.contains("<details"), "faq still lowers to a disclosure element:\n{out}");
}

#[test]
fn tabs_iterate_the_declared_domain() {
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains(r#"{#each ["all", "open", "done"] as option (option)}"#), "{out}");
    assert!(out.contains("onclick={() => (filter = option)}"), "{out}");
    assert!(out.contains("aria-pressed={filter === option}"), "{out}");
}

/// `where=` filters on the row's boolean field, whatever it is called — and both backends agree.
///
/// This is the third place the same hardcode sat. The aggregate `.open`/`.done` and the body-less
/// `save` toggle each assumed a field named `done`; here the assumption produced *no* filter at all
/// for `state view=all|open|paid`, so the tabs rendered, changed the state, and the table ignored them.
/// It warned rather than lying, which is invariant 3 holding — but the right answer was available from
/// the row type the whole time.
///
/// Asserted across both backends deliberately. The Svelte version was a hand-copied paraphrase of the
/// React one under a comment claiming they matched, and it stopped matching the moment React was fixed.
#[test]
fn a_where_filter_uses_the_rows_own_boolean_field_in_both_backends() {
    let src = fixture("invoices.guml");
    let sv = svelte(&src);
    assert!(sv.contains(r#"view === "open" ? invoices.filter((it) => !it.paid)"#), "{sv}");
    assert!(sv.contains(r#"view === "paid" ? invoices.filter((it) => it.paid)"#), "{sv}");

    let (program, _) = guml_compiler::check(&src);
    let react = guml_codegen::react::ReactBackend.emit(&program).files[0].contents.clone();
    // Same predicates, whatever the surrounding syntax each target needs.
    for predicate in ["invoices.filter((it) => !it.paid)", "invoices.filter((it) => it.paid)"] {
        assert!(react.contains(predicate), "react is missing {predicate}:\n{react}");
        assert!(sv.contains(predicate), "svelte is missing {predicate}:\n{sv}");
    }
}

/// And the `done`-named case is untouched, so the generalisation did not trade one shape for another.
#[test]
fn the_done_named_case_still_filters_the_same_way() {
    let out = svelte(&fixture("b.guml"));
    assert!(out.contains(r#"filter === "open" ? tasks.filter((it) => !it.done)"#), "{out}");
    assert!(out.contains(r#"filter === "done" ? tasks.filter((it) => it.done)"#), "{out}");
}
