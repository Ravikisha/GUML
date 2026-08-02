#!/usr/bin/env node
/**
 * Self-test for the parts of GUML-Bench that compute a number.
 *
 * Modelled on `bench/phase0/selftest.mjs` and there for the same reason: the harness produces figures that
 * end up in a paper, so the harness itself needs to be tested by something other than the figures looking
 * plausible.
 *
 * Right now that means the TOON encoder for arm B4, and it is the most important thing in this directory to
 * test — because it is the *rival's* arm. A bug that makes GUML look better is not a bug anyone will report.
 * Two properties:
 *
 *   1. **Lossless.** Encode, decode, deep-compare against the original. On every A2UI payload the report
 *      measures, plus a set of hand-written edge cases. Without this, "TOON is 31% smaller" is
 *      indistinguishable from "we deleted 31% of the characters".
 *   2. **Actually using the tabular form.** If `tabularKeys` silently stopped firing, the arm would report
 *      TOON as *worse* than it is and the comparison would flatter GUML. Asserted directly on a uniform
 *      array, so the feature cannot regress into a no-op.
 *
 * Run from the repository root:
 *
 *   node bench/guml-bench/selftest.mjs
 */
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { deepStrictEqual } from "node:assert";

import { TASKS } from "./tasks.mjs";
import { decode, encode, tabularKeys, uniformity } from "./toon.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..");

let failed = 0;
const check = (name, fn) => {
  try {
    fn();
    console.log(`  ok    ${name}`);
  } catch (err) {
    failed++;
    console.error(`  FAIL  ${name}\n        ${err.message.split("\n")[0]}`);
  }
};

const roundTrips = (value) => deepStrictEqual(decode(encode(value)), value);

console.log("GUML-Bench selftest\n");
console.log("TOON encoder — edge cases");

// Each of these is a way an encoder loses information while looking smaller. The string cases are the
// dangerous ones: `"true"` and `true` must not both come out bare, or a decoder cannot tell them apart.
check("scalars keep their types", () =>
  roundTrips({ s: "text", n: 42, f: 1.5, t: true, f2: false, z: null }),
);
check("a string that looks like a literal stays a string", () =>
  roundTrips({ a: "true", b: "false", c: "null", d: "42", e: "1.5", f: "-" }),
);
check("delimiters and quotes inside a value survive", () =>
  roundTrips({ a: "one,two", b: 'say "hi"', c: "a: b", d: "x[0]", e: "y{z}" }),
);
check("leading and trailing space is not eaten", () => roundTrips({ a: " pad ", b: "", c: "  " }));
check("an empty array and an empty object", () => roundTrips({ a: [], b: {}, c: { d: [] } }));
check("a scalar array stays inline and ordered", () => roundTrips({ xs: [3, 1, 2, "a", null] }));
check("nested objects", () => roundTrips({ a: { b: { c: 1 } }, d: { e: "f" } }));
check("a non-uniform object array", () =>
  roundTrips({ xs: [{ id: "a", text: "hi" }, { id: "b", children: ["c", "d"] }, { id: "c" }] }),
);
check("a uniform object array uses the tabular form", () => {
  const rows = [
    { name: "aria", value: "New task", bound: false },
    { name: "placeholder", value: "Add a task…", bound: false },
  ];
  deepStrictEqual(tabularKeys(rows), ["name", "value", "bound"]);
  const text = encode({ properties: rows });
  // The header declares the keys once, and they do not appear again.
  if (!text.includes("properties[2]{name,value,bound}:")) {
    throw new Error(`tabular header missing:\n${text}`);
  }
  if (text.split("bound").length - 1 !== 1) {
    throw new Error(`a key repeated per row — the tabular form is not firing:\n${text}`);
  }
  roundTrips({ properties: rows });
});
check("an array whose rows differ by one key is NOT tabular", () => {
  // Strictness matters: encoding this tabularly would need a hole marker, and inventing one produces a
  // dialect no TOON decoder reads.
  deepStrictEqual(tabularKeys([{ a: 1, b: 2 }, { a: 3 }]), null);
  deepStrictEqual(tabularKeys([{ a: 1, b: 2 }, { b: 2, a: 1 }]), null);
  deepStrictEqual(tabularKeys([{ a: 1, b: { c: 2 } }, { a: 3, b: { c: 4 } }]), null);
});

console.log("\nTOON encoder — every payload the report measures");

let jsonChars = 0;
let toonChars = 0;
let tabularRows = 0;
let totalRows = 0;

for (const task of TASKS) {
  const source = readFileSync(join(ROOT, task.reference), "utf8");
  let json;
  try {
    json = execFileSync(
      "cargo",
      ["run", "-q", "-p", "guml-cli", "--", "build", task.reference, "--backend", "a2ui"],
      { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    );
  } catch {
    failed++;
    console.error(`  FAIL  ${task.id} — the a2ui backend did not produce a payload`);
    continue;
  }
  void source;
  const doc = JSON.parse(json);
  const toon = encode(doc);
  const u = uniformity(doc);
  jsonChars += json.length;
  toonChars += toon.length;
  tabularRows += u.tabularRows;
  totalRows += u.rows;
  check(`${task.id} round-trips`, () => deepStrictEqual(decode(toon), doc));
}

const saving = jsonChars === 0 ? 0 : 100 - (100 * toonChars) / jsonChars;
console.log(
  `\nAcross ${TASKS.length} payloads: TOON is ${saving.toFixed(1)}% fewer characters than the JSON.`,
);
console.log(
  `Tabular form reaches ${tabularRows} of ${totalRows} object rows ` +
    `(${((100 * tabularRows) / (totalRows || 1)).toFixed(0)}%) — the rest fall back to list form,\n` +
    `so most of that saving is dropped punctuation rather than TOON's headline feature. That is a fact\n` +
    `about this IR's shape, not about the format.`,
);

if (failed > 0) {
  console.error(`\n${failed} check(s) failed`);
  process.exit(1);
}
console.log("\nselftest passed");
