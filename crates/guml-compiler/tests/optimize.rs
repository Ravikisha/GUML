//! Dead-declaration elimination.
//!
//! A `state` or `data` that nothing refers to costs real output: one `useState` line, or ~60 lines
//! of fetch/effect/callbacks *plus* a network request on mount for data no element reads. Eliding
//! it is safe only because the liveness answer is shared — `guml_ast::referenced_names` is the same
//! function the validator uses for `GUML0074`/`GUML0075`, so anything dropped here was already
//! reported to the author, and anything the walker counts as a reference survives.
//!
//! The dangerous direction is under-approximating liveness: elide a declaration the emitted code
//! still mentions and the output stops compiling. Most of this file is about that direction.

use guml_codegen::Backend as _;
use guml_compiler::check;

fn react(src: &str) -> String {
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    guml_codegen::react::ReactBackend.emit(&program).files[0].contents.clone()
}

const WITH_DEAD: &str = r#"page P
state used=0
state dead=""
type Row {id}
data rows:Row[] GET /api/rows
data orphan:Row[] GET /api/orphan
metric {used}
list rows
  text {id}
"#;

#[test]
fn unreferenced_declarations_are_not_emitted() {
    let out = react(WITH_DEAD);
    assert!(!out.contains("dead"), "dead state emitted:\n{out}");
    assert!(!out.contains("orphan"), "orphan resource emitted:\n{out}");
    // The live ones are untouched.
    assert!(out.contains("const [used, setUsed] = useState"), "{out}");
    assert!(out.contains("/api/rows"), "{out}");
}

#[test]
fn the_author_is_told_rather_than_left_to_notice() {
    // Silent elimination would be fine for a compiler nobody debugs; here it would hide a
    // generation mistake the repair loop needs to see. The warnings are the other half of it.
    let (_, diags) = check(WITH_DEAD);
    let codes: Vec<_> = diags.items.iter().map(|d| d.id.as_str()).collect();
    assert!(codes.contains(&"GUML0074"), "no unused-state warning: {codes:?}");
    assert!(codes.contains(&"GUML0075"), "no unused-resource warning: {codes:?}");
}

#[test]
fn eliminating_a_declaration_removes_its_import_too() {
    // `useState` is imported because a state or resource exists. If that is decided before
    // elimination, a document whose only state is dead emits an import nothing uses.
    let out = react("page P\nstate dead=0\ncard Hi\n  p x\n");
    assert!(!out.contains("useState"), "unused import survived elimination:\n{out}");
    assert!(!out.contains("import"), "no hook is needed at all here:\n{out}");
}

#[test]
fn a_reference_from_a_js_body_keeps_a_declaration_alive() {
    // The failure this guards against is silent and total: elide `month`, and the `js` block that
    // reads it is emitted verbatim referring to a variable that no longer exists. The output does
    // not compile, and the compiler said nothing.
    let out = react("page P\nstate month=all|q1\njs\n  const isQ1 = month === \"q1\";\ncard Hi\n");
    assert!(out.contains("const [month, setMonth]"), "state elided despite a js use:\n{out}");
    assert!(out.contains("const isQ1 = month"), "{out}");
}

#[test]
fn a_reference_from_an_action_keeps_a_resource_alive() {
    // `>rows.add{title:draft}` refers to `rows` and to `draft`, neither as a binding.
    let out = react(
        "page P\nstate draft=\"\"\ntype Row {id, title}\ndata rows:Row[] GET /api/rows\n  add POST /api/rows {title}\nform >rows.add{title:draft}; draft=\"\"\n  input draft aria=\"New\"\n",
    );
    assert!(out.contains("/api/rows"), "resource elided despite an action use:\n{out}");
    assert!(out.contains("const [draft, setDraft]"), "state elided despite an action use:\n{out}");
}

