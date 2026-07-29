//! The desugar pass, end to end from source text.
//!
//! These live here rather than in `guml-codegen` because they need the parser, and
//! codegen must not depend on it (that would be a cycle through this crate).
//!
//! What they assert is the project's central claim in concrete form: one `data`
//! declaration expands into the fetch, cancellation, optimistic update, rollback,
//! loading, empty and error handling that a model would otherwise have to write —
//! and get right — by hand.

use guml_codegen::{Backend, react::ReactBackend};
use guml_diagnostics::Diagnostics;
use guml_registry::Registry;

fn emit(src: &str) -> (String, Diagnostics) {
    let reg = Registry::builtin();
    let parsed = guml_parser::parse(src, &reg);
    let out = ReactBackend.emit(&parsed.program);
    (out.files[0].contents.clone(), out.diagnostics)
}

const TASKS: &str = "page Tasks\n\
    type Task {id, title, done:bool}\n\
    data tasks:Task[] GET /api/tasks\n\
    \x20 add  POST   /api/tasks      {title}  optimistic:prepend\n\
    \x20 save PATCH  /api/tasks/{id} {done}   optimistic\n\
    \x20 drop DELETE /api/tasks/{id}          optimistic\n\
    state draft=\"\"\n\
    state filter=all|open|done\n\
    head Tasks — {tasks.open.count} open\n\
    form >tasks.add{title:draft}; draft=\"\"\n\
    \x20 input draft aria=\"New task\" placeholder=\"Add a task…\"\n\
    \x20 btn Add primary disabled={!draft.trim()} busy=\"Adding…\"\n\
    tabs filter\n\
    list tasks where={filter}\n\
    \x20 check {done} >tasks.save\n\
    \x20 text {title} strike={done}\n\
    \x20 btn Delete quiet aria=\"Delete {title}\" >tasks.drop\n\
    \x20 empty Nothing here yet.\n";

/// A non-boolean filter domain, with a row field of the same name.
const PROJECTS: &str = "page P\n\
    type Project {id, name, area}\n\
    data projects:Project[] GET /api/projects\n\
    state area=all|web|systems\n\
    tabs area\n\
    table projects where={area}\n\
    \x20 text {name}\n";

/// A field aggregate over a resource: count the rows whose `live` is true.
const SHIPPED: &str = "page P\n\
    type Project {id, name, live:bool}\n\
    data projects:Project[] GET /api/projects\n\
    head Shipped {projects.live.count}\n\
    list projects\n\
    \x20 text {name}\n";

#[test]
fn counter_still_compiles() {
    let (src, d) = emit(
        "page Counter\nstate count=0\n\ncard sm center\n  h Clicks\n  metric {count}\n  btn Increment primary >count++\n",
    );
    assert!(!d.has_errors());
    assert!(src.contains("const [count, setCount] = useState(0);"));
    assert!(src.contains("onClick={() => { setCount(count + 1); }}"));
    assert!(src.contains("bg-slate-900"));
}

// ---- the desugar pass: what the model no longer has to write ----

#[test]
fn a_resource_becomes_state_a_fetch_and_callbacks() {
    let (src, d) = emit(TASKS);
    assert!(!d.has_errors(), "{:?}", d.items);

    assert!(src.contains("const [tasks, setTasks] = useState<Task[]>([]);"));
    assert!(src.contains("const [tasksLoading, setTasksLoading] = useState(true);"));
    assert!(src.contains("const [tasksError, setTasksError] = useState<string | null>(null);"));

    // Cancellation is the thing hand-written effects get wrong.
    assert!(src.contains("const controller = new AbortController();"));
    assert!(src.contains("return () => controller.abort();"));
    assert!(src.contains("err.name === \"AbortError\""));
}

#[test]
fn mutations_apply_optimistically_and_roll_back() {
    let (src, _) = emit(TASKS);

    assert!(src.contains("setTasks((prev) => [optimistic, ...prev]);"), "prepend");
    assert!(src.contains("it === optimistic ? created : it"), "temporary row replaced");
    assert!(src.contains("it === item ? { ...it, ...body } : it"), "patch in place");
    assert!(src.contains("prev.filter((it) => it !== item)"), "delete removes");
    assert_eq!(
        src.matches("setTasks(snapshot);").count(),
        3,
        "every mutation restores the snapshot on failure"
    );
}

#[test]
fn a_repeater_emits_loading_empty_error_and_keys() {
    let (src, _) = emit(TASKS);
    assert!(src.contains("{tasksError && ("), "error banner");
    assert!(src.contains("role=\"alert\""));
    assert!(src.contains("animate-pulse"), "loading skeleton");
    assert!(src.contains("visibleTasks.length === 0"), "empty branch");
    assert!(src.contains("Nothing here yet."));
    assert!(src.contains("key={item.id}"), "keyed by the item's id, not the index");
}

