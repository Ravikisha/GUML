//! Runs the conformance suite in `spec/tests/`.
//!
//! # Why the files are the authority and this file is not
//!
//! Until the suite existed, the Rust implementation *was* the specification: the only way to know what
//! GUML meant was to read the parser. Two implementations have already disagreed inside this project —
//! the docs site's TypeScript highlighter against `guml highlight`, and an expression grammar that
//! existed twice — and both times the disagreement surfaced only because somebody thought to compare
//! them.
//!
//! `spec/tests/*.txt` inverts that. The cases are the specification, this runner checks the Rust
//! against them, and an implementation in another language can be checked against the same files
//! without reading a line of Rust. That is the difference between shipping a program and defining a
//! language.
//!
//! # The assertion that matters most
//!
//! Diagnostics are matched **exactly**: a case that expects `GUML0033` on line 3 fails if that is
//! missing, and also fails if anything else is reported. Expected-set equality is what stops a change
//! from quietly adding a warning to every document — the failure mode where a compiler becomes noisy
//! one commit at a time and nobody notices because every individual diagnostic looked reasonable.

use guml_ast::{Positional, Program};
use guml_codegen::Backend as _;
use guml_compiler::check_with;
use guml_registry::Registry;

struct Case {
    file: String,
    name: String,
    level: String,
    guml: String,
    ast: Option<String>,
    diagnostics: Option<String>,
    html: Option<String>,
}

fn parse_cases(file: &str, text: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    // `::::` opens a case and closes it, so a case body can contain anything else.
    let mut blocks = text.split("::::").skip(1);
    while let Some(header_and_body) = blocks.next() {
        let mut lines = header_and_body.lines();
        let name = lines.next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }

        let mut level = "app".to_string();
        let mut sections: Vec<(String, String)> = Vec::new();
        let mut current: Option<(String, String)> = None;
        for line in lines {
            if let Some(rest) = line.strip_prefix("--- ") {
                if let Some(section) = current.take() {
                    sections.push(section);
                }
                current = Some((rest.trim().to_string(), String::new()));
            } else if let Some(value) = line.strip_prefix("level:") {
                if current.is_none() {
                    level = value.trim().to_string();
                }
            } else if let Some((_, body)) = current.as_mut() {
                body.push_str(line);
                body.push('\n');
            }
        }
        if let Some(section) = current.take() {
            sections.push(section);
        }

        let take = |key: &str| sections.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
        cases.push(Case {
            file: file.to_string(),
            name,
            level,
            guml: take("guml").unwrap_or_default(),
            ast: take("ast"),
            diagnostics: take("diagnostics"),
            html: take("html"),
        });
        // Every second block is the text between a closing `::::` and the next opening one.
        let _ = blocks.next();
    }
    cases
}

