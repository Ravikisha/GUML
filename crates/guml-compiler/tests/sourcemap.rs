//! Source-map coverage: which GUML line an emitted line is attributed to.
//!
//! The map exists so a stack trace or an editor jump lands on the line the author wrote. Line
//! granularity only — one GUML line becomes a *region* of TSX, so a column claim would be invented.
//!
//! What is asserted here is **attribution**, not merely presence. A map that points every line of a
//! row template at the `list` twenty lines above it is still a valid Source Map v3 and still useless:
//! the debugger opens the right file at the wrong line, which is worse than no map, because it looks
//! authoritative.

use guml_codegen::Backend as _;
use guml_compiler::check;

/// The emitted text, and where each mapping *starts*.
///
/// `source_line_of` answers per emitted line, and a mapping is a range: every line inherits the last
/// mark at or before it. So "which line does this construct begin at" is the line where the resolved
/// source line *changes* — which is exactly where a mark was recorded, and the only position a
/// debugger jump can be judged against.
fn mapped(src: &str) -> (Vec<String>, Vec<(usize, u32)>) {
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "{:?}", diags.items);
    let out = guml_codegen::react::ReactBackend.emit(&program);
    let file = &out.files[0];
    let map = file.source_map.as_ref().expect("a source map");
    let lines: Vec<String> = file.contents.lines().map(str::to_string).collect();

    let mut starts = Vec::new();
    let mut previous = None;
    for i in 0..lines.len() {
        let resolved = map.source_line_of(i as u32);
        if let Some(line) = resolved
            && resolved != previous
        {
            starts.push((i, line));
        }
        previous = resolved;
    }
    (lines, starts)
}

/// First emitted line attributed to each GUML line.
fn starts_by_source(marks: &[(usize, u32)]) -> std::collections::HashMap<u32, usize> {
    let mut out = std::collections::HashMap::new();
    for (emitted, source) in marks {
        out.entry(*source).or_insert(*emitted);
    }
    out
}

const DOC: &str = r#"page Tasks

type Task {id, title, done:bool}
data tasks:Task[] GET /api/tasks
  save PATCH /api/tasks/{id} {done} optimistic

state filter=all|open|done

list tasks where={filter}
  check {done} >tasks.save
  text {title} strike={done}
  empty Nothing here yet.
"#;

/// Line numbers in `DOC`, so a failure names the construct rather than a magic number.
const L_DATA: u32 = 4;
const L_STATE: u32 = 7;
const L_LIST: u32 = 9;
const L_CHECK: u32 = 10;
const L_TEXT: u32 = 11;
const L_EMPTY: u32 = 12;

#[test]
fn nested_elements_map_to_their_own_line() {
    let (lines, marks) = mapped(DOC);
    let by_source = starts_by_source(&marks);

    for (line, label) in [(L_CHECK, "check"), (L_TEXT, "text"), (L_EMPTY, "empty")] {
        assert!(
            by_source.contains_key(&line),
            "`{label}` (guml line {line}) has no mapping; mapped lines: {:?}",
            marks.iter().map(|(_, l)| *l).collect::<Vec<_>>()
        );
    }

    // And each points at the code that construct actually produced.
    let at = |l: u32| lines[by_source[&l]].as_str();
    assert!(at(L_CHECK).contains("type=\"checkbox\""), "check → {:?}", at(L_CHECK));
    assert!(at(L_TEXT).contains("line-through"), "text → {:?}", at(L_TEXT));
    assert!(at(L_EMPTY).contains("Nothing here yet."), "empty → {:?}", at(L_EMPTY));
}

#[test]
fn a_row_template_is_not_attributed_to_the_repeater() {
    // The regression this guards: before nesting, everything inside `list` resolved to the `list`
    // line, so three different constructs shared one attribution.
    let (_, marks) = mapped(DOC);
    for line in [L_CHECK, L_TEXT, L_EMPTY] {
        assert!(marks.iter().any(|(_, l)| *l == line), "guml line {line} lost its mapping");
    }

    // The repeater's own scaffolding — the `<ul>`, the `.map(`, the closing tags — is reclaimed by
    // the `list` line after each child region, so `list` opens several ranges rather than one.
    let list_regions = marks.iter().filter(|(_, l)| *l == L_LIST).count();
    assert!(
        list_regions >= 2,
        "the `list` should reclaim its scaffolding, got {list_regions} region(s)"
    );
}

#[test]
fn declarations_still_map_to_their_directive() {
    let (lines, marks) = mapped(DOC);
    let by_source = starts_by_source(&marks);
    // A failed fetch should point at the `data` line, which is the mapping that matters most.
    assert!(
        lines[by_source[&L_DATA]].contains("resource `tasks`"),
        "{:?}",
        lines[by_source[&L_DATA]]
    );
    assert!(lines[by_source[&L_STATE]].contains("useState"), "{:?}", lines[by_source[&L_STATE]]);
}

#[test]
fn every_mapping_points_inside_both_files() {
    // A mapping past the end of either side makes an editor throw when it tries to follow it.
    let source_lines = DOC.lines().count() as u32;
    let (lines, marks) = mapped(DOC);
    for (emitted, source) in &marks {
        assert!(*emitted < lines.len(), "emitted line {emitted} is past the end of the file");
        assert!(
            *source >= 1 && *source <= source_lines,
            "guml line {source} is outside a {source_lines}-line document"
        );
    }
    assert!(
        marks.len() >= 6,
        "expected a mapping per declaration and element, got {}",
        marks.len()
    );
}

#[test]
fn an_escape_block_does_not_claim_a_line_it_did_not_emit() {
    // A `js` block is hoisted into the component body and emits nothing where it sits, so marking it
    // in the JSX would attribute the *next* element's line to the block.
    let (lines, marks) = mapped("page P\njs\n  const x = 1;\ncard Hi\n  p body\n");
    let by_source = starts_by_source(&marks);
    assert!(by_source.contains_key(&4), "the `card` lost its mapping: {marks:?}");
    assert!(lines[by_source[&4]].contains("<div"), "{:?}", lines[by_source[&4]]);
    // Line 2 is the `js` header; it produces no JSX, so it must not own a JSX line.
    if let Some(i) = by_source.get(&2) {
        panic!("the `js` header claimed emitted line {i}: {:?}", lines[*i]);
    }
}
