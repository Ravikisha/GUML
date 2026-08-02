//! Declared effects: `on mount` and `on {expr}`.
//!
//! # What is actually being claimed
//!
//! Not "GUML has effects" — React has effects. The claim is that the dependency cannot be wrong,
//! because there is no second list to keep in sync with the first. `useEffect(fn, [deps])` fails in
//! two directions: a missing entry reads stale values, a spurious one re-runs forever. Both are
//! mistakes a model makes readily, because the correct array is not derivable from the lines nearby.
//!
//! So most of what is asserted here is that the trigger *is* the dependency, in each backend's own
//! idiom — and that the Svelte case needs `untrack` to be exact, which is the sharpest evidence that
//! this is worth compiling rather than leaving to the author.
//!
//! The other half is that an effect is not a second-class citizen: the same action language, the same
//! diagnostics, the same liveness rules. A hole in any of those would make the stricter path the real
//! one and this an escape from it.

use guml_codegen::Backend as _;
use guml_compiler::check;

fn codes(src: &str) -> Vec<String> {
    check(src).1.items.iter().map(|d| d.id.to_string()).collect()
}

fn messages(src: &str) -> Vec<String> {
    check(src).1.items.iter().map(|d| d.message.clone()).collect()
}

fn react(src: &str) -> String {
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    guml_codegen::react::ReactBackend.emit(&program).files[0].contents.clone()
}

fn svelte(src: &str) -> String {
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    guml_codegen::svelte::SvelteBackend.emit(&program).files[0].contents.clone()
}

const DOC: &str = "page Tasks\n\
                   type Task {id, title, done:bool}\n\
                   data tasks:Task[] GET /api/tasks\n\
                   state filter=all|open|done\n\
                   \n";

fn doc(rest: &str) -> String {
    format!("{DOC}{rest}\n\ntabs filter\nlist tasks where={{filter}}\n  text {{title}}\n")
}

/* ------------------------------------------------------------------ the trigger is the dependency */

#[test]
fn mount_is_an_empty_dependency_array() {
    let out = react(&doc("on mount >tasks.list"));
    assert!(out.contains("useEffect(() => { tasksList(); }, []);"), "{out}");
}

#[test]
fn a_binding_trigger_becomes_that_one_dependency() {
    let out = react(&doc("on {filter} >tasks.list"));
    assert!(out.contains("useEffect(() => { tasksList(); }, [filter]);"), "{out}");
}

#[test]
fn svelte_reads_the_trigger_and_untracks_the_body() {
    // The case that justifies compiling this rather than writing it. Svelte's `$effect` tracks every
    // reactive read in its body, so the naive translation would re-run whenever anything the action
    // touches changes — not when the declared trigger does. Reading the trigger and wrapping the body
    // in `untrack` makes the dependency exactly what the author wrote.
    let out = svelte(&doc("on {filter} >tasks.list"));
    assert!(out.contains("void filter;"), "the trigger must be read to be tracked:\n{out}");
    assert!(out.contains("untrack(() => { tasksList(); })"), "{out}");
    assert!(out.contains("import { untrack } from \"svelte\";"), "{out}");
}

#[test]
fn svelte_mount_is_on_mount_not_an_effect() {
    // `$effect` with an empty body would still run whenever a read inside it invalidated. `onMount`
    // is the idiom that means once.
    let out = svelte(&doc("on mount >tasks.list"));
    assert!(out.contains("onMount(() => { tasksList(); });"), "{out}");
    assert!(out.contains("import { onMount } from \"svelte\";"), "{out}");
}

/* ------------------------------------------------------------------ the implicit `list` mutation */

#[test]
fn a_resource_fetch_is_callable_as_list() {
    // Without this there is no way to say "fetch that again", which makes `on {filter}` useless and a
    // Reload button impossible. `list` is not new vocabulary: the JSON IR has always called the GET
    // that, and every `data` declaration has exactly one.
    let out = react(&doc("btn Reload quiet >tasks.list"));
    assert!(out.contains("const tasksList = useCallback(() => {"), "{out}");
    assert!(out.contains("onClick={() => { tasksList(); }}"), "{out}");
    // Still fetched on mount with no effect declared, and still aborting. The cleanup is now a block
    // rather than a one-liner, because it also clears the `alive` flag that guards the state writes — a
    // cache hit can resolve after unmount without the abort ever firing.
    assert!(out.contains("useEffect(tasksList, [tasksList]);"), "{out}");
    assert!(out.contains("controller.abort();"), "{out}");
    assert!(out.contains("alive = false;"), "{out}");
}

#[test]
fn list_takes_no_body_even_inside_a_repeater() {
    // A mutation inside a row is called with `(item, body)`. `list` is not a row operation, so
    // getting this wrong would emit `tasksList(item, {})` against a zero-argument function.
    let src = format!(
        "{DOC}list tasks\n  text {{title}}\n  btn Refresh quiet aria=\"Refresh {{title}}\" >tasks.list\n"
    );
    let out = react(&src);
    assert!(out.contains("tasksList()"), "{out}");
    assert!(!out.contains("tasksList(item"), "`list` is not a row operation:\n{out}");
}

