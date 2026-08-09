/**
 * The two numbers this project is entitled to quote, measured so they cannot go stale.
 *
 * # Why the old measurement had a soft underbelly
 *
 * The headline figure — 178 tokens of GUML against 1,441 of React — compares a `.guml` fixture with a
 * `.react.tsx` fixture **written by the same person**, who has a thesis. A referee attacks that
 * immediately and is right to: the React could be verbose, the GUML could be tuned, and nothing in the
 * repository distinguishes a real compression ratio from an authoring style.
 *
 * The fix needs no second author. **Compile the GUML and measure what comes out.** The emitted React
 * is not written by anyone — it is what the compiler produces, so `tokens(source) / tokens(emitted)`
 * is a property of the language and the compiler rather than of two people's habits.
 *
 * **It is not automatically the smaller number, and pretending otherwise would be the same mistake
 * again.** On `b.guml` it is 16.0× against the emitted output and 8.8× against the hand-written
 * baseline: the compiler emits a retry helper, a cache layer and explanatory comments that a person
 * would not write, so the denominator is larger. The self-consistent figure is the *defensible* one,
 * not the conservative one, and the two are printed side by side so the gap is visible rather than
 * chosen.
 *
 * Both are reported. The hand-paired one is not deleted — it answers a different question ("versus
 * what a person would write") and is worth having as long as it is labelled as what it is.
 *
 * # The second number, which is the better argument
 *
 * How many errors in real model output are fixed **with no model call**. Nobody else can quote that,
 * because it requires a compiler that names its failures. `bench/gen` has six real generations; this
 * counts what the mechanical layers do to them.
 *
 * # Units
 *
 * `guml tokens` is a ~3.6 chars/token estimate, not a tokenizer, and every figure here says so. See
 * `.claude/skills/guml-measure`. A tokenizer count belongs in a paper; a ratio that cannot drift
 * belongs in CI.
 */

import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import process from "node:process";

/** The project's own estimate, so this agrees with `guml tokens` rather than inventing a second one. */
const estTokens = (text) => Math.round(text.length / 3.6);

/**
 * Run the CLI and return stdout **even when it exits non-zero**.
 *
 * `guml check` exits 1 on a document with errors — correct for a shell, and it makes `execFileSync`
 * throw. Here a non-zero exit is the normal case: this whole script is about counting errors, so the
 * runs that fail are the ones with something to count. The output is on the exception either way.
 */
const cli = (...args) => {
  try {
    return execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", ...args], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
      maxBuffer: 32 * 1024 * 1024,
    });
  } catch (e) {
    if (typeof e.stdout === "string") return e.stdout;
    throw e;
  }
};

// ---------------------------------------------------------------- compression, self-consistently

const fixtures = readdirSync("fixtures")
  .filter((f) => f.endsWith(".guml"))
  .sort();

console.log("Compression — GUML against its own emitted React");
console.log("(no second author: the right-hand side is what the compiler produced)\n");
console.log("  fixture        guml    emitted   ratio   vs hand-written");
console.log("  " + "-".repeat(58));

let sourceTotal = 0;
let emittedTotal = 0;

for (const file of fixtures) {
  const source = readFileSync(join("fixtures", file), "utf8");
  const emitted = cli("build", `fixtures/${file}`);

  const src = estTokens(source);
  const out = estTokens(emitted);
  sourceTotal += src;
  emittedTotal += out;

  // The hand-written baseline, where one exists, purely for comparison against the honest number.
  const paired = file.replace(/\.guml$/, ".react.tsx");
  let handwritten = "—";
  try {
    handwritten = `${(estTokens(readFileSync(join("fixtures", paired), "utf8")) / src).toFixed(1)}×`;
  } catch {
    // Most fixtures have no hand-written pair, which is itself the point: the ratio above needs none.
  }

  console.log(
    `  ${file.padEnd(14)} ${String(src).padStart(5)} ${String(out).padStart(9)}` +
      `   ${(out / src).toFixed(1).padStart(4)}×   ${handwritten.padStart(6)}`,
  );
}

const ratio = (emittedTotal / sourceTotal).toFixed(1);
console.log("  " + "-".repeat(58));
console.log(`  ${"total".padEnd(14)} ${String(sourceTotal).padStart(5)} ${String(emittedTotal).padStart(9)}   ${ratio.padStart(4)}×`);
console.log(
  `\n  ${ratio}× fewer estimated tokens to write than the React it compiles to,` +
    `\n  across ${fixtures.length} fixtures. ~3.6 chars/token estimate, not a tokenizer count.`,
);

// ---------------------------------------------------------------- mechanical repair rate

const GEN = "bench/gen/out";
let apps = [];
try {
  apps = readdirSync(GEN).filter((f) => f.endsWith(".guml") && !f.includes("repaired"));
} catch {
  console.log("\n(no generation output in bench/gen/out — skipping the repair rate)");
  process.exit(0);
}

console.log("\n\nMechanical repair — errors fixed with no model call");
console.log("(real output from `bench/gen`, six applications through a hosted model)\n");
console.log("  app          raw   after repair   fixed");
console.log("  " + "-".repeat(44));

const errorsIn = (path) => {
  const raw = cli("check", path, "--format", "json").trim();
  return JSON.parse(raw).filter((d) => d.severity === "error").length;
};

let rawTotal = 0;
let leftTotal = 0;
let compiledRaw = 0;
let compiledAfter = 0;

for (const file of apps.sort()) {
  const path = join(GEN, file);
  const raw = errorsIn(path);

  // `guml repair` is the whole mechanical stack: sanitise a fenced response, format, then apply every
  // unambiguous suggestion. No model involved at any point.
  const repaired = cli("repair", path);
  const tmp = join("target", `repair-${file}`);
  writeFileSync(tmp, repaired, "utf8");
  const left = errorsIn(tmp);

  rawTotal += raw;
  leftTotal += left;
  if (raw === 0) compiledRaw++;
  if (left === 0) compiledAfter++;

  const fixed = raw === 0 ? "—" : `${raw - left}/${raw}`;
  console.log(
    `  ${file.replace(".guml", "").padEnd(12)} ${String(raw).padStart(3)}   ${String(left).padStart(12)}   ${fixed.padStart(5)}`,
  );
}

const pct = rawTotal ? Math.round(((rawTotal - leftTotal) / rawTotal) * 100) : 0;
console.log("  " + "-".repeat(44));
console.log(`  ${"total".padEnd(12)} ${String(rawTotal).padStart(3)}   ${String(leftTotal).padStart(12)}   ${pct}%`);
console.log(
  `\n  ${compiledRaw} of ${apps.length} compiled as generated; ${compiledAfter} of ${apps.length} after mechanical repair.` +
    `\n  ${pct}% of errors fixed with no model call — n=${apps.length}, so this points at a` +
    `\n  behaviour rather than establishing a rate.`,
);
