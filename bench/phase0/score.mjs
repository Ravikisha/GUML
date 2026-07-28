#!/usr/bin/env node
/**
 * Phase 0 scoring.
 *
 * Two halves, deliberately separated:
 *
 *  - Mechanical, done here: parse validity, diagnostic codes, output tokens, prompt
 *    tax, cache reads, latency, escape-hatch rate.
 *  - Human, not done here: semantic correctness against each task's checklist. This
 *    script emits a *blind* scoresheet (arm and model stripped, rows shuffled) and
 *    reads it back once filled. Scoring while knowing which arm you are looking at
 *    is the single easiest way to produce the result you were hoping for.
 *
 *   node score.mjs                # mechanical report + blind scoresheet
 *   node score.mjs --seed 7       # different shuffle
 *   node score.mjs --results DIR  # score an alternative results directory
 *   node score.mjs --refresh-scoresheet   # rebuild the blind sheet, DISCARDING scores
 */
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BENCH, ROOT } from "./lib/prompt.mjs";
import { CATEGORIES, taskById } from "./tasks/index.mjs";

const flag = (name, fallback) => {
  const i = process.argv.indexOf(name);
  return i > -1 ? process.argv[i + 1] : fallback;
};

// `--results` exists so selftest.mjs can score a synthetic directory without
// touching real generations.
const RESULTS = flag("--results", join(BENCH, "results"));
const RAW = join(RESULTS, "raw");
const SEED = Number(flag("--seed", 1));

if (!existsSync(RAW) || readdirSync(RAW).filter((f) => f.endsWith(".json")).length === 0) {
  console.error(`no runs in ${RAW}. Run: node run.mjs`);
  process.exit(1);
}

const runs = readdirSync(RAW)
  .filter((f) => f.endsWith(".json"))
  .map((f) => JSON.parse(readFileSync(join(RAW, f), "utf8")))
  .filter((r) => !r.error);

/* ---------------------------------------------------------------- extraction */

/**
 * A fence is a rule violation, not a formatting detail: the whole point is that the
 * output goes straight into a compiler. It is stripped so parse validity measures
 * the language rather than the wrapper, and counted separately.
 */
function extract(output) {
  const fenced = /^\s*```[a-zA-Z]*\n([\s\S]*?)\n?```\s*$/.exec(output);
  return { code: (fenced ? fenced[1] : output).trim(), fenced: Boolean(fenced) };
}

const ESCAPE_MARKER = /^\/\/\s*UNSUPPORTED:\s*(.+)$/gm;

/**
 * The marker is a GUML comment, so it compiles away and needs no stripping.
 *
 * It was originally `# UNSUPPORTED:`, written on the belief that GUML had no comment
 * syntax. It does — `//`, dropped by the lexer (`crates/guml-syntax`) — and the `#` form
 * parsed as an unknown tag, so every honest "I could not express this" was *also* counted
 * as a parse failure. Two measurements that must stay apart were being conflated by the
 * harness itself.
 */
function stripMarkers(code) {
  return code.trim();
}

function escapeHatches(code) {
  const found = [];
  for (const m of code.matchAll(ESCAPE_MARKER)) found.push(m[1].trim());
  // `js` / `raw` blocks are PLANNED in the spec; a model reaching for them is the
  // same signal as an UNSUPPORTED marker, so it counts the same.
  if (/^\s*(js|raw)\s*$/m.test(code)) found.push("used a js/raw escape block");
  return found;
}

const CHECK_DIR = join(tmpdir(), "guml-phase0-check");
mkdirSync(CHECK_DIR, { recursive: true });