/// A structural fingerprint: one line per declaration and element, indented by depth.
///
/// Deliberately not the serialised AST. A fingerprint stays readable in a diff and does not churn when
/// a field is added, which is what keeps these files reviewable by a person rather than regenerated
/// blindly whenever they break.
fn fingerprint(program: &Program) -> String {
    fn element(el: &guml_ast::Element, depth: usize, out: &mut String) {
        out.push_str(&"  ".repeat(depth));
        out.push_str(&el.tag);
        for p in &el.positionals {
            match p {
                Positional::Text(t) => out.push_str(&format!(" text={t:?}")),
                Positional::Binding(b) => out.push_str(&format!(" bind={{{}}}", b.source)),
                Positional::Route(r) => out.push_str(&format!(" route={r}")),
                Positional::Anchor(a) => out.push_str(&format!(" anchor=#{a}")),
                Positional::Modifier(m) => out.push_str(&format!(" mod={m}")),
            }
        }
        for a in &el.attrs {
            out.push_str(&format!(" {}=", a.name));
            // Every variant explicitly: `as_text()` returns `None` for a number, so a fallback would
            // render `cols=3` as `<flag>` and hide a real difference behind a placeholder.
            match &a.value {
                guml_ast::Value::Binding(b) => out.push_str(&format!("{{{}}}", b.source)),
                guml_ast::Value::Num(n) => out.push_str(&format!("{n}")),
                guml_ast::Value::Bool(b) => out.push_str(&format!("{b}")),
                guml_ast::Value::Str(t) => out.push_str(&format!("{t:?}")),
                guml_ast::Value::Word(w) => out.push_str(w),
                guml_ast::Value::Flag => out.push_str("<flag>"),
            }
        }
        for action in &el.actions {
            out.push_str(&format!(" action={action:?}"));
        }
        if let Some(c) = &el.content {
            out.push_str(&format!(" content={c:?}"));
        }
        for line in &el.text_lines {
            out.push_str(&format!(" line={line:?}"));
        }
        out.push('\n');
        for child in &el.children {
            element(child, depth + 1, out);
        }
    }

    let mut out = String::new();
    if let Some(p) = &program.page {
        out.push_str(&format!("page={}", p.name));
        for (key, value) in [
            ("title", &p.meta.title),
            ("description", &p.meta.description),
            ("lang", &p.meta.lang),
            ("dir", &p.meta.dir),
        ] {
            if let Some(v) = value {
                out.push_str(&format!(" {key}={v:?}"));
            }
        }
        out.push('\n');
    }
    for t in &program.types {
        let fields: Vec<String> = t.fields.iter().map(|f| format!("{}:{}", f.name, f.ty)).collect();
        out.push_str(&format!("type {} {{{}}}\n", t.name, fields.join(", ")));
    }
    for s in &program.states {
        out.push_str(&format!("state {}", s.name));
        if !s.domain.is_empty() {
            out.push_str(&format!(" domain={}", s.domain.join("|")));
        }
        out.push('\n');
    }
    for r in &program.resources {
        out.push_str(&format!("data {} {} {} {}", r.name, r.ty, r.method, r.url));
        for m in &r.mutations {
            out.push_str(&format!(" [{} {} {}]", m.name, m.method, m.url));
        }
        out.push('\n');
    }
    // Effects are part of the fingerprint, or an `on` case would pass by asserting nothing: the
    // declaration produces no element, so a tree-only fingerprint is identical with and without it.
    for e in &program.effects {
        let trigger = match &e.trigger {
            guml_ast::Trigger::Mount => "mount".to_string(),
            guml_ast::Trigger::Change(expr) => format!("{{{expr}}}"),
        };
        out.push_str(&format!("on {trigger} >{}\n", e.actions.join("; ")));
    }
    for el in &program.tree {
        element(el, 0, &mut out);
    }
    out
}

fn run(case: &Case) -> Vec<String> {
    let mut failures = Vec::new();
    let registry = match case.level.as_str() {
        "core" => Registry::core(),
        _ => Registry::builtin(),
    };
    let (program, diags) = check_with(&case.guml, &registry);

    if let Some(expected) = &case.ast {
        let got = fingerprint(&program);
        if got.trim() != expected.trim() {
            failures.push(format!(
                "ast mismatch\n--- expected\n{}\n--- got\n{}",
                expected.trim(),
                got.trim()
            ));
        }
    }

    if let Some(expected) = &case.diagnostics {
        let mut want: Vec<String> = expected
            .split_whitespace()
            .collect::<Vec<_>>()
            .chunks(2)
            .map(|c| c.join(" "))
            .collect();
        let mut got: Vec<String> =
            diags.items.iter().map(|d| format!("{} {}", d.id, d.span.line)).collect();
        want.sort();
        got.sort();
        if want != got {
            failures.push(format!(
                "diagnostics mismatch\n--- expected\n{}\n--- got\n{}",
                want.join("\n"),
                got.join("\n")
            ));
        }
    }

    if let Some(expected) = &case.html {
        let out = guml_codegen::html::HtmlBackend::default().emit(&program);
        let html = out.files.first().map(|f| f.contents.clone()).unwrap_or_default();
        for needle in expected.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if !html.contains(needle) {
                failures.push(format!("html does not contain {needle:?}"));
            }
        }
    }

    failures
}

#[test]
fn the_conformance_suite_passes() {
    let dir = std::path::Path::new("../../spec/tests");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("spec/tests exists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no conformance files found in {}", dir.display());

    let mut cases = Vec::new();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(path).expect("readable");
        cases.extend(parse_cases(&name, &text));
    }

    // A suite that silently parsed to zero cases would pass forever.
    assert!(cases.len() >= 25, "only {} cases parsed — the format is probably wrong", cases.len());

    let mut report = String::new();
    let mut failed = 0;
    for case in &cases {
        let failures = run(case);
        if !failures.is_empty() {
            failed += 1;
            report.push_str(&format!("\n=== {} :: {}\n", case.file, case.name));
            for f in failures {
                report.push_str(&f);
                report.push('\n');
            }
        }
    }

    assert!(failed == 0, "{failed} of {} conformance cases failed:{report}", cases.len());
    println!("{} conformance cases pass across {} files", cases.len(), files.len());
}
