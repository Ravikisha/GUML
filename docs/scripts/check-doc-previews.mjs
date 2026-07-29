/**
 * Every docs sample that carries a live preview must compile clean.
 *
 * The preview on a docs page is rendered by the real compiler in the reader's browser, so a sample
 * that stopped compiling would not break the build — it would quietly replace an interface with a
 * diagnostics panel on a page arguing that the language works. This turns that into a build failure.
 *
 * Warnings fail too, not just errors. A warning is almost always the *scaffold's* fault — a
 * declaration supplied for the preview that the sample does not actually use — and the preview header
 * displays the count, so "3 warnings" would appear under a paragraph that has nothing to do with them.
 *
 * Run: node --experimental-strip-types --no-warnings scripts/check-doc-previews.mjs
 */

import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { SAMPLES } from "../lib/samples.ts";

const repoRoot = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..");
const work = mkdtempSync(join(tmpdir(), "guml-doc-previews-"));

/** Exactly what `CodePreview` compiles, so the check cannot pass on a different string. */
function previewSource({ code, scaffold }) {
  const needsPage = !/^\s*page\s/m.test(code);
  const preamble = [
    needsPage && !scaffold?.trimStart().startsWith("page") ? "page Preview" : null,
    scaffold?.trim(),
  ]
    .filter(Boolean)
    .join("\n");
  return preamble ? `${preamble}\n\n${code}` : code;
}

let failed = 0;
const ids = Object.keys(SAMPLES);
console.log(`checking ${ids.length} previewable docs samples\n`);

for (const id of ids) {
  const source = previewSource(SAMPLES[id]);
  const path = join(work, `${id.replace(/\W/g, "_")}.guml`);
  writeFileSync(path, source, "utf8");

  let diagnostics;
  try {
    const out = execFileSync(
      "cargo",
      ["run", "-q", "-p", "guml-cli", "--", "check", path, "--format", "json"],
      { cwd: repoRoot, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    );
    diagnostics = JSON.parse(out);
  } catch (err) {
    // A non-zero exit is itself the answer when the document does not parse.
    try {
      diagnostics = JSON.parse(err.stdout ?? "[]");
    } catch {
      console.error(`✗ ${id}\n    could not run the compiler: ${err.message}`);
      failed++;
      continue;
    }
  }

  const bad = diagnostics.filter((d) => d.severity === "error" || d.severity === "warning");
  if (bad.length === 0) {
    console.log(`✓ ${id}`);
    continue;
  }

  failed++;
  console.error(`✗ ${id}`);
  for (const d of bad) {
    console.error(`    ${d.severity} ${d.id} line ${d.span.line}: ${d.message}`);
  }
  // The line numbers above are in the scaffolded document, so print it once to locate them.
  console.error(source.split("\n").map((l, i) => `    ${String(i + 1).padStart(3)} | ${l}`).join("\n"));
}

rmSync(work, { recursive: true, force: true });

if (failed) {
  console.error(`\n${failed} of ${ids.length} previewable samples do not compile clean.`);
  process.exit(1);
}
console.log(`\n${ids.length} previewable samples compile with no errors and no warnings`);