function check(code, id) {
  const file = join(CHECK_DIR, `${id}.guml`);
  writeFileSync(file, code.endsWith("\n") ? code : `${code}\n`);
  try {
    const out = execFileSync(
      "cargo",
      ["run", "-q", "-p", "guml-cli", "--", "check", "--format", "json", file],
      { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    );
    return JSON.parse(out.trim() || "[]");
  } catch (e) {
    // A non-zero exit means errors were found; the diagnostics are still on stdout.
    const stdout = e.stdout?.toString().trim();
    if (stdout) {
      try {
        return JSON.parse(stdout);
      } catch {
        /* fall through */
      }
    }
    return [{ id: "HARNESS", severity: "error", message: `check failed: ${e.message}` }];
  }
}

/* ------------------------------------------------------------------- scoring */

const scored = runs.map((r) => {
  const base = {
    id: r.id,
    task: r.task,
    category: r.category,
    arm: r.arm,
    model: r.modelAlias,
    examples: r.examples,
    repeat: r.repeat ?? 1,
    outputTokens: r.usage?.output_tokens ?? null,
    inputTokens: r.usage?.input_tokens ?? null,
    cacheRead: r.usage?.cache_read_input_tokens ?? 0,
    cacheWrite: r.usage?.cache_creation_input_tokens ?? 0,
    latencyMs: r.latencyMs ?? null,
    truncated: r.stopReason === "max_tokens",
  };
  if (r.arm !== "guml") return { ...base, parseable: null, fenced: null, hatches: [], diagnostics: [] };

  const { code, fenced } = extract(r.output ?? "");
  const hatches = escapeHatches(code);
  const diagnostics = check(stripMarkers(code), r.id);
  const errors = diagnostics.filter((d) => d.severity === "error");
  return {
    ...base,
    parseable: errors.length === 0,
    fenced,
    hatches,
    diagnostics: diagnostics.map((d) => d.id),
    errorCodes: errors.map((d) => d.id),
  };
});

/* ---------------------------------------------------------------- statistics */

const median = (xs) => {
  const s = xs.filter((x) => typeof x === "number").sort((a, b) => a - b);
  if (s.length === 0) return null;
  const m = Math.floor(s.length / 2);
  return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
};
const pct = (n, d) => (d === 0 ? "—" : `${Math.round((n / d) * 100)}%`);
const where = (f) => scored.filter(f);

function tokenTable() {
  const rows = [];
  for (const category of CATEGORIES) {
    const react = median(where((s) => s.arm === "react" && s.category === category).map((s) => s.outputTokens));
    for (const examples of [0, 3]) {
      const guml = median(
        where((s) => s.arm === "guml" && s.category === category && s.examples === examples).map(
          (s) => s.outputTokens,
        ),
      );
      if (guml === null && react === null) continue;
      rows.push({
        category,
        examples,
        guml,
        react,
        ratio: guml && react ? (react / guml).toFixed(1) : "—",
      });
    }
  }
  return rows;
}

function parseTable() {
  const rows = [];
  for (const model of ["haiku", "sonnet", "opus"]) {
    for (const examples of [0, 3]) {
      const set = where((s) => s.arm === "guml" && s.model === model && s.examples === examples);
      if (set.length === 0) continue;
      rows.push({
        model,
        examples,
        n: set.length,
        parseable: pct(set.filter((s) => s.parseable).length, set.length),
        fenced: pct(set.filter((s) => s.fenced).length, set.length),
        hatched: pct(set.filter((s) => s.hatches.length > 0).length, set.length),
        truncated: set.filter((s) => s.truncated).length,
      });
    }
  }
  return rows;
}

function errorFrequency() {
  const counts = new Map();
  for (const s of where((x) => x.arm === "guml")) {
    for (const code of s.errorCodes ?? []) counts.set(code, (counts.get(code) ?? 0) + 1);
  }
  return [...counts.entries()].sort((a, b) => b[1] - a[1]);
}

/* ----------------------------------------------------------- blind scoresheet */

/** Deterministic shuffle: a seed means a re-run produces the same sheet. */
function shuffled(items, seed) {
  let state = seed * 2654435761 % 2147483647;
  const next = () => (state = (state * 48271) % 2147483647) / 2147483647;
  const out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(next() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

function writeScoresheet() {
  const sheet = join(RESULTS, "scoresheet.csv");
  const gradable = shuffled(
    // An unparseable generation has nothing to score, and putting it in front of a
    // human as a zero would double-count a failure already in the parse column.
    scored.filter((s) => s.arm === "react" || s.parseable),
    SEED,
  );
  const keymap = {};
  const lines = ["blindId,task,checklistItem,score"];
  gradable.forEach((s, i) => {
    const blindId = `S${String(i + 1).padStart(3, "0")}`;
    keymap[blindId] = { id: s.id, arm: s.arm, model: s.model, examples: s.examples };
    for (const item of taskById(s.task).checklist) {
      lines.push(`${blindId},${s.task},"${item.replace(/"/g, '""')}",`);
    }
  });

  // Never clobber human work: this script is re-run to refresh the report, and
  // overwriting the sheet would silently discard hours of blind scoring.
  if (existsSync(sheet) && !process.argv.includes("--refresh-scoresheet")) {
    return { n: gradable.length, items: lines.length - 1, kept: true };
  }
  writeFileSync(sheet, `${lines.join("\n")}\n`);
  writeFileSync(join(RESULTS, "keymap.json"), `${JSON.stringify(keymap, null, 2)}\n`);
  return { n: gradable.length, items: lines.length - 1, kept: false };
}

/** Reads scoresheet.csv back once a human has filled the score column. */
function correctness() {
  const file = join(RESULTS, "scoresheet.csv");
  const keymapFile = join(RESULTS, "keymap.json");
  if (!existsSync(file) || !existsSync(keymapFile)) return null;
  const rows = readFileSync(file, "utf8").trim().split("\n").slice(1);
  const keymap = JSON.parse(readFileSync(keymapFile, "utf8"));
  const totals = new Map();
  let filled = 0;
  for (const line of rows) {
    const m = /^([^,]+),([^,]+),(?:"(?:[^"]|"")*"|[^,]*),([^,]*)$/.exec(line);
    if (!m) continue;
    const [, blindId, , raw] = m;
    if (raw.trim() === "") continue;
    filled++;
    const score = Number(raw);
    const entry = totals.get(blindId) ?? { sum: 0, n: 0 };
    entry.sum += Number.isFinite(score) ? score : 0;
    entry.n += 1;
    totals.set(blindId, entry);
  }
  if (filled === 0) return { filled: 0 };
  const byArm = { guml: [], react: [] };
  for (const [blindId, t] of totals) {
    const key = keymap[blindId];
    if (!key) continue;
    byArm[key.arm]?.push(t.sum / t.n);
  }
  return {
    filled,
    guml: median(byArm.guml),
    react: median(byArm.react),
    nGuml: byArm.guml.length,
    nReact: byArm.react.length,
  };
}

/* -------------------------------------------------------------------- report */

const tokens = tokenTable();
const parse = parseTable();
const errors = errorFrequency();
const sheet = writeScoresheet();
const human = correctness();

const structure = tokens.find((r) => r.category === "structure" && r.examples === 3);
const sonnet3 = parse.find((r) => r.model === "sonnet" && r.examples === 3);

const gates = [
  {
    text: "≥80% of Sonnet 5 generations at 3 examples are parseable GUML",
    value: sonnet3 ? sonnet3.parseable : "not run",
    pass: sonnet3 ? Number.parseInt(sonnet3.parseable, 10) >= 80 : null,
  },
  {
    text: "Median output-token reduction ≥3× on structure-heavy tasks",
    value: structure ? `${structure.ratio}×` : "not run",
    pass: structure && structure.ratio !== "—" ? Number(structure.ratio) >= 3 : null,
  },
  {
    text: "Semantic correctness not worse than the React baseline",
    value:
      human && human.filled > 0
        ? `GUML ${human.guml?.toFixed(2)} vs React ${human.react?.toFixed(2)}`
        : "scoresheet not filled in",
    pass: human && human.filled > 0 && human.guml !== null && human.react !== null ? human.guml >= human.react : null,
  },
];

const md = [];
md.push("# Phase 0 — mechanical results", "");
md.push(`Generated from ${runs.length} runs in \`results/raw\`. Shuffle seed ${SEED}.`, "");

if (scored.some((s) => s.truncated)) {
  md.push(
    "> **Truncated generations present.** Runs that hit `max_tokens` have a censored",
    "> output-token count and must not be pooled into a median. They are listed at the end.",
    "",
  );
}

md.push("## Output tokens (median)", "");
md.push("| category | examples | GUML | React | ratio |", "|---|---|---|---|---|");
for (const r of tokens) {
  md.push(`| ${r.category} | ${r.examples} | ${r.guml ?? "—"} | ${r.react ?? "—"} | ${r.ratio} |`);
}
md.push("", "Per category, never pooled: the content floor makes a single average meaningless.", "");

md.push("## Parse validity", "");
md.push("| model | examples | n | parseable | fenced | escape hatch | truncated |", "|---|---|---|---|---|---|---|");
for (const r of parse) {
  md.push(`| ${r.model} | ${r.examples} | ${r.n} | ${r.parseable} | ${r.fenced} | ${r.hatched} | ${r.truncated} |`);
}
md.push("");

md.push("## Prompt tax", "");
const gumlRuns = where((s) => s.arm === "guml");
md.push(
  `| metric | value |`,
  `|---|---|`,
  `| median input tokens, GUML arm | ${median(gumlRuns.map((s) => s.inputTokens)) ?? "—"} |`,
  `| median cache read, GUML arm | ${median(gumlRuns.map((s) => s.cacheRead)) ?? "—"} |`,
  `| median input tokens, React arm | ${median(where((s) => s.arm === "react").map((s) => s.inputTokens)) ?? "—"} |`,
  `| median latency, GUML arm | ${median(gumlRuns.map((s) => s.latencyMs)) ?? "—"} ms |`,
  `| median latency, React arm | ${median(where((s) => s.arm === "react").map((s) => s.latencyMs)) ?? "—"} ms |`,
  "",
  "Input is reported separately from output and is never netted off against it.",
  "",
);

md.push("## What was invalid", "");
if (errors.length === 0) {
  md.push("No error diagnostics across the GUML arm.", "");
} else {
  md.push("| diagnostic | occurrences |", "|---|---|");
  for (const [code, n] of errors) md.push(`| ${code} | ${n} |`);
  md.push("", "`GUML0030` is the one that matters: an unknown tag means the vocabulary was", "guessed at, which is the low-resource-DSL failure mode.", "");
}

md.push("## Escape hatches", "");
const hatched = where((s) => s.hatches.length > 0);
md.push(`${hatched.length} of ${gumlRuns.length} GUML runs reached for something the language cannot express.`, "");
if (hatched.length > 0) {
  for (const s of hatched) md.push(`- \`${s.id}\` — ${s.hatches.join("; ")}`);
  md.push("");
}

md.push("## Semantic correctness", "");
if (!human || human.filled === 0) {
  md.push(
    `Not scored yet. \`results/scoresheet.csv\` has ${sheet.items} checklist rows across`,
    `${sheet.n} generations, blinded and shuffled. Fill the \`score\` column with 1, 0.5 or 0`,
    "per row (see `rubric.md`), then re-run this script. `keymap.json` de-blinds it — do not",
    "open it while scoring.",
    "",
  );
} else {
  md.push(
    `| arm | n | mean checklist score |`,
    `|---|---|---|`,
    `| GUML | ${human.nGuml} | ${human.guml?.toFixed(2)} |`,
    `| React | ${human.nReact} | ${human.react?.toFixed(2)} |`,
    "",
    `${human.filled} checklist rows scored.`,
    "",
  );
}

md.push("## The gate", "");
for (const g of gates) {
  const mark = g.pass === null ? "?" : g.pass ? "x" : " ";
  md.push(`- [${mark}] ${g.text} — **${g.value}**`);
}
md.push("");
const decided = gates.every((g) => g.pass !== null);
md.push(
  decided
    ? gates.every((g) => g.pass)
      ? "**All three gates pass.** Proceed to Phase 1."
      : "**A gate failed.** See `spec/PHASE0.md` for what each failure mode implies — several are still publishable."
    : "**Undecided.** At least one gate has no data yet.",
  "",
);

writeFileSync(join(RESULTS, "summary.md"), `${md.join("\n")}\n`);

const csv = [
  "id,task,category,arm,model,examples,repeat,outputTokens,inputTokens,cacheRead,latencyMs,parseable,fenced,hatches,truncated",
  ...scored.map((s) =>
    [
      s.id,
      s.task,
      s.category,
      s.arm,
      s.model,
      s.examples,
      s.repeat,
      s.outputTokens ?? "",
      s.inputTokens ?? "",
      s.cacheRead,
      s.latencyMs ?? "",
      s.parseable ?? "",
      s.fenced ?? "",
      s.hatches.length,
      s.truncated,
    ].join(","),
  ),
];
writeFileSync(join(RESULTS, "runs.csv"), `${csv.join("\n")}\n`);

console.log(`scored ${scored.length} runs`);
console.log(
  sheet.kept
    ? `wrote summary.md, runs.csv; kept the existing scoresheet (${sheet.items} rows expected)`
    : `wrote summary.md, runs.csv, scoresheet.csv (${sheet.items} rows)`,
);
for (const g of gates) console.log(`  [${g.pass === null ? "?" : g.pass ? "x" : " "}] ${g.text} — ${g.value}`);
