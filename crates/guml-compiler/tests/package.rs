//! The example registry package in `packages/guml-widgets`, checked from Rust.
//!
//! The Node side (`pnpm typecheck:example`) proves the emitted props match the components' types. This proves
//! the compiler half without needing Node at all, so `cargo test` alone catches a regression:
//!
//! * the package audits with no errors
//! * its example compiles against it with no diagnostics
//! * every declared attribute reaches the emitted props, and the import is generated
//!
//! The third is the one that earned this file. A package declares `attrs`; the React backend's attribute loop
//! encodes what each name means *for a builtin*, and applying that to someone else's component silently
//! dropped two of them — `of` because a repeater uses it, `kind` because it folds into an `<input>`'s `type`.

use guml_registry::Registry;

fn package_json() -> String {
    std::fs::read_to_string("../../packages/guml-widgets/guml.registry.json").expect("the package")
}

fn compile_example() -> String {
    let src = std::fs::read_to_string("../../packages/guml-widgets/example.guml").expect("example");
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

    // The registry has to be the *active* one for codegen too, or `custom_element` cannot see the entries.
    let _ = guml_codegen::set_registry(reg);
    let out = guml_codegen::backend("react").unwrap().emit(&program);
    out.files[0].contents.clone()
}

#[test]
fn the_package_audits_with_no_errors() {
    let audit = Registry::audit_package(&package_json());
    assert!(audit.ok(), "{:?}", audit.errors);
    assert_eq!(audit.name.as_deref(), Some("@guml/widgets"));
    assert_eq!(audit.components.len(), 5, "{:?}", audit.components);
    // The three `requires_label` containers are warned about, which is the audit telling the truth rather
    // than a defect: a container's accessible name comes from a title positional that is easy to omit.
    assert_eq!(audit.warnings.len(), 3, "{:?}", audit.warnings);
}

#[test]
fn every_declared_attribute_reaches_the_emitted_props() {
    let out = compile_example();

    // The import, generated for exactly the tags the document uses.
    assert!(
        out.contains("from \"@guml/widgets\""),
        "no import for the package's components:\n{out}"
    );
    for component in ["Chart", "Calendar", "DateField", "Upload", "CommandMenu"] {
        assert!(out.contains(&format!("<{component}")), "{component} not emitted:\n{out}");
    }

    // The regression: `of` and `kind` were dropped, so the chart plotted nothing and nothing said so.
    for prop in ["rows={points}", "of=\"revenue\"", "label=\"month\"", "kind=\"line\""] {
        assert!(out.contains(prop), "declared prop {prop} was dropped:\n{out}");
    }

    // `requires_label` means the title positional *is* the accessible name. It used to be emitted as
    // children, so a component whose entry demands a name got none — the compiler enforcing the contract on
    // the document and then dropping it on the way out.
    assert!(out.contains("aria-label=\"Revenue by month\""), "{out}");

    // A `field`-kind component gets the two-way binding its kind promises. Only `input` and `select` were
    // wired, so `date from` emitted the state *name* as children and the control was decorative.
    assert!(out.contains("value={from}") && out.contains("onChange={setFrom}"), "{out}");

    // `if=` is the compiler's conditional, not a prop. Forwarding it as well put an unknown attribute on
    // someone else's component *and* guarded the same subtree twice.
    assert!(out.contains("{palette && ("), "the conditional was not applied:\n{out}");
    assert!(!out.contains("if={palette}"), "`if=` leaked through as a prop:\n{out}");
}