#[test]
fn nothing_is_eliminated_from_the_real_fixtures() {
    // The fixtures are the published token measurements. If elimination changed their output, the
    // numbers in the report would silently stop describing what the compiler emits.
    for file in ["a.guml", "b.guml", "c.guml", "d.guml", "portfolio.guml"] {
        let src = std::fs::read_to_string(format!("../../fixtures/{file}")).expect(file);
        let (program, diags) = check(&src);
        assert!(!diags.has_errors(), "{file}: {:?}", diags.items);
        let live = guml_ast::referenced_names(&program);
        for s in &program.states {
            assert!(live.contains(&s.name), "{file}: state `{}` would be elided", s.name);
        }
        for r in &program.resources {
            assert!(live.contains(&r.name), "{file}: resource `{}` would be elided", r.name);
        }
    }
}

#[test]
fn the_json_backend_eliminates_the_same_declarations() {
    // This tree drives the browser runtime, where an unreferenced `data` is a live network request
    // for data nothing renders.
    let (program, _) = check(WITH_DEAD);
    let json = &guml_codegen::json::JsonBackend.emit(&program).files[0].contents;
    assert!(!json.contains("orphan"), "runtime would fetch an unused resource:\n{json}");
    assert!(!json.contains("dead"), "{json}");
    assert!(json.contains("/api/rows"), "{json}");
}

/// Common-subexpression elimination for aggregates.
///
/// `{tasks.open.count}` lowers to `tasks.filter(…).length` — an O(n) scan. Used three times on a page
/// that is three scans of the list per render, for one number. The cheap thing is a `useMemo`.
///
/// Only aggregates qualify: `{count}` lowers to `count`, and hoisting that would add a hook to save
/// nothing.
#[test]
fn a_repeated_aggregate_is_computed_once() {
    let src = "page P\ntype T {id, done:bool}\ndata tasks:T[] GET /api/t\nhead {tasks.open.count} open\nmetric {tasks.open.count}\np Still {tasks.open.count} to go\nlist tasks\n  text {id}\n";
    let out = react(src);

    assert!(
        out.contains(
            "const tasksOpenCount = useMemo(() => tasks.filter((it) => !it.done).length, [tasks])"
        ),
        "no memo was hoisted:\n{out}"
    );
    // One scan, not three.
    assert_eq!(
        out.matches("tasks.filter((it) => !it.done).length").count(),
        1,
        "the aggregate is still computed more than once:\n{out}"
    );
    // And every use site reads the memo.
    assert_eq!(out.matches("{tasksOpenCount}").count(), 3, "{out}");
}

#[test]
fn a_single_use_is_not_hoisted() {
    // A memo for one use is a hook and a dependency array bought for nothing.
    let out = react(
        "page P\ntype T {id, done:bool}\ndata tasks:T[] GET /api/t\nmetric {tasks.open.count}\nlist tasks\n  text {id}\n",
    );
    assert!(!out.contains("useMemo(() => tasks.filter"), "a single use was hoisted:\n{out}");
    assert!(out.contains("tasks.filter((it) => !it.done).length"), "{out}");
}

#[test]
fn a_cheap_expression_is_never_hoisted() {
    // `{count}` used four times lowers to `count` four times, which costs nothing.
    let out =
        react("page P\nstate count=0\nhead {count}\nmetric {count}\np a {count}\np b {count}\n");
    assert!(!out.contains("useMemo"), "a plain state read should not be memoised:\n{out}");
}

#[test]
fn a_row_scoped_expression_is_left_inside_the_repeater() {
    // Inside a repeater the value depends on `item`, so hoisting it above the map would not be an
    // optimisation — it would be wrong.
    let src = "page P\ntype T {id, tags}\ndata rows:T[] GET /api/r\nlist rows\n  text {tags.count}\n  text {tags.count}\n";
    let out = react(src);
    assert!(!out.contains("useMemo(() => item"), "a row value was hoisted out of its row:\n{out}");
}

#[test]
fn the_memo_name_is_derived_from_the_expression() {
    // Deterministic, so the emitted name does not move when an unrelated part of the document changes.
    let src = "page P\ntype T {id, done:bool}\ndata tasks:T[] GET /api/t\nhead {tasks.done.count}\nmetric {tasks.done.count}\nlist tasks\n  text {id}\n";
    let out = react(src);
    assert!(out.contains("const tasksDoneCount = useMemo"), "{out}");
}
