#!/usr/bin/env node
/**
 * The repair-round experiment.
 *
 * Question: of the five applications that failed to compile, how many are fixed by the free
 * deterministic layers, how many need a model round, and what survives both?
 *
 * The prediction being tested — from `FINDINGS.md` — is specific and falsifiable:
 *
 *  - **invented references** (`>order.ship` on nothing, a `data` with no path) should fall to
 *    one model round, because the diagnostic names the problem precisely;
 *  - **`option` under `select`** should survive it, because both an 8B and a 70B produce it
 *    unprompted and a prompt rule already failed to stop it. If a repair round fixes that, the
 *    vocabulary conclusion in FINDINGS.md is wrong and should be withdrawn.
 *
 *   node repair.mjs                      # score what is in out/
 *   node repair.mjs --out out-70b
 *   node repair.mjs --dry               # free layers only, no model call
 */
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { APPS } from "./apps.mjs";
import { diagnose, fixedBetween, repair } from "./lib/pipeline.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
process.env.GUML_ROOT = ROOT;

const flag = (name, fallback) => {
  const i = process.argv.indexOf(name);
  return i > -1 ? process.argv[i + 1] : fallback;
};
const OUT = join(HERE, flag("--out", "out"));
const MODEL = flag("--model", "meta/llama-3.1-8b-instruct");
const DRY = process.argv.includes("--dry");
const TRIALS = Number(flag("--trials", 1));

function apiKey() {
  if (process.env.NVIDIA_API_KEY) return process.env.NVIDIA_API_KEY;
  try {
    return /NVIDIA_API_KEY=(.+)/.exec(readFileSync(join(ROOT, "docs", ".env.local"), "utf8"))?.[1]?.trim();
  } catch {
    return null;
  }
}

const key = apiKey();

/** One model call. Returns null on any upstream failure, which the loop treats as "no help". */
async function askModel(prompt) {
  const res = await fetch("https://integrate.api.nvidia.com/v1/chat/completions", {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
    body: JSON.stringify({
      model: MODEL,
      messages: [{ role: "user", content: prompt }],
      temperature: 0.1,
      max_tokens: 2048,
      stream: false,
    }),
  }).catch(() => null);
  if (!res?.ok) {
    console.warn(`  ! repair call failed: ${res ? res.status : "network"}`);
    return null;
  }
  const json = await res.json().catch(() => null);
  return json?.choices?.[0]?.message?.content ?? null;
}

/* --------------------------------------------------------------------- main */

const els = (ast) => {
  const out = [];
  const walk = (list) => {
    for (const el of list ?? []) {
      out.push(el);
      walk(el.children);
    }
  };
  walk(ast.tree);
  return out;
};

const files = readdirSync(OUT).filter((f) => f.endsWith(".guml"));
if (files.length === 0) {
  console.error(`no generations in ${OUT}`);
  process.exit(1);
}

console.log(`${files.length} documents from ${OUT}${DRY ? " (free layers only)" : ` · repair model ${MODEL}`}\n`);

const rows = [];
for (const file of files) {
  const app = APPS.find((a) => file === `${a.id}.guml` || file.startsWith(`${a.id}-r`));
  if (!app) continue;

  const raw = readFileSync(join(OUT, file), "utf8");
  process.stdout.write(`${app.id.padEnd(11)} `);
  // `null` rather than a no-op function: the pipeline counts a round whenever it *calls*,
  // and a dry run must not report rounds it never made.
  const result = await repair(raw, {
    askModel: DRY || !key ? null : askModel,
    trials: TRIALS,
  });

  const at = (layer) => result.trace.find((t) => t.layer === layer)?.errors ?? null;
  const sanitizeNotes = result.trace.find((t) => t.layer === "sanitize") ?? {};

  writeFileSync(join(OUT, `${app.id}.repaired.guml`), result.text);
  writeFileSync(
    join(OUT, `${app.id}.trace.json`),
    JSON.stringify({ app: app.id, model: MODEL, trace: result.trace }, null, 2),
  );

  rows.push({
    app: app.id,
    raw: at("raw"),
    sanitized: at("sanitize"),
    formatted: at("format"),
    fixed: at("fix"),
    afterModel: at("model"),
    // A repair that increases the error count is discarded, so the reported figure has to
    // say so — otherwise the table shows a number that was never adopted.
    modelAccepted: result.trace.find((t) => t.layer === "model")?.accepted ?? null,
    final: result.errors.length,
    modelRounds: result.modelRounds,
    droppedLines: sanitizeNotes.droppedLines ?? 0,
    freeFixed: [
      ...fixedBetween(result.trace, "raw", "sanitize"),
      ...fixedBetween(result.trace, "sanitize", "format"),
      ...fixedBetween(result.trace, "format", "fix"),
    ],
    survived: [...new Set(result.errors.map((d) => d.id))].sort(),
    attempts: result.attempts,
  });
  console.log(`${at("raw")} → ${result.errors.length} errors`);
}

/* ------------------------------------------------------------------- report */

console.log("\n| app | raw | after free layers | after model | model rounds | survived |");
console.log("|---|---|---|---|---|---|");
for (const r of rows) {
  console.log(
    `| ${r.app} | ${r.raw} | ${r.fixed} | ${
      r.attempts.length === 0
        ? "—"
        : `${r.attempts.filter((a) => a.accepted).length}/${r.attempts.length}`
    } | ${r.final} | ${r.survived.join(", ") || "—"} |`,
  );
}

const freed = rows.filter((r) => r.raw > 0 && r.fixed === 0);
const needed = rows.filter((r) => r.fixed > 0);
const solved = needed.filter((r) => r.final === 0);
const madeWorse = rows.filter((r) => r.modelAccepted === false);

console.log("");
console.log(`${rows.filter((r) => r.raw === 0).length}/${rows.length} compiled with no repair at all`);
console.log(`${freed.length} fixed by the free layers alone — no model call`);
console.log(
  `${solved.length}/${needed.length} of the rest fixed by a repair round (${TRIALS} trial(s) each)`,
);
const discarded = rows.reduce(
  (n, r) => n + r.attempts.filter((a) => !a.accepted && a.errors !== null).length,
  0,
);
if (discarded > 0) {
  console.log(`${discarded} repair attempt(s) did not improve on the free layers and were discarded`);
}

const stillBroken = rows.filter((r) => r.final > 0);
if (stillBroken.length > 0) {
  console.log("\nsurvived every layer:");
  for (const r of stillBroken) console.log(`  ${r.app.padEnd(11)} ${r.survived.join(", ")}`);
}

const droppedTotal = rows.reduce((n, r) => n + r.droppedLines, 0);
if (droppedTotal > 0) {
  console.log(`\n${droppedTotal} trailing line(s) of commentary dropped by the sanitiser`);
}

writeFileSync(join(OUT, "repair-report.json"), JSON.stringify({ model: MODEL, dry: DRY, rows }, null, 2));
