//! Compile latency.
//!
//! `CLAUDE.md` commits to `check` under 2 ms and `build` under 10 ms on a 200-line document,
//! and until now that was an assertion rather than a measurement. It is a real budget for two
//! reasons: the LSP calls `check` on a keystroke, and the repair loop calls it between model
//! rounds where it competes with a network request.
//!
//! The 200-line document is generated rather than hand-written so the size is exact, and it
//! is built from the same constructs the fixtures use — resources with mutations, repeaters,
//! bound attributes, prose — because a document of 200 `p` lines would measure the lexer and
//! nothing else.

use criterion::{Criterion, criterion_group, criterion_main};
use guml_compiler::{Options, check, compile};
use std::hint::black_box;

/// A document of roughly `lines` lines, structured like real GUML.
fn synth(lines: usize) -> String {
    let mut out = String::from("page Bench\n\n");
    out.push_str("type Row {id, title, note, area, amount:number, done:bool}\n");
    out.push_str("data rows:Row[] GET /api/rows\n");
    out.push_str("  add  POST   /api/rows      {title} optimistic:prepend\n");
    out.push_str("  save PATCH  /api/rows/{id} {done}  optimistic\n");
    out.push_str("  drop DELETE /api/rows/{id}         optimistic\n\n");
    out.push_str("state draft=\"\"\n");
    out.push_str("state filter=all|open|done\n\n");
    out.push_str("head Rows — {rows.open.count} open\n\n");
    out.push_str("form >rows.add{title:draft}; draft=\"\"\n");
    out.push_str("  input draft aria=\"New row\" placeholder=\"Add a row…\"\n");
    out.push_str("  btn Add primary disabled={!draft.trim()} busy=\"Adding…\"\n\n");
    out.push_str("tabs filter\n\n");
    out.push_str("list rows where={filter}\n");
    out.push_str("  check {done} >rows.save\n");
    out.push_str("  text {title} strike={done}\n");
    out.push_str("  btn Delete quiet aria=\"Delete {title}\" >rows.drop\n");
    out.push_str("  empty Nothing here yet.\n\n");

    // Pad with sections until the line count is reached: four lines per section.
    let mut n = 0;
    while out.lines().count() < lines {
        n += 1;
        out.push_str(&format!("section #s{n} Section {n} cols=3\n"));
        out.push_str(&format!("  card \"Card {n}a\" | Prose that is taken verbatim by the lexer, never quoted or escaped.\n"));
        out.push_str(&format!("  card \"Card {n}b\" | A second card so the section has the width its `cols` promises.\n"));
        out.push_str(&format!(
            "  p Body copy for section {n}, which the parser hands through as content.\n\n"
        ));
    }
    out
}

fn latency(c: &mut Criterion) {
    let small = synth(50);
    let target = synth(200);
    let large = synth(1000);

    // Sanity: a benchmark of a document that does not compile measures the error path.
    for (name, src) in [("50", &small), ("200", &target), ("1000", &large)] {
        let (_, d) = check(src);
        assert!(!d.has_errors(), "{name}-line bench document must be valid: {:?}", d.items);
    }

    let mut group = c.benchmark_group("check");
    group.bench_function("50 lines", |b| b.iter(|| check(black_box(&small))));
    // The budget: < 2 ms.
    group.bench_function("200 lines", |b| b.iter(|| check(black_box(&target))));
    group.bench_function("1000 lines", |b| b.iter(|| check(black_box(&large))));
    group.finish();

    // Measured in the same run as `check`, deliberately. Absolute timings on a laptop drift with
    // thermal state — during development a run that did *twice* the work measured faster than the
    // single-work run before it — so the defensible number is this pass's share of a `check` timed
    // under identical conditions, not either figure on its own.
    let mut group = c.benchmark_group("analysis");
    group.bench_function("referenced_names 200 lines", |b| {
        let (program, _) = check(&target);
        b.iter(|| guml_ast::referenced_names(black_box(&program)))
    });
    group.finish();

    let opts = Options::default();
    let mut group = c.benchmark_group("build");
    group.bench_function("50 lines", |b| b.iter(|| compile(black_box(&small), &opts)));
    // The budget: < 10 ms.
    group.bench_function("200 lines", |b| b.iter(|| compile(black_box(&target), &opts)));
    group.bench_function("1000 lines", |b| b.iter(|| compile(black_box(&large), &opts)));
    group.finish();
}

criterion_group!(benches, latency);
criterion_main!(benches);
