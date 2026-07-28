#!/usr/bin/env node
/**
 * Scoring self-test. Runs without an API key.
 *
 * Synthesises generations with known properties — one clean, one with an unknown
 * tag, one wrapped in a markdown fence, one that declares an unsupported construct,
 * plus React-arm outputs — then asserts `score.mjs` classifies each correctly.
 *
 * The point is to find scoring bugs before spending money on 90 generations. A
 * harness that miscounts parse validity would produce a plausible, wrong answer to
 * the only question Phase 0 exists to ask.
 *
 *   node selftest.mjs
 */
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BENCH, ROOT } from "./lib/prompt.mjs";
import { taskById } from "./tasks/index.mjs";

const dir = mkdtempSync(join(tmpdir(), "guml-phase0-selftest-"));
const raw = join(dir, "raw");
mkdirSync(raw, { recursive: true });

const read = (p) => readFileSync(join(ROOT, p), "utf8");
const fixtureB = read("fixtures/b.guml");
const fixtureC = read("fixtures/c.guml");
const reference = readFileSync(join(BENCH, "references", "t01-crud.tsx"), "utf8");

/** `synthetic: true` so these can never be mistaken for real generations. */
function write(name, run) {
  writeFileSync(join(raw, `${name}.json`), JSON.stringify({ synthetic: true, id: name, ...run }, null, 2));
}

const usage = (out, cacheRead = 2800) => ({
  input_tokens: 40,
  output_tokens: out,
  cache_read_input_tokens: cacheRead,
  cache_creation_input_tokens: 0,
});

const CASES = {
  // Clean, parseable, structure-heavy.
  clean: {
    file: "t01-crud__guml__sonnet__ex3__r1",
    run: {
      task: "t01-crud",
      category: "structure",
      arm: "guml",
      modelAlias: "sonnet",
      examples: 3,
      repeat: 1,
      usage: usage(180),
      stopReason: "end_turn",
      latencyMs: 3000,
      output: fixtureB,
    },
    expect: { parseable: "true", fenced: "false", hatches: 0 },
  },
  // A markdown fence: strippable, but a rule violation that must be counted.
  fenced: {
    file: "t03-landing__guml__sonnet__ex3__r1",
    run: {
      task: "t03-landing",
      category: "content",
      arm: "guml",
      modelAlias: "sonnet",
      examples: 3,
      repeat: 1,
      usage: usage(380),
      stopReason: "end_turn",
      latencyMs: 6000,
      output: `\`\`\`guml\n${fixtureC.trim()}\n\`\`\``,
    },
    expect: { parseable: "true", fenced: "true", hatches: 0 },
  },
  // An unknown tag — the low-resource-DSL failure mode.
  invalid: {
    file: "t02-dashboard__guml__haiku__ex0__r1",
    run: {
      task: "t02-dashboard",
      category: "structure",
      arm: "guml",
      modelAlias: "haiku",
      examples: 0,
      repeat: 1,
      usage: usage(140),
      stopReason: "end_turn",
      latencyMs: 1800,
      output: "page Dash\n\ncrad Tickets\n  p Nothing yet.\n",
    },
    expect: { parseable: "false", fenced: "false", hatches: 0 },
  },
  // Declares what it could not express, as a GUML comment — free, and never a parse error.
  hatched: {
    file: "t07-filters__guml__opus__ex3__r1",
    run: {
      task: "t07-filters",
      category: "mixed",
      arm: "guml",
      modelAlias: "opus",
      examples: 3,
      repeat: 1,
      usage: usage(210),
      stopReason: "end_turn",
      latencyMs: 4200,
      output: `page Events\nstate view=all|web|ios\n\ntabs view\n\n// UNSUPPORTED: multi-select filter chips\n`,
    },
    expect: { parseable: "true", fenced: "false", hatches: 1 },
  },
  // React baseline for the same structure-heavy task.
  react: {
    file: "t01-crud__react__sonnet__ex0__r1",
    run: {
      task: "t01-crud",
      category: "structure",
      arm: "react",
      modelAlias: "sonnet",
      examples: 0,
      repeat: 1,
      usage: usage(1440, 0),
      stopReason: "end_turn",
      latencyMs: 24000,
      output: reference,
    },
    expect: { parseable: "", fenced: "", hatches: 0 },
  },
  // Truncated: its output-token count is censored and must be flagged.
  truncated: {
    file: "t04-docs__react__opus__ex0__r1",
    run: {
      task: "t04-docs",
      category: "content",
      arm: "react",
      modelAlias: "opus",
      examples: 0,
      repeat: 1,
      usage: usage(8000, 0),
      stopReason: "max_tokens",
      latencyMs: 61000,
      output: reference,
    },
    expect: { parseable: "", fenced: "", hatches: 0, truncated: "true" },
  },
};

for (const c of Object.values(CASES)) write(c.file, c.run);

execFileSync("node", [join(BENCH, "score.mjs"), "--results", dir], { cwd: BENCH, stdio: "inherit" });

/* ------------------------------------------------------------------ asserts */

const failures = [];
const csv = readFileSync(join(dir, "runs.csv"), "utf8").trim().split("\n");
const header = csv[0].split(",");
const rows = new Map(
  csv.slice(1).map((line) => {
    const cells = line.split(",");
    return [cells[0], Object.fromEntries(header.map((h, i) => [h, cells[i]]))];
  }),
);

