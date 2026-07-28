#!/usr/bin/env node
/**
 * Generate the applications in `apps.mjs` and check what came back.
 *
 * Three questions, kept separate because they fail for different reasons:
 *
 *  1. **Parses** — is it GUML at all? (`guml check`)
 *  2. **Validates** — does it mean anything? (the static validator, `--strict`)
 *  3. **Is the app** — does it contain what was asked for? (`must` predicates over the AST)
 *
 * A generation can parse and still be useless: a todo list with no delete action is valid
 * GUML and the wrong program. Only the third question catches that, which is why it exists
 * separately from "0 errors".
 *
 *   node run.mjs                     # generate, then score
 *   node run.mjs --model X           # a specific NIM
 *   node run.mjs --apps todo,bmi     # a subset
 *   node run.mjs --score-only        # re-score what is already in out/
 *   node run.mjs --repeat 3          # variance across identical prompts
 *   node run.mjs --out out-70b       # keep runs of different models apart
 */
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { systemPrompt } from "../phase0/lib/prompt.mjs";
import { APPS } from "./apps.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
/**
 * Output directory. Overridable because two runs of different models write the same
 * filenames — the first 70B run silently overwrote the 8B generations mid-scoring, which is
 * the kind of thing that quietly turns a comparison into nonsense.
 */
const OUT = join(HERE, process.argv.includes("--out") ? process.argv[process.argv.indexOf("--out") + 1] : "out");

const flag = (name, fallback) => {
  const i = process.argv.indexOf(name);
  return i > -1 ? process.argv[i + 1] : fallback;
};
const MODEL = flag("--model", process.env.NVIDIA_MODEL || "meta/llama-3.1-8b-instruct");
const REPEAT = Number(flag("--repeat", 1));
const ONLY = flag("--apps", null)?.split(",");
const SCORE_ONLY = process.argv.includes("--score-only");
const apps = ONLY ? APPS.filter((a) => ONLY.includes(a.id)) : APPS;

mkdirSync(OUT, { recursive: true });

/* ------------------------------------------------------------------ generate */

/** The key is read from the docs env file: one place, already gitignored. */
function apiKey() {
  if (process.env.NVIDIA_API_KEY) return process.env.NVIDIA_API_KEY;
  const envPath = join(ROOT, "docs", ".env.local");
  if (!existsSync(envPath)) return null;
  return /NVIDIA_API_KEY=(.+)/.exec(readFileSync(envPath, "utf8"))?.[1]?.trim() || null;
}

function extract(text) {
  const fenced = /```(?:guml)?\s*\n([\s\S]*?)(?:```|$)/.exec(text);
  return (fenced ? fenced[1] : text).trim();
}

async function generate(app, key, system, repeat) {
  const t0 = Date.now();
  const res = await fetch("https://integrate.api.nvidia.com/v1/chat/completions", {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
    body: JSON.stringify({
      model: MODEL,
      messages: [
        { role: "system", content: system },
        { role: "user", content: app.prompt },
      ],
      temperature: 0.2,
      max_tokens: 2048,
      stream: false,
    }),
  });

  const json = await res.json().catch(() => null);
  if (!res.ok) {
    return { error: `${res.status} ${json?.detail ?? json?.error?.message ?? ""}`.trim() };
  }
  const text = json?.choices?.[0]?.message?.content ?? "";
  return {
    source: extract(text),
    latencyMs: Date.now() - t0,
    usage: json?.usage ?? null,
    repeat,
  };
}

/* --------------------------------------------------------------------- score */

