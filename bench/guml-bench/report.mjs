#!/usr/bin/env node
/**
 * GUML-Bench report: the numbers this repository can produce without an API key.
 *
 * # What this reports on
 *
 * Every task with a `fixture` has an authored GUML document in `fixtures/`, which means the *representation*
 * side of the comparison can be measured right now — compile rate, emitted size, the accessibility
 * guarantees the compiler claims, escape-hatch rate — with no model in the loop. That is a real result and
 * it is the one a reviewer can reproduce from a checkout.
 *
 * It is emphatically **not** the result the report is after. The open question is whether a *model* can
 * produce correct GUML, and that needs generations. So this prints the authored-artifact numbers, labels
 * them as such in the output, and prints what is missing beside them.
 *
 * # Two rules the output obeys
 *
 * **No overall average.** The content floor makes one actively misleading: a landing page is mostly prose
 * and prose is incompressible, so its ratio asymptotes at 2–3× while a CRUD app approaches 8×. A single
 * mean over both describes neither and moves with the category mix. Per category, always.
 *
 * **Every figure carries its n.** A category with two tasks in it says `n=2` next to the number. The
 * report's target is 25, and a two-task median presented without its n is the specific way a thin dataset
 * becomes a false claim.
 *
 * Run from the repository root:
 *
 *   node bench/guml-bench/report.mjs
 *   node bench/guml-bench/report.mjs --json
 */
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { CATEGORIES, armStatus, coverage } from "./schema.mjs";
import { TASKS } from "./tasks.mjs";
import { claimedGuarantees, compileMetrics, median } from "./metrics.mjs";
import { approxTokens } from "./metrics.mjs";
import { encode as toonEncode, uniformity } from "./toon.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..");
const asJson = process.argv.includes("--json");

const measured = [];
// Every task, not only the three that reuse a repository fixture. `report` measured 3 of 12 for as long as
// nine tasks had no authored answer, and printed the coverage beside it — which was honest and thin. The
// nine references in `bench/guml-bench/reference/` closed that, and writing them found ten compiler
// defects, all of them silent. See the header of `tasks.mjs`.
for (const task of TASKS) {
  const source = readFileSync(join(ROOT, task.reference), "utf8");
  const react = compileMetrics(ROOT, source, { backend: "react" });
  const html = compileMetrics(ROOT, source, { backend: "html" });
  const a2ui = compileMetrics(ROOT, source, { backend: "a2ui" });

  // Arm B4. The *same* payload as B3, re-encoded — which is the only version of this comparison that
  // answers the objection it exists for. A hand-tuned TOON structure would measure the tuning.
  let toon = null;
  let toonUniformity = null;
  if (a2ui.emitted) {
    try {
      const doc = JSON.parse(a2ui.emitted);
      toon = approxTokens(toonEncode(doc));
      toonUniformity = uniformity(doc);
    } catch {
      // A payload that is not JSON is a compiler bug, not a reason for this arm to guess.
      toon = null;
    }
  }

  // The compression figure, and it needs its denominator stated. This is *emitted* React against
  // *authored* GUML — which is the ratio the report's §1.5 measures, and it is a claim about the
  // representations rather than about any model.
  const ratio =
    react.approxEmittedTokens && react.approxSourceTokens
      ? Number((react.approxEmittedTokens / react.approxSourceTokens).toFixed(2))
      : null;

  measured.push({
    id: task.id,
    category: task.category,
    reference: task.reference,
    parses: react.parses,
    errorCodes: react.errorCodes,
    escapeHatches: react.escapeHatches,
    approxSourceTokens: react.approxSourceTokens,
    approxReactTokens: react.approxEmittedTokens,
    approxHtmlTokens: html.approxEmittedTokens,
    approxA2uiTokens: a2ui.approxEmittedTokens,
    approxToonTokens: toon,
    toonTabularRowShare: toonUniformity ? toonUniformity.tabularRowShare : null,
    ratioVsReact: ratio,
    // The static-HTML output is the one that can be inspected without a DOM, which is why the guarantee
    // checks run against it rather than against the JSX.
    guarantees: html.emitted ? claimedGuarantees(html.emitted) : null,
  });
}

