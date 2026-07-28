#!/usr/bin/env node
/**
 * Harness preflight. Runs without an API key.
 *
 * Everything checked here is a way the study could be silently invalid rather than
 * merely broken: an example that is also an answer, a registry slice missing a tag
 * the task needs, a prompt over the 3,000-token budget the spec commits to, a
 * reference that does not compile. A benchmark that fails any of these produces
 * numbers, which is worse than producing an error.
 *
 *   node preflight.mjs
 */
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { BENCH, EXAMPLE_ORDER, ROOT, buildPrompt, readExamples, registrySlice } from "./lib/prompt.mjs";
import { TASKS } from "./tasks/index.mjs";

const problems = [];
const notes = [];
const fail = (m) => problems.push(m);

/* 1. Task set shape ------------------------------------------------------- */

if (TASKS.length !== 10) fail(`expected 10 tasks, found ${TASKS.length}`);
const ids = new Set();
for (const t of TASKS) {
  if (ids.has(t.id)) fail(`duplicate task id: ${t.id}`);
  ids.add(t.id);
  if (t.checklist.length < 10) fail(`${t.id}: only ${t.checklist.length} checklist items`);
  if (!existsSync(join(BENCH, "references", t.reference))) fail(`${t.id}: missing reference ${t.reference}`);
  if (t.prompt.includes("GUML")) fail(`${t.id}: the task prompt names GUML — it must be arm-neutral`);
}
const byCategory = TASKS.reduce((acc, t) => ({ ...acc, [t.category]: (acc[t.category] ?? 0) + 1 }), {});
notes.push(`categories: ${Object.entries(byCategory).map(([k, v]) => `${k} ${v}`).join(", ")}`);

/* 2. Examples: valid, and never an answer -------------------------------- */

for (const name of EXAMPLE_ORDER) {
  const file = join(BENCH, "examples", name);
  if (!existsSync(file)) {
    fail(`missing example ${name}`);
    continue;
  }
  try {
    execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", "check", file], {
      cwd: ROOT,
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (e) {
    fail(`example ${name} does not compile cleanly:\n${e.stdout?.toString() ?? e.message}`);
  }
}

// Leakage: an example that is also a task's fixture would put the answer in context.
const fixtureExamples = TASKS.filter((t) => t.fixture).map((t) => `fixtures/${t.fixture}.guml`);
const exampleBodies = readExamples(EXAMPLE_ORDER.length).map((e) => e.source.trim());
for (const path of fixtureExamples) {
  const body = readFileSync(join(ROOT, path), "utf8").trim();
  if (exampleBodies.includes(body)) fail(`leakage: ${path} is both a task answer and an in-context example`);
}
notes.push(`${EXAMPLE_ORDER.length} examples, none of which is a task answer`);

/* 3. Registry slices actually contain the task's tags -------------------- */

const fullRegistry = registrySlice(null, { full: true });
const knownTags = new Set(
  fullRegistry
    .split("\n")
    .map((l) => l.trim().split(/\s+/)[0])
    .filter((w) => w && !w.includes(":")),
);
for (const t of TASKS) {
  const tags = t.tags.split(",").map((s) => s.trim());
  for (const tag of tags) {
    if (!knownTags.has(tag)) fail(`${t.id}: tag \`${tag}\` is not in the registry`);
  }
  const slice = registrySlice(t.tags);
  for (const tag of tags) {
    if (!new RegExp(`^${tag}\\s`, "m").test(slice)) fail(`${t.id}: slice omits \`${tag}\``);
  }
}

/* 4. Prompt budget ------------------------------------------------------- */

/**
 * A ~3.6 chars/token heuristic, matching `guml tokens`. It is an estimate and is
 * labelled as one: the real figure comes from `count_tokens` during the run and is
 * what goes in any write-up.
 */
const estTokens = (s) => Math.round(s.length / 3.6);
const BUDGET = 3000;

let worst = { tokens: 0 };
for (const t of TASKS) {
  for (const examples of [0, 3]) {
    const p = buildPrompt({ task: t, arm: "guml", examples });
    const tokens = estTokens(p.system.map((s) => s.text).join("\n") + p.user);
    if (tokens > worst.tokens) worst = { tokens, task: t.id, examples };
  }
}
if (worst.tokens > BUDGET) {
  fail(
    `prompt budget: ${worst.task} at ${worst.examples} examples is ~${worst.tokens} estimated tokens, over the ${BUDGET} the spec commits to`,
  );
} else {
  notes.push(`largest prompt ~${worst.tokens} est. tokens (${worst.task}, ${worst.examples} examples), budget ${BUDGET}`);
}

/* 5. References typecheck under --strict --------------------------------- */

try {
  execFileSync("node", ["node_modules/typescript/bin/tsc", "-p", "../bench/phase0/tsconfig.json"], {
    cwd: join(ROOT, "docs"),
    stdio: ["ignore", "pipe", "pipe"],
  });
  notes.push(`${TASKS.length} React references typecheck under --strict`);
} catch (e) {
  fail(`React references do not typecheck:\n${e.stdout?.toString() ?? e.message}`);
}

/* ------------------------------------------------------------------------ */

for (const n of notes) console.log(`  · ${n}`);
if (problems.length > 0) {
  console.error(`\n${problems.length} problem(s):`);
  for (const p of problems) console.error(`  ✗ ${p}`);
  process.exit(1);
}
console.log("\npreflight clean — the harness is runnable");