for (const [name, c] of Object.entries(CASES)) {
  const row = rows.get(c.file);
  if (!row) {
    failures.push(`${name}: no row in runs.csv`);
    continue;
  }
  for (const [key, want] of Object.entries(c.expect)) {
    const got = key === "hatches" ? Number(row.hatches) : row[key];
    if (String(got) !== String(want)) failures.push(`${name}.${key}: expected ${want}, got ${got}`);
  }
}

const summary = readFileSync(join(dir, "summary.md"), "utf8");

// The React fixture is 1,440 output tokens against GUML's 180 → 8.0×.
if (!summary.includes("| structure | 3 | 180 | 1440 | 8.0 |")) {
  failures.push("summary: structure-heavy token row is wrong or missing");
}
// Sonnet at 3 examples: two runs, both parseable, one of them fenced.
if (!/\| sonnet \| 3 \| 2 \| 100% \| 50% \| 0% \| 0 \|/.test(summary)) {
  failures.push("summary: sonnet parse-validity row is wrong or missing");
}
if (!summary.includes("GUML0030")) failures.push("summary: unknown-tag diagnostic not reported");
if (!summary.includes("multi-select filter chips")) failures.push("summary: escape hatch not listed");
if (!summary.includes("Truncated generations present")) failures.push("summary: truncation not warned about");
if (!summary.includes("- [x] Median output-token reduction ≥3×")) {
  failures.push("summary: token gate should pass at 8.0×");
}
if (!summary.includes("scoresheet not filled in")) failures.push("summary: correctness should be unscored");

// The invalid generation must not be gradable; everything else must be.
const gradable = ["clean", "fenced", "hatched", "react", "truncated"].reduce(
  (n, k) => n + taskById(CASES[k].run.task).checklist.length,
  0,
);
const sheetRows = readFileSync(join(dir, "scoresheet.csv"), "utf8").trim().split("\n").length - 1;
if (sheetRows !== gradable) failures.push(`scoresheet: ${sheetRows} rows, expected ${gradable}`);
const keymap = JSON.parse(readFileSync(join(dir, "keymap.json"), "utf8"));
if (Object.values(keymap).some((v) => v.id === CASES.invalid.file)) {
  failures.push("scoresheet: an unparseable generation was put up for human scoring");
}
if (readFileSync(join(dir, "scoresheet.csv"), "utf8").includes("guml")) {
  failures.push("scoresheet: leaks the arm — scoring would not be blind");
}

/* ------------------------------------------- phase two: human score read-back */

/**
 * The correctness gate is the one that decides whether GUML is a reliability win or
 * only a cost one, and it is computed from a hand-filled CSV. Fill it synthetically
 * in both directions to prove the comparison is not wired backwards.
 */
function fillSheet(scoreFor) {
  const path = join(dir, "scoresheet.csv");
  const keymap = JSON.parse(readFileSync(join(dir, "keymap.json"), "utf8"));
  const lines = readFileSync(path, "utf8").trim().split("\n");
  const filled = [lines[0]];
  for (const line of lines.slice(1)) {
    const blindId = line.slice(0, line.indexOf(","));
    // Replace the score column rather than appending to it: filling twice must not
    // concatenate "1" and "0.4" into "10.4".
    const withoutScore = line.slice(0, line.lastIndexOf(",") + 1);
    filled.push(`${withoutScore}${scoreFor(keymap[blindId].arm)}`);
  }
  writeFileSync(path, `${filled.join("\n")}\n`);
}

function rescore() {
  execFileSync("node", [join(BENCH, "score.mjs"), "--results", dir], { cwd: BENCH, stdio: "pipe" });
  return readFileSync(join(dir, "summary.md"), "utf8");
}

fillSheet((arm) => (arm === "guml" ? 1 : 0.5));
let report = rescore();
if (!/\| GUML \| \d+ \| 1\.00 \|/.test(report)) failures.push("read-back: GUML mean should be 1.00");
if (!/\| React \| \d+ \| 0\.50 \|/.test(report)) failures.push("read-back: React mean should be 0.50");
if (!report.includes("- [x] Semantic correctness not worse")) {
  failures.push("read-back: correctness gate should pass when GUML scores higher");
}

// Re-running must not have wiped the scores that were just read.
if (readFileSync(join(dir, "scoresheet.csv"), "utf8").trim().split("\n")[1].endsWith(",")) {
  failures.push("read-back: re-running score.mjs clobbered the filled scoresheet");
}

fillSheet((arm) => (arm === "guml" ? 0.4 : 0.9));
report = rescore();
if (!report.includes("- [ ] Semantic correctness not worse")) {
  failures.push("read-back: correctness gate should fail when GUML scores lower");
}

/* ------------------------------------------------------------------- verdict */

console.log("");
if (failures.length > 0) {
  console.error(`${failures.length} scoring failure(s):`);
  for (const f of failures) console.error(`  ✗ ${f}`);
  console.error(`\nartifacts: ${dir}`);
  process.exit(1);
}
console.log(`scoring self-test passed (${Object.keys(CASES).length} synthetic runs)`);
console.log(`artifacts: ${dir}`);
