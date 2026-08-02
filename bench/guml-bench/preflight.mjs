#!/usr/bin/env node
/**
 * GUML-Bench preflight: is this benchmark runnable, and is it honest about its own size?
 *
 * The Phase 0 harness has a preflight for a reason — a benchmark that fails halfway through a paid sweep
 * has wasted the sweep — and this one adds a second job that matters more. The report specifies 150 tasks
 * across 6 categories and 9 arms. The dataset is currently a fraction of that, which is fine and normal
 * while it is being built, and the way it stops being fine is somebody quoting a category figure computed
 * over two tasks as though it were twenty-five.
 *
 * So this prints the coverage, prints which arms cannot run and why, and **exits non-zero if anything in
 * the dataset is malformed** — including a task that tries to give different arms different prompts, which
 * is the one defect that would make a published number wrong rather than merely thin.
 *
 * Run from the repository root:
 *
 *   node bench/guml-bench/preflight.mjs
 */
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { ARMS, MODELS, armStatus, coverage, reviewTask, validateTask } from "./schema.mjs";
import { TASKS } from "./tasks.mjs";
import { approxTokens } from "./metrics.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..");

const problems = [];
const notes = [];

/* ------------------------------------------------------------------ dataset */

const seen = new Set();
for (const task of TASKS) {
  for (const p of validateTask(task, seen)) {
    problems.push(`${task.id ?? "<no id>"}: ${p}`);
  }
  for (const n of reviewTask(task)) {
    notes.push(`${task.id}: ${n}`);
  }
  if (task.id) seen.add(task.id);
}
notes.push(`${TASKS.length} tasks, ${seen.size} distinct ids`);

const cover = coverage(TASKS);
const total = cover.reduce((a, c) => a + c.have, 0);
const target = cover.reduce((a, c) => a + c.target, 0);

// Every category needs at least one task, or the harness cannot exercise it at all.
for (const c of cover) {
  if (c.have === 0) problems.push(`category \`${c.id}\` has no tasks`);
}

/* --------------------------------------------------------------------- arms */

const { available, unavailable } = armStatus();
if (available.length === 0) problems.push("no arm can run");
notes.push(`${available.length} of ${ARMS.length} arms runnable`);

// Every runnable arm whose target is a compiler backend must be a backend the compiler actually has.
// Naming an arm the compiler cannot emit would produce a column of nulls labelled as a comparison.
let backends = [];
try {
  // From `--help`, whose `[possible values: …]` list clap generates from the compiler's own
  // `backend_names()`. Parsing the help text rather than hardcoding a list here: a second list is a
  // second answer waiting to disagree, and this preflight exists to catch exactly that class of drift.
  const help = execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", "build", "--help"], {
    cwd: ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  backends = help.match(/possible values: ([^\]]+)\]/)?.[1]?.split(",").map((s) => s.trim()) ?? [];
} catch (err) {
  problems.push(`could not ask the compiler which backends exist: ${err.message}`);
}
if (backends.length === 0) {
  // A silent empty list would report every arm as needing a missing backend, which is a preflight that
  // fails loudly for the wrong reason — worse than one that fails quietly for the right one.
  problems.push("the compiler reported no backends; the `--help` format has changed");
}

const COMPILER_ARMS = { B1: "react", B2: "html", B3: "a2ui", T1: "react", T3: "react" };
for (const [arm, backend] of Object.entries(COMPILER_ARMS)) {
  if (!ARMS.find((a) => a.id === arm)?.available) continue;
  if (!backends.includes(backend)) {
    problems.push(`arm ${arm} needs the \`${backend}\` backend, which the compiler does not have`);
  }
}

/* ---------------------------------------------------------------- references */

// Every task must have an authored GUML answer on disk. Fatal rather than advisory: a task with no
// reference is silently dropped from `report`, which then measures a subset of the dataset while the
// coverage line above still counts the whole thing. That is the exact shape of a thin result presented as
// a full one, so it fails here instead.
const { existsSync } = await import("node:fs");
for (const task of TASKS) {
  if (!task.reference) {
    problems.push(`${task.id}: no \`reference\`, so \`report\` cannot measure it`);
    continue;
  }
  if (!existsSync(join(ROOT, task.reference))) {
    problems.push(`${task.id}: reference \`${task.reference}\` does not exist`);
  }
}

/* -------------------------------------------------------------------- output */

console.log("GUML-Bench preflight\n");

console.log("Coverage against the report's 6 × 25:");
for (const c of cover) {
  const bar = c.have >= c.target ? "✓" : `${c.have}/${c.target}`;
  console.log(`  ${c.id.padEnd(11)} ${String(bar).padStart(6)}   ${c.expectation}`);
}
console.log(`  ${"total".padEnd(11)} ${`${total}/${target}`.padStart(6)}`);

console.log("\nArms:");
for (const a of available) {
  console.log(`  ✓ ${a.id.padEnd(3)} ${a.title}`);
}
for (const a of unavailable) {
  console.log(`  · ${a.id.padEnd(3)} ${a.title} — ${a.unavailable}`);
}

console.log(`\nModels: ${MODELS.map((m) => m.label).join(", ")}`);
console.log(
  `Planned runs at full coverage: ${target} tasks × ${available.length} arms × ${MODELS.length} models = ${target * available.length * MODELS.length}`,
);
console.log(
  `Planned runs at current coverage: ${total} × ${available.length} × ${MODELS.length} = ${total * available.length * MODELS.length}`,
);

// The prompt cost per task, so the sweep's size is knowable before it is paid for.
const promptTokens = TASKS.map((t) => approxTokens(t.prompt));
console.log(
  `Task prompts: ~${Math.min(...promptTokens)}–${Math.max(...promptTokens)} est. tokens each`,
);

console.log(`\n${notes.join("\n")}`);

if (problems.length > 0) {
  console.error(`\n${problems.length} problem(s):`);
  for (const p of problems) console.error(`  ✗ ${p}`);
  process.exit(1);
}

// The line that matters most, and it is deliberately not congratulatory.
console.log(
  `\npreflight clean — the harness is runnable. The dataset is ${total} of ${target} tasks:`,
);
console.log(
  "a per-category figure from this set is not publishable, and `report.mjs` prints the count beside",
);
console.log("every number for that reason.");