function guml(args, allowFailure = false) {
  try {
    return execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", ...args], {
      cwd: ROOT,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (e) {
    if (allowFailure && e.stdout) return e.stdout.toString();
    throw e;
  }
}

function score(app, file) {
  const diagnostics = JSON.parse(guml(["check", file, "--format", "json"], true).trim() || "[]");
  const errors = diagnostics.filter((d) => d.severity === "error");
  const warnings = diagnostics.filter((d) => d.severity === "warning");

  let ast = null;
  try {
    ast = JSON.parse(guml(["ast", file], true));
  } catch {
    /* unparseable enough that the AST dump failed */
  }

  const checks = app.must.map(([label, predicate]) => {
    if (!ast) return { label, ok: false, note: "no AST" };
    try {
      return { label, ok: Boolean(predicate(ast)) };
    } catch (e) {
      return { label, ok: false, note: e.message };
    }
  });

  return {
    parses: errors.length === 0,
    errors: errors.map((d) => `${d.id} line ${d.span.line}: ${d.message}`),
    warnings: warnings.map((d) => d.id),
    checks,
    met: checks.filter((c) => c.ok).length,
    total: checks.length,
  };
}

/* ---------------------------------------------------------------------- main */

if (!SCORE_ONLY) {
  const key = apiKey();
  if (!key) {
    console.error("No API key. Set NVIDIA_API_KEY or fill docs/.env.local.");
    process.exit(1);
  }

  // The shipping prompt, not a special one for the test — otherwise the test measures
  // something the product does not do.
  const system = systemPrompt({ examples: 3 });
  console.log(`model: ${MODEL}\nsystem prompt: ${system.length} chars\n`);

  for (const app of apps) {
    for (let r = 1; r <= REPEAT; r++) {
      const stem = REPEAT > 1 ? `${app.id}-r${r}` : app.id;
      const file = join(OUT, `${stem}.guml`);
      process.stdout.write(`${stem.padEnd(14)} `);
      const result = await generate(app, key, system, r);
      if (result.error) {
        console.log(`✗ ${result.error}`);
        writeFileSync(join(OUT, `${stem}.error.txt`), result.error);
        continue;
      }
      writeFileSync(file, `${result.source}\n`);
      writeFileSync(
        join(OUT, `${stem}.meta.json`),
        JSON.stringify({ model: MODEL, ...result, source: undefined }, null, 2),
      );
      console.log(`${result.latencyMs}ms, ${result.source.split("\n").length} lines`);
    }
  }
  console.log("");
}

const files = readdirSync(OUT)
  .filter((f) => f.endsWith(".guml"))
  .filter((f) => apps.some((a) => f === `${a.id}.guml` || f.startsWith(`${a.id}-r`)));

if (files.length === 0) {
  console.error("nothing to score in bench/gen/out");
  process.exit(1);
}

const rows = [];
for (const f of files) {
  const app = apps.find((a) => f === `${a.id}.guml` || f.startsWith(`${a.id}-r`));
  const result = score(app, join(OUT, f));
  rows.push({ file: f, app: app.id, ...result });
}

console.log("| app | parses | requirements | errors |");
console.log("|---|---|---|---|");
for (const r of rows) {
  console.log(
    `| ${r.file.replace(/\.guml$/, "")} | ${r.parses ? "yes" : "NO"} | ${r.met}/${r.total} | ${
      r.errors.length ? r.errors[0] : "—"
    } |`,
  );
}

const failures = rows.filter((r) => !r.parses || r.met < r.total);
if (failures.length > 0) {
  console.log("\nwhat is missing:");
  for (const r of failures) {
    console.log(`\n  ${r.file}`);
    for (const e of r.errors) console.log(`    error  ${e}`);
    for (const c of r.checks.filter((c) => !c.ok)) {
      console.log(`    unmet  ${c.label}${c.note ? ` (${c.note})` : ""}`);
    }
  }
}

const parsed = rows.filter((r) => r.parses).length;
const met = rows.reduce((n, r) => n + r.met, 0);
const total = rows.reduce((n, r) => n + r.total, 0);
console.log(
  `\n${parsed}/${rows.length} parse · ${met}/${total} requirements met · model ${MODEL}`,
);
writeFileSync(join(OUT, "report.json"), JSON.stringify({ model: MODEL, rows }, null, 2));
