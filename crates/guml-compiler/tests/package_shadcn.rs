//! `packages/guml-shadcn` — every shadcn/ui component GUML has no builtin for, checked from Rust.
//!
//! Its own file rather than a case in `package.rs` because `guml_codegen::set_registry` is process-global:
//! two packages loaded in one test binary would race over which vocabulary is active, and each `tests/*.rs`
//! is a separate binary.
//!
//! The Node side (`pnpm typecheck:example`) proves the emitted props typecheck against the real components.
//! This is the half that needs no Node, so `cargo test` alone catches a regression.
//!
//! The `options` case is the one that earned the file. A `field`-kind host component was emitted with
//! `value`/`onChange` and nothing else, so `radio` and `combobox` produced a control bound correctly to a
//! state and offering the reader no way to change it — a working binding to an empty box. The typecheck gate
//! could not see it either: an empty `<RadioGroup>` is valid TypeScript.

use guml_registry::Registry;

fn package_json() -> String {
    std::fs::read_to_string("../../packages/guml-shadcn/guml.registry.json").expect("the package")
}

fn compile_example() -> String {
    let src = std::fs::read_to_string("../../packages/guml-shadcn/example.guml").expect("example");
    let reg = Registry::builtin()
        .extend_from_json(&package_json())
        .expect("the package loads on top of the builtins");
    let (program, diags) = guml_compiler::check_with(&src, &reg);
    let errors: Vec<_> = diags
        .items
        .iter()
        .filter(|d| d.severity == guml_diagnostics::Severity::Error)
        .map(|d| format!("{} {}", d.id, d.message))
        .collect();
    assert!(errors.is_empty(), "the example should compile: {errors:?}");

    let _ = guml_codegen::set_registry(reg);
    guml_codegen::backend("react").unwrap().emit(&program).files[0].contents.clone()
}

#[test]
fn the_package_audits_with_no_errors() {
    let audit = Registry::audit_package(&package_json());
    assert!(audit.ok(), "{:?}", audit.errors);
    assert_eq!(audit.name.as_deref(), Some("@guml/shadcn"));
    assert_eq!(audit.components.len(), 26, "{:?}", audit.components);
}

#[test]
fn no_entry_shadows_a_builtin() {
    // `GUML0092` already refuses this at load, but the failure would read as "the package is broken" rather
    // than "these two tags mean the same thing". shadcn has a `card` and so does GUML; the point of the
    // package is the components GUML has *no* word for, and a second spelling of `card` would split every
    // document's vocabulary in two.
    let builtin = Registry::builtin();
    let audit = Registry::audit_package(&package_json());
    for c in &audit.components {
        assert!(builtin.get(c).is_none(), "`{c}` is already a builtin tag");
    }
}

#[test]
fn a_bound_field_is_emitted_with_the_options_it_offers() {
    let out = compile_example();

    // `size` is declared with a domain, and nothing in the document repeats it — the compiler reconciles the
    // domain and any `option` children through the same function `select` uses, so the two spellings cannot
    // disagree about one element.
    assert!(
        out.contains(r#"options={["small", "medium", "large"]}"#),
        "the radio's options should reach the component:\n{out}"
    );

    // And the binding itself is still the uniform field contract every backend and every package rely on.
    assert!(out.contains("value={size} onChange={setSize}"), "{out}");
}

#[test]
fn option_children_are_not_rendered_twice() {
    // Consumed into the `options` prop above, so emitting them again would put a bare `<option>` inside a
    // component that never asked for one: the list would appear once as data the component draws and once as
    // stray markup beside it.
    let src = "state pick: one\n  domain: one, two\nradio pick\n  option one\n  option two\n";
    let reg = Registry::builtin().extend_from_json(&package_json()).expect("loads");
    let (program, _) = guml_compiler::check_with(src, &reg);
    let _ = guml_codegen::set_registry(reg);
    let out = guml_codegen::backend("react").unwrap().emit(&program).files[0].contents.clone();

    assert!(out.contains("options={"), "the options should be a prop:\n{out}");
    assert!(!out.contains("<option"), "and not also markup:\n{out}");
}

#[test]
fn every_element_the_package_names_is_imported_from_it() {
    // A tag whose `element` is not in the generated import is a `ReferenceError` at runtime, and the emitted
    // file is the only place the two are connected.
    let out = compile_example();
    let import = out.lines().find(|l| l.contains(r#"from "@guml/shadcn""#)).expect("an import");
    for element in ["Textarea", "RadioGroup", "Slider", "DatePicker", "Collapsible", "Kbd"] {
        assert!(import.contains(element), "`{element}` missing from `{import}`");
        assert!(out.contains(&format!("<{element}")), "`{element}` imported but never used");
    }
}