/* ------------------------------------------------------------------- output */

const cover = coverage(TASKS);
const { available, unavailable } = armStatus();
const byCategory = CATEGORIES.map((c) => {
  const rows = measured.filter((m) => m.category === c.id);
  const ratios = rows.map((r) => r.ratioVsReact).filter((r) => r !== null);
  return {
    ...c,
    measured: rows.length,
    medianRatio: median(ratios),
    ratios,
  };
});

if (asJson) {
  console.log(
    JSON.stringify(
      {
        counter: "approx (~3.6 chars/token). Never a published figure — see metrics.mjs.",
        what: "authored GUML against emitted output. No model in the loop.",
        coverage: cover.map(({ id, have, target }) => ({ id, have, target })),
        armsAvailable: available.map((a) => a.id),
        armsUnavailable: unavailable.map((a) => ({ id: a.id, reason: a.unavailable })),
        tasks: measured,
        byCategory: byCategory.map(({ id, measured: n, medianRatio }) => ({
          id,
          n,
          medianRatio,
        })),
        toonNote:
          "Arm B4 is the same A2UI payload re-encoded, not a separate structure. `toon.mjs` ships a decoder and `selftest.mjs` asserts the encoding round-trips on every payload here. Key folding and alternate delimiters are unimplemented, so these figures are a lower bound on how well TOON does.",
        ratioCaveat:
          "This ratio rises when the compiler generates more code. c01-tasks reads ~15.9x here against the 8.10x the report publishes for the same fixture, because the compiler gained a response cache. It is a size measurement, not a quality one, and is not comparable to the published cl100k figures.",
        missing: [
          "Every generation metric: parse rate from a model, repair rounds, USD, latency. Needs an API key.",
          "Playwright interaction pass rate, visual similarity, Lighthouse. Needs a browser.",
          "Human semantic-correctness scoring against each checklist, blind. Needs a grader.",
        ],
      },
      null,
      2,
    ),
  );
  process.exit(0);
}

console.log("GUML-Bench — authored-artifact report\n");
console.log("What this measures: authored GUML compiled to each target. **No model is involved.**");
console.log("Tokens are a ~3.6 chars/token estimate, never a published figure.\n");

console.log(`${"task".padEnd(20)}${"cat".padEnd(11)}${"GUML".padStart(6)}${"React".padStart(7)}${"HTML".padStart(7)}${"A2UI".padStart(7)}${"TOON".padStart(7)}${"ratio".padStart(8)}`);
console.log("-".repeat(73));
for (const m of measured) {
  console.log(
    `${m.id.padEnd(20)}${m.category.padEnd(11)}${String(m.approxSourceTokens).padStart(6)}${String(m.approxReactTokens ?? "—").padStart(7)}${String(m.approxHtmlTokens ?? "—").padStart(7)}${String(m.approxA2uiTokens ?? "—").padStart(7)}${String(m.approxToonTokens ?? "—").padStart(7)}${`${m.ratioVsReact ?? "—"}×`.padStart(8)}`,
  );
}

// The hazard that matters most, and it surfaced on this report's first run: `c01-tasks` reads 15.9× here
// against the 8.10× the report publishes for the *same fixture*. Nothing about the language changed. The
// compiler started generating a response cache, so the emitted side grew — and a ratio whose denominator is
// "code the compiler writes" goes **up** when the compiler writes more of it.
//
// That makes the ratio gameable by generating boilerplate, which is the opposite of a quality signal. It is
// printed with the warning attached rather than quietly, because a number that moves for the wrong reason is
// worse than no number at all.
console.log("\nCAVEAT: this ratio rises when the compiler generates *more* code.");
console.log("  `c01-tasks` reads 15.9× here against the 8.10× the report publishes for the same fixture.");
console.log("  Nothing about the language changed — the compiler gained a response cache, so the emitted");
console.log("  side grew. A ratio with 'code the compiler writes' as its numerator is gameable by writing");
console.log("  more of it, so it is a size measurement and not a quality one. The published figures use");
console.log("  cl100k on a fixed compiler revision; these use a ~3.6 chars/token estimate on the current");
console.log("  one. They are not comparable and must not be quoted together.");

