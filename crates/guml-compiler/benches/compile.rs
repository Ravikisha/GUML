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
//!
//! # Why there is a calibration benchmark
//!
//! Absolute milliseconds on a developer laptop are not a measurement. On the machine this was
//! developed on, criterion reported a **100% regression** on `referenced_names` — a function that had
//! not been touched between the two runs — and on another occasion reported a build doing strictly
//! *more* work as 22% faster. Thermal state and background load move the numbers by ~2×, which is
//! larger than any regression worth catching.
//!
//! So the budget is expressed as a **ratio** against a fixed reference workload measured in the same
//! run. `calibration/reference` does a known amount of pure-Rust work that has nothing to do with the
//! compiler; whatever slows the machine down slows both, so the ratio survives what the absolutes do
//! not. `just latency` prints it.
//!
//! The absolute figures are still reported, because they are what a user experiences — they are simply
//! not what a regression should be judged on.

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
        // Quoted title: a container reads one positional slot, so `Section 1` as two bare words is
        // `GUML0099`. The bench document has to be valid — it asserts that below — because timing the
        // error path would measure diagnostic construction rather than compilation.
        out.push_str(&format!("section #s{n} \"Section {n}\" cols=3\n"));
        out.push_str(&format!("  card \"Card {n}a\" | Prose that is taken verbatim by the lexer, never quoted or escaped.\n"));
        out.push_str(&format!("  card \"Card {n}b\" | A second card so the section has the width its `cols` promises.\n"));
        out.push_str(&format!(
            "  p Body copy for section {n}, which the parser hands through as content.\n\n"
        ));
    }
    out
}

/// A fixed amount of pure-Rust work: string building, hashing, and allocation, in roughly the
/// proportions the compiler does them.
///
/// Deliberately *not* a compiler function. If the reference shared code with `check`, a change to that
/// code would move both sides and the ratio would hide exactly the regression it exists to catch.
fn reference_work() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut acc: u64 = 0;
    for i in 0..2_000u64 {
        let s = format!("line {i} of a synthetic document with some prose on it");
        let mut h = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut h);
        acc = acc.wrapping_add(h.finish());
        // A little allocation churn, which is what the compiler's hot path actually costs.
        let parts: Vec<&str> = s.split_whitespace().collect();
        acc = acc.wrapping_add(parts.len() as u64);
    }
    acc
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

    // Measured first, in the same run, so the ratio below is taken under the same machine conditions.
    let mut group = c.benchmark_group("calibration");
    group.bench_function("reference", |b| b.iter(|| black_box(reference_work())));
    group.finish();

    // Per stage, so a regression can be attributed instead of argued about. All four measured in the
    // same run as the calibration above.
    let reg = guml_registry::Registry::builtin();
    let mut group = c.benchmark_group("stage");
    group.bench_function("lex", |b| b.iter(|| guml_syntax::lex(black_box(&target))));
    group.bench_function("parse", |b| {
        b.iter(|| guml_parser::parse(black_box(&target), black_box(&reg)))
    });
    group.bench_function("analyse", |b| {
        // Everything `check` does after parsing: expansion, resolution, validation, inference.
        let parsed = guml_parser::parse(&target, &reg);
        b.iter(|| {
            let mut program = parsed.program.clone();
            let mut diags = guml_diagnostics::Diagnostics::default();
            guml_compiler::analyse_for_bench(&mut program, &reg, &mut diags);
            black_box(diags)
        })
    });
    group.finish();

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