#[test]
fn the_filter_becomes_a_memo() {
    let (src, _) = emit(TASKS);
    assert!(src.contains("const visibleTasks = useMemo("));
    assert!(src.contains("filter === \"open\""));
    assert!(src.contains("[tasks, filter]"), "dependency list is derived, not guessed");
    assert!(src.contains("import { useState, useCallback, useEffect, useMemo }"));
}

#[test]
fn aggregates_in_prose_become_real_javascript() {
    let (src, _) = emit(TASKS);
    // `{tasks.open.count}` used to be emitted verbatim, which is not valid JS.
    assert!(src.contains("{tasks.filter((it) => !it.done).length}"));
    assert!(!src.contains("tasks.open.count"));
}

#[test]
fn row_bindings_are_qualified_to_the_item() {
    let (src, _) = emit(TASKS);
    assert!(src.contains("checked={item.done}"));
    assert!(src.contains("{item.title}"));
    assert!(src.contains("tasksDrop(item, {})"));
    assert!(
        src.contains("tasksSave(item, { done: !item.done })"),
        "a body-less save on a row toggles the boolean"
    );
}

#[test]
fn a_form_submits_and_resets() {
    let (src, _) = emit(TASKS);
    assert!(src.contains("onSubmit={(e) => { e.preventDefault();"));
    assert!(src.contains("tasksAdd({ title: draft })"));
    assert!(src.contains("setDraft(\"\")"));
    assert!(src.contains("disabled={!draft.trim()}"));
    assert!(
        src.contains("{tasksSaving ? \"Adding…\" : \"Add\"}"),
        "the busy label watches the mutation, not the initial fetch: `tasksLoading` is \
         true during page load, so the button would read \"Adding…\" before anyone typed"
    );
}

#[test]
fn a_mutation_pending_flag_clears_on_both_paths() {
    let (src, _) = emit(TASKS);
    assert!(src.contains("const [tasksSaving, setTasksSaving] = useState(false);"));
    assert!(src.contains("setTasksSaving(true);"));
    assert!(
        src.contains("} finally {\n        setTasksSaving(false);\n      }"),
        "a failed mutation must clear the flag too, or the button stays busy forever"
    );
}

#[test]
fn a_pending_flag_is_only_declared_where_it_is_read() {
    // No `busy` anywhere, so the flag would be dead weight in every emitted file.
    let src = emit(
        "page Feed\n\
         type Post {id, title}\n\
         data posts:Post[] GET /api/posts\n\
           drop DELETE /api/posts/{id} optimistic\n\n\
         list posts\n  text {title}\n",
    )
    .0;
    assert!(!src.contains("postsSaving"), "unused pending state should not be emitted");
    assert!(src.contains("const [postsLoading"), "the fetch flag is still there");
}

#[test]
fn a_binding_inside_an_attribute_becomes_a_template_literal() {
    let (src, _) = emit(TASKS);
    assert!(
        src.contains("aria-label={`Delete ${item.title}`}"),
        "JSX has no expression syntax inside quotes, so `aria=\"Delete {{title}}\"` would \
         ship the braces to the DOM and the accessible name would read literally"
    );
    assert!(!src.contains("aria-label=\"Delete {item.title}\""));
}

#[test]
fn a_filter_over_a_non_boolean_domain_uses_the_matching_row_field() {
    // Found by writing an actual portfolio page. The filter was hardcoded to fixture B's
    // shape — `open`/`done` against a `done` field — so any other domain emitted comparisons
    // that could never match, plus `.done` on a type without it. It compiled; only `tsc` over
    // the output caught it.
    let (src, _) = emit(PROJECTS);
    assert!(
        src.contains("area === \"all\" ? projects : projects.filter((it) => it.area === area)"),
        "the row field of the same name is the discriminant
{src}"
    );
    assert!(
        !src.contains("it.done"),
        "no field borrowed from another fixture's type
{src}"
    );
}

#[test]
fn a_filter_with_nothing_to_filter_on_warns_instead_of_guessing() {
    // Invariant 3: an unfiltered list is visibly wrong, a wrong filter is not.
    let (src, diags) = emit(&PROJECTS.replace(", area}", "}"));
    assert!(
        diags.items.iter().any(|d| d.message.contains("where=")),
        "expected a warning, got {:?}",
        diags.items
    );
    assert!(
        src.contains("const visibleProjects = projects;"),
        "renders everything
{src}"
    );
}

#[test]
fn a_field_aggregate_over_a_resource_filters_the_rows() {
    let (src, _) = emit(SHIPPED);
    assert!(
        src.contains("projects.filter((it) => it.live).length"),
        "`projects.live.count` counts the rows whose field is true
{src}"
    );
    assert!(
        !src.contains("projects.live.length"),
        "not a property of the array
{src}"
    );
}

