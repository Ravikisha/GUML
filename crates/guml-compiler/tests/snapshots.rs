//! Whole-output snapshots of every fixture.
//!
//! # Why these exist alongside `desugar.rs`
//!
//! `desugar.rs` asserts that specific strings appear — `setTasks(snapshot)` three times, a
//! `finally` that clears the pending flag. Those tests state *why* something must be true, which
//! is worth keeping and worth reading.
//!
//! What they cannot do is notice a change nobody predicted. Every codegen bug found in this
//! project so far was found by typechecking the output or by reading it, never by an assertion
//! written in advance: the wrong `where=` filter, `.live.length`, `kind` as a DOM prop, an
//! `aria-label` with literal braces. A snapshot fails on all four the moment they appear.
//!
//! So the division is deliberate: named assertions for the invariants, snapshots for everything
//! else. Reviewing a snapshot diff is the point, not a chore to bypass — `cargo insta review`.

use guml_compiler::{Options, compile};

fn emitted(name: &str, backend: &str) -> String {
    let src = std::fs::read_to_string(format!("../../fixtures/{name}")).expect("fixture");
    let out = compile(&src, &Options { backend: backend.to_string(), ..Default::default() });
    assert!(
        !out.diagnostics.has_errors(),
        "{name} must compile cleanly for its snapshot to mean anything: {:?}",
        out.diagnostics.items
    );
    out.files
        .iter()
        .map(|f| format!("// ---- {} ----\n{}", f.path, f.contents))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn counter_react() {
    insta::assert_snapshot!(emitted("a.guml", "react"));
}

#[test]
fn tasks_react() {
    insta::assert_snapshot!(emitted("b.guml", "react"));
}

#[test]
fn landing_react() {
    insta::assert_snapshot!(emitted("c.guml", "react"));
}

#[test]
fn portfolio_react() {
    insta::assert_snapshot!(emitted("portfolio.guml", "react"));
}

/// The render tree the browser runtime consumes. A change here moves the live preview and the
/// playground, not just the emitted file.
#[test]
fn tasks_json_tree() {
    insta::assert_snapshot!(emitted("b.guml", "json"));
}

/// Line provenance, as a table rather than a base64 blob: a VLQ diff is unreviewable, and the
/// point of a snapshot is that a human can see what moved.
#[test]
fn tasks_source_map() {
    let src = std::fs::read_to_string("../../fixtures/b.guml").expect("fixture");
    let out = compile(&src, &Options::default());
    let file = &out.files[0];
    let map = file.source_map.as_ref().expect("the React backend records provenance");

    let emitted: Vec<&str> = file.contents.lines().collect();
    let source: Vec<&str> = src.lines().collect();

    let mut table = String::new();
    let mut last = None;
    for (i, _) in emitted.iter().enumerate() {
        let Some(origin) = map.source_line_of(i as u32) else { continue };
        // One row per *change* of origin: a row per emitted line would be 160 lines of noise.
        if last == Some(origin) {
            continue;
        }
        last = Some(origin);
        table.push_str(&format!(
            "tsx {:>3}  <-  guml {:>2}  {}\n",
            i + 1,
            origin,
            source.get(origin as usize - 1).unwrap_or(&"").trim()
        ));
    }
    insta::assert_snapshot!(table);
}