/* ------------------------------------------------------------------ an effect is not exempt */

#[test]
fn an_undeclared_trigger_is_reported() {
    let src = doc("on {nothere} >tasks.list");
    assert_eq!(codes(&src), vec!["GUML0033"]);
    assert!(messages(&src)[0].contains("effect trigger"), "{:?}", messages(&src));
}

#[test]
fn an_undeclared_action_target_is_reported() {
    let src = doc("on mount >nosuch.list");
    assert_eq!(codes(&src), vec!["GUML0033"]);
}

#[test]
fn an_unknown_mutation_is_the_same_error_as_on_a_button() {
    // The point of routing both through one validator. If these two diverged, the looser path would
    // be a hole in the stricter one.
    let from_effect = codes(&doc("on mount >tasks.nope"));
    let from_button = codes(&doc("btn Go >tasks.nope"));
    assert_eq!(from_effect, vec!["GUML0061"]);
    assert_eq!(from_effect, from_button);
}

#[test]
fn a_bad_aggregate_in_a_trigger_is_a_type_error() {
    let src = doc("on {tasks.title.sum} >tasks.list");
    assert_eq!(codes(&src), vec!["GUML0065"]);
}

#[test]
fn a_missing_trigger_and_a_missing_action_are_both_reported() {
    assert_eq!(codes(&doc("on sometimes >tasks.list")), vec!["GUML0098"]);
    assert_eq!(codes(&doc("on mount")), vec!["GUML0098"]);
    assert!(
        messages(&doc("on mount"))[0].contains("no action"),
        "{:?}",
        messages(&doc("on mount"))
    );
}

#[test]
fn every_error_in_one_pass() {
    // Invariant 1. Each repair round is a full generation, so reporting these one at a time would turn
    // one round into four.
    let src = doc(
        "on {nothere} >tasks.list\non mount >nosuch.list\non mount >tasks.nope\non never >tasks.list",
    );
    let got = codes(&src);
    // Four mistakes on four lines, all from one `check`. Two `GUML0033`s because the undeclared
    // trigger and the undeclared action target are distinct references, not one line reported twice.
    assert_eq!(got, vec!["GUML0098", "GUML0033", "GUML0033", "GUML0061"], "{got:?}");
}

/* ------------------------------------------------------------------ liveness and levels */

#[test]
fn a_state_read_only_by_a_trigger_is_still_live() {
    // The optimizer deletes declarations nothing references. Before effects fed the liveness walker,
    // `on {filter}` did not count as a use — so the state was warned about as dead and then elided,
    // leaving an effect with a dangling dependency.
    let src = format!("{DOC}on {{filter}} >tasks.list\n\nlist tasks\n  text {{title}}\n");
    assert_eq!(codes(&src), Vec::<String>::new(), "`filter` is used by the effect");
    assert!(react(&src).contains("useState"), "the state must survive elimination");
}

#[test]
fn on_is_app_level() {
    // An effect performs I/O on a schedule. A host that asked for markup only must not get one, and
    // the answer is rejection rather than quiet filtering.
    let src = doc("on mount >tasks.list");
    let (_, diags) = guml_compiler::check_with(&src, &guml_registry::Registry::core());
    assert!(diags.items.iter().any(|d| d.id == "GUML0091"), "{:?}", diags.items);
}

#[test]
fn the_html_backend_says_it_cannot_run_them() {
    // Invariant 3. A static page with a declared refetch renders as it was on first paint; silence
    // here would look like the effect was honoured.
    let (program, _) = check(&doc("on {filter} >tasks.list"));
    let out = guml_codegen::html::HtmlBackend {
        style: guml_codegen::html::Style::Inline,
        ..Default::default()
    }
    .emit(&program);
    let msgs: Vec<&str> = out.diagnostics.items.iter().map(|d| d.message.as_str()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("declared effect")),
        "the html backend must report what it cannot honour: {msgs:?}"
    );
}

#[test]
fn the_json_tree_carries_them_for_the_runtime() {
    // Unlike a `js` body, an effect is not arbitrary code — the trigger is an expression and the
    // action is the same restricted language every button already carries in this tree. So it travels
    // rather than being dropped: nothing here can reach `eval`.
    let (program, _) = check(&doc("on {filter} >tasks.list"));
    let mut sink = guml_diagnostics::Diagnostics::new();
    let tree = guml_codegen::json::ui_tree(&program, &mut sink);
    assert_eq!(tree.effects.len(), 1);
    assert_eq!(tree.effects[0].on, "filter");
    assert_eq!(tree.effects[0].actions, vec!["tasks.list".to_string()]);
}

#[test]
fn the_formatter_round_trips_an_effect() {
    let src = doc("on {filter} >tasks.list");
    let once = guml_fmt::format_str(&src);
    assert_eq!(guml_fmt::format_str(&once), once, "fmt must be idempotent");
    assert!(once.contains("on {filter} >tasks.list"), "{once}");
}
