/**
 * The counts the README states must be the counts that exist.
 *
 * # Why a script rather than another careful edit
 *
 * These go stale constantly, and every instance was found by accident. In one session: the wasm size
 * claimed as 298 KB when it was 787 (three places), 29 tags when there were 49, 367 tests when there
 * were 514, 507 when there were 530, 49 diagnostic codes when there were 50.
 *
 * None of them was a *lie* — each was true when written. That is exactly why a person re-reading the
 * file does not catch them: the sentence still reads correctly. Only a comparison against the thing
 * being counted catches it, and a comparison is a script.
 *
 * This matters more here than in most projects. The README is the front page of a project whose claim
 * discipline is one of its arguments, and "507 Rust tests" being wrong is small but it is wrong in the
 * one dimension the project asks to be trusted on.
 *
 * # What it does not check
 *
 * Token counts (`178 tokens`, `1,441 tokens`) come from a measured run with a named tokenizer and are
 * properties of fixtures that do not change; re-deriving them would mean re-running the measurement,
 * which is `guml tokens` and a deliberate act. They are labelled as measured, with the tokenizer
 * named, which is the requirement — see `.claude/skills/guml-measure`.
 */

import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";
import process from "node:process";

const readme = readFileSync("README.md", "utf8");

/** Count unique diagnostic codes the compiler defines. */
function diagnosticCodes() {
  const src = readFileSync("crates/guml-diagnostics/src/lib.rs", "utf8");
  return new Set(src.match(/GUML0\d{3}/g) ?? []).size;
}

/** Total passing Rust tests across the workspace. */
function rustTests() {
  const out = execFileSync("cargo", ["test", "--workspace"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    maxBuffer: 64 * 1024 * 1024,
  });
  let total = 0;
  for (const m of out.matchAll(/^test result: ok\. (\d+) passed/gm)) total += Number(m[1]);
  return total;
}

function tags() {
  const out = execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", "registry"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  return out
    .split(/\r?\n/)
    .filter((l) => l.trim() && !/^[a-z-]+:/.test(l))
    .length;
}

const fixtures = readdirSync("fixtures").filter((f) => f.endsWith(".guml")).length;

/** GUML source against its own emitted React — the figure with no second author. */
function compressionRatio() {
  const est = (t) => Math.round(t.length / 3.6);
  let src = 0;
  let out = 0;
  for (const f of readdirSync("fixtures").filter((x) => x.endsWith(".guml"))) {
    src += est(readFileSync(`fixtures/${f}`, "utf8"));
    out += est(
      execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", "build", `fixtures/${f}`], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
        maxBuffer: 32 * 1024 * 1024,
      }),
    );
  }
  return Number((out / src).toFixed(1));
}

/** How many of the six real generations in `bench/gen/out` compile as written. */
function compiledCount() {
  const apps = readdirSync("bench/gen/out").filter(
    (f) => f.endsWith(".guml") && !f.includes("repaired"),
  );
  let clean = 0;
  for (const f of apps) {
    let raw;
    try {
      raw = execFileSync(
        "cargo",
        ["run", "-q", "-p", "guml-cli", "--", "check", `bench/gen/out/${f}`, "--format", "json"],
        { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
      );
    } catch (e) {
      // `check` exits non-zero on errors, which is the normal case here.
      raw = typeof e.stdout === "string" ? e.stdout : "[]";
    }
    if (JSON.parse(raw.trim() || "[]").every((d) => d.severity !== "error")) clean++;
  }
  return clean;
}

/**
 * Each claim: a regex capturing the number in the README, and the truth.
 *
 * The regex is deliberately narrow. A loose one that matched several places would report the first
 * and leave the rest stale, which is the failure this replaces.
 */
const CLAIMS = [
  { what: "Rust tests", pattern: /(\d+) Rust tests/, actual: rustTests },
  { what: "diagnostic codes", pattern: /(\d+) diagnostic codes/, actual: diagnosticCodes },
  { what: "fixtures", pattern: /(\d+) fixtures render/, actual: () => fixtures },
  { what: "tags", pattern: /(\d+) primitives/, actual: tags },
  // The two numbers the README now leads with. Both are derived, both move when the compiler
  // changes, and both are the kind of figure that reads correctly long after it stopped being true.
  {
    what: "compression ratio",
    pattern: /\*\*([\d.]+)× fewer estimated tokens/,
    actual: compressionRatio,
    compare: (claimed, actual) => Math.abs(claimed - actual) < 0.15,
  },
  {
    what: "generations that compile",
    pattern: /(\d+) of 6 compiled/,
    actual: compiledCount,
  },
];

let failures = 0;
for (const claim of CLAIMS) {
  const found = readme.match(claim.pattern);
  if (!found) {
    console.error(`  MISSING  no "${claim.what}" claim matched ${claim.pattern} — did the wording change?`);
    failures++;
    continue;
  }
  const claimed = Number(found[1]);
  const actual = claim.actual();
  const same = claim.compare ? claim.compare(claimed, actual) : claimed === actual;
  if (!same) {
    console.error(`  STALE    README says ${claimed} ${claim.what}; there are ${actual}`);
    failures++;
  } else {
    console.log(`  ok       ${actual} ${claim.what}`);
  }
}

if (failures) {
  console.error(`\n${failures} claim(s) wrong. Update README.md.`);
  process.exit(1);
}
console.log(`\n${CLAIMS.length} README claims match the repository`);