#[test]
fn the_input_kind_becomes_the_dom_type() {
    // `kind=email` was emitted as a `kind` DOM prop beside `type="text"`. React has no such
    // property, so the output did not typecheck.
    let (src, _) = emit(
        "page P
state email=\"\"

input email kind=email aria=\"Email\"
",
    );
    assert!(src.contains("type=\"email\""), "{src}");
    assert!(!src.contains("kind="), "{src}");
}

#[test]
fn tabs_come_from_the_enumerated_domain() {
    let (src, _) = emit(TASKS);
    assert!(src.contains("[\"all\", \"open\", \"done\"] as const"));
    assert!(src.contains("aria-pressed={filter === option}"));
    assert!(src.contains("setFilter(option)"));
    assert!(
        src.contains("useState<\"all\" | \"open\" | \"done\">"),
        "an invalid value will not typecheck"
    );
}

#[test]
fn tier_and_faq_lower_from_content_lines() {
    let src = "page Pricing\n\
               tier Pro $24/mo \"For working developers\" cta=\"Go Pro\" /signup featured\n\
               \x20 Unlimited projects\n\
               \x20 Custom domains\n\
               faq open=1\n\
               \x20 Can I export? | Yes. Plain source.\n";
    let (out, d) = emit(src);
    assert!(!d.has_errors(), "{:?}", d.items);
    assert!(out.contains("border-2 border-slate-900"), "featured tier");
    assert!(out.contains("Unlimited projects"));
    assert!(out.contains("Go Pro"));
    assert!(out.contains("<details"), "faq needs no hook to be accessible");
    assert!(out.contains("Yes. Plain source."));
}

#[test]
fn an_undeclared_repeater_source_is_reported_not_guessed() {
    let (_, d) = emit("page P\nlist ghosts\n  text {title}\n");
    assert!(!d.is_empty(), "the gap has to be reported");
}

#[test]
fn the_task_fixture_leaves_no_todo_markers() {
    let (out, _) = emit(TASKS);
    assert!(!out.contains("TODO(guml)"), "everything in this fixture lowers");
}

// ---- bugs found by typechecking the emitted output, kept fixed ----

#[test]
fn several_root_elements_are_wrapped_in_a_fragment() {
    // JSX allows one root. Without this the emitted file does not parse at all,
    // which is what `tsc` reported on both multi-section fixtures.
    let (src, _) = emit(
        "page P
state n=0

h One
p Two
",
    );
    assert!(src.contains("    <>"));
    assert!(src.contains("    </>"));

    // A single root needs no fragment.
    let (single, _) = emit(
        "page P
card sm
  h One
",
    );
    assert!(!single.contains("<>"));
}

#[test]
fn layout_attributes_become_classes_not_dom_props() {
    // `cols={3}` is not a valid prop on <section>; it is presentation, so it
    // belongs in the class list.
    let (src, _) = emit(
        "page P
section #features Features cols=3
  card \"A\" | body
",
    );
    assert!(src.contains("md:grid-cols-3"));
    assert!(!src.contains("cols={3}"));
}

#[test]
fn emitted_code_carries_line_provenance() {
    // Without a map, a stack trace points at generated code the author never wrote — which the
    // report names as an adoption blocker rather than a nicety. The assertion is about
    // *correctness* of the mapping, not its presence: a map that points at the wrong line is
    // worse than none, because a debugger will confidently open the wrong place.
    let reg = Registry::builtin();
    let parsed = guml_parser::parse(TASKS, &reg);
    let out = ReactBackend.emit(&parsed.program);
    let file = &out.files[0];
    let map = file.source_map.as_ref().expect("the React backend records provenance");

    let emitted: Vec<&str> = file.contents.lines().collect();
    let source: Vec<&str> = TASKS.lines().collect();

    // Find the line that declares the resource's state and ask where it came from.
    let hook_line = emitted
        .iter()
        .position(|l| l.contains("const [tasks, setTasks]"))
        .expect("resource state is emitted") as u32;
    let origin = map.source_line_of(hook_line).expect("mapped") as usize;
    assert!(
        source[origin - 1].starts_with("data tasks"),
        "the resource hooks map to the `data` line, got {:?}",
        source[origin - 1]
    );

    // And a JSX line maps to the element that produced it.
    let form_line =
        emitted.iter().position(|l| l.contains("<form")).expect("form is emitted") as u32;
    let origin = map.source_line_of(form_line).expect("mapped") as usize;
    assert!(
        source[origin - 1].starts_with("form >"),
        "the form maps to its own line, got {:?}",
        source[origin - 1]
    );
}
