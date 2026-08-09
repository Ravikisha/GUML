/// `--format json` must always emit valid JSON, including for a clean document.
///
/// It printed *nothing* for a document with no diagnostics. Empty output is not valid JSON, so every
/// consumer had to special-case it before parsing — and the case they most need to handle correctly
/// ("the document is fine") was the one the format failed to describe. The repair loop reads this.
#[test]
fn the_json_form_of_no_diagnostics_is_an_empty_array() {
    let empty = guml_diagnostics::Diagnostics::default();
    assert_eq!(empty.to_json().trim(), "[]");
    serde_json::from_str::<serde_json::Value>(&empty.to_json()).expect("must parse as JSON");
}
