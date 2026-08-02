//! End-to-end: real fixture source -> React output.
//!
//! Fixture A is the v0.1 vertical-slice gate (ROADMAP Phase 3). It is also one of the three
//! fixtures behind the token measurement in the research report, so keeping it compiling keeps
//! the headline claim honest.

use guml_compiler::{Options, compile};

const FIXTURE_A: &str = include_str!("../../../fixtures/a.guml");
const FIXTURE_B: &str = include_str!("../../../fixtures/b.guml");
const FIXTURE_C: &str = include_str!("../../../fixtures/c.guml");

#[test]
fn fixture_a_compiles_end_to_end() {
    let res = compile(FIXTURE_A, &Options::default());
    assert!(
        res.ok(),
        "fixture A must compile cleanly:\n{}",
        res.diagnostics.render(FIXTURE_A, "fixtures/a.guml")
    );

    let out = &res.files[0];
    assert_eq!(out.path, "Counter.tsx");
    let src = &out.contents;

    // Component shell
    assert!(src.contains("export default function Counter()"));
    assert!(src.contains("const [count, setCount] = useState(0);"));

    // All three actions lowered
    assert!(src.contains("setCount(count - 1)"));
    assert!(src.contains("setCount(count + 1)"));
    assert!(src.contains("setCount(0)"));

    // Content preserved verbatim
    assert!(src.contains("Press the buttons to change the value."));
    assert!(src.contains(">Clicks<"));

    // Binding reached JSX
    assert!(src.contains("{count}"));

    // Conditional disable came from `disabled={!count}`
    assert!(src.contains("disabled={!count}"));

    // Design system was supplied by the compiler, not the source
    assert!(!FIXTURE_A.contains("rounded"), "GUML source must contain no utility classes");
    assert!(src.contains("rounded-md"), "emitted code carries the design system");
}

#[test]
fn fixture_a_output_is_larger_than_its_source() {
    // The whole thesis in one assertion: the representation the model writes is far smaller
    // than the artifact it stands for.
    let res = compile(FIXTURE_A, &Options::default());
    let ratio = res.stats.ratio();
    assert!(
        ratio > 3.0,
        "expected emitted/source token ratio > 3, got {ratio:.2} \
         (source ~{} tokens, emitted ~{} tokens)",
        res.stats.approx_source_tokens,
        res.stats.approx_emitted_tokens
    );
}

#[test]
fn fixtures_b_and_c_parse_without_errors() {
    // Phase 2 gate: the front end handles the full fixture set even though the v0.1 React
    // backend cannot lower resources or repeaters yet. Those gaps must surface as warnings,
    // never as silent wrong output.
    for (name, src) in [("b", FIXTURE_B), ("c", FIXTURE_C)] {
        let (_, diags) = guml_compiler::check(src);
        assert!(
            !diags.has_errors(),
            "fixture {name} should parse cleanly:\n{}",
            diags.render(src, &format!("fixtures/{name}.guml"))
        );
    }
}

#[test]
fn the_task_fixture_compiles_with_no_diagnostics_at_all() {
    // This test used to assert the opposite: that fixture B *must* produce a warning, because the v0.1
    // React backend could not lower resources. That gap closed, and the only warning left was a real
    // accessibility defect — an input named only by its placeholder, which the paired React and JSON
    // representations shared. Fixing all three closed the Phase 3 gate item, and inverted this test.
    let res = compile(FIXTURE_B, &Options::default());
    assert!(
        res.diagnostics.items.is_empty(),
        "fixture B should be clean:
{}",
        res.diagnostics.render(FIXTURE_B, "fixtures/b.guml")
    );
    assert!(!res.files.is_empty(), "and it should still emit");
}

#[test]
fn a_genuine_lowering_gap_warns_rather_than_miscompiling() {
    // The invariant the old test was reaching for, against a construct that really is unsupported: an
    // action in the no-JavaScript backend. Reported, marked in the output, and not silently dropped.
    let res = compile(FIXTURE_B, &Options { backend: "html".to_string(), ..Default::default() });
    assert!(
        res.diagnostics.items.iter().any(|d| d.id == "GUML0030"),
        "the gap must be reported: {:?}",
        res.diagnostics.items
    );
    assert!(
        !res.diagnostics.has_errors(),
        "a gap is a warning, so the rest of the pipeline still runs"
    );
    let html = &res.files[0].contents;
    assert!(html.contains("data-guml-inert"), "the gap must be visible in the output too");
}