console.log("\nPer category. **No overall average** — the content floor makes one misleading.\n");
for (const c of byCategory) {
  const n = c.measured;
  const figure = c.medianRatio === null ? "—" : `${c.medianRatio.toFixed(2)}×`;
  const warn = n === 0 ? "no fixture yet" : n < 5 ? `n=${n} of ${c.target} — not publishable` : `n=${n}`;
  console.log(`  ${c.id.padEnd(11)} ${figure.padStart(7)}   ${warn}`);
  console.log(`  ${" ".repeat(11)} ${" ".repeat(7)}   expected: ${c.expectation}`);
}

console.log(`\nA2UI is the arm that answers "why not just emit JSON".`);
const withA2ui = measured.filter((m) => m.approxA2uiTokens);
if (withA2ui.length > 0) {
  const shares = withA2ui.map((m) => m.approxSourceTokens / m.approxA2uiTokens);
  console.log(
    `  GUML is ${(100 - median(shares) * 100).toFixed(0)}% smaller than the A2UI payload for the same document (n=${withA2ui.length}).`,
  );
}

// Arm B4, and this is the paragraph a hostile reviewer reads first — so it states the rival's result before
// GUML's, and states what would make the rival look better.
const withToon = measured.filter((m) => m.approxToonTokens && m.approxA2uiTokens);
if (withToon.length > 0) {
  const vsJson = median(withToon.map((m) => m.approxToonTokens / m.approxA2uiTokens));
  const vsGuml = median(withToon.map((m) => m.approxSourceTokens / m.approxToonTokens));
  const share = median(withToon.map((m) => m.toonTabularRowShare ?? 0));
  console.log(`\nB4 — TOON, the same IR as B3 re-encoded. This is the arm that could sink the thesis.`);
  console.log(
    `  TOON is ${(100 - vsJson * 100).toFixed(0)}% smaller than the JSON of the identical payload, so the ` +
      `"just use a compact\n  serialisation" objection is real and worth this much.`,
  );
  console.log(
    `  GUML is still ${(100 - vsGuml * 100).toFixed(0)}% smaller than the TOON (n=${withToon.length}), which ` +
      `is the answer: the saving is\n  structural, not a property of the punctuation.`,
  );
  console.log(
    `  Caveat in TOON's favour: its tabular form reaches only ${(share * 100).toFixed(0)}% of object rows in ` +
      `this IR, because the\n  IR's own arrays are not uniform. That is a fact about the payload's shape, not ` +
      `about the format —\n  and key folding and alternate delimiters are unimplemented here, so this is a ` +
      `lower bound on TOON.`,
  );
}

const hatches = measured.reduce((a, m) => a + m.escapeHatches, 0);
console.log(
  `\nEscape hatches across the measured set: ${hatches}. A rising rate is the early warning that the`,
);
console.log("vocabulary is hitting an expressiveness cliff.");

console.log("\nWhat is missing, and why:");
console.log("  · every generation metric — parse rate from a model, repair rounds, USD, latency: API key");
console.log("  · Playwright pass rate, visual similarity, Lighthouse: a browser");
console.log("  · blind human scoring against each checklist: a grader");
for (const a of unavailable) {
  console.log(`  · arm ${a.id}: ${a.unavailable}`);
}
console.log(
  `\nDataset: ${cover.reduce((a, c) => a + c.have, 0)} of ${cover.reduce((a, c) => a + c.target, 0)} tasks; ${measured.length} have an authored fixture to measure.`,
);
