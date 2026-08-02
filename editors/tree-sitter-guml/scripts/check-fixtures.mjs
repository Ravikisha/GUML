#!/usr/bin/env node
/* Parse every real `.guml` document in the repository and fail on a single ERROR or MISSING node.
 *
 * # Why this exists alongside the corpus
 *
 * `test/corpus/basics.txt` is twelve documents someone wrote to demonstrate features, and it passed while
 * five separate bugs were live — every one of them found by pointing the parser at `fixtures/` instead.
 * Two are worth naming because they show the pattern:
 *
 *   * Two top-level siblings nested, because at depth zero the scanner emitted INDENT for *any* line.
 *     Invisible to the corpus: every case had a `page` directive before the first indent, and a directive
 *     has no body, so `valid_symbols[INDENT]` was false and the broken branch never ran.
 *   * `divider` — a text-kind tag that normally carries no text — produced no token at all, so the line
 *     could not be parsed. No corpus case had a bare text tag.
 *
 * A hand-written corpus can only contain cases someone thought of. The fixtures are the documents the
 * compiler is actually tested against, so agreement with them is the property that matters: by the rule
 * at the top of `grammar.js`, a document the compiler accepts and this grammar rejects is a bug here.
 *
 * Not the whole story either — zero ERROR nodes is not the same as the *right* tree, which is what the
 * corpus is for. Both, or neither is enough.
 */
import { execFileSync } from "node:child_process";
import { readdirSync, existsSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const grammarDir = resolve(here, "..");
const root = resolve(here, "../../..");

// The binary directly rather than `npx`, and not via a shell. `npx tree-sitter` costs a resolution per
// file, prints a "you have not configured any parser directories" warning per file, and passing arguments
// through `shell: true` is a Node deprecation warning of its own.
const bin = join(
  grammarDir,
  "node_modules/tree-sitter-cli",
  process.platform === "win32" ? "tree-sitter.exe" : "tree-sitter",
);
if (!existsSync(bin)) {
  console.error(`check-fixtures: no tree-sitter binary at ${bin} — run \`npm install\` in ${grammarDir}`);
  process.exit(1);
}

// Directories of documents that are known-good GUML. Generated model output (`bench/gen/out`) is
// deliberately excluded: those files are *supposed* to contain mistakes — that is what the repair loop is
// measured on — so a parse error there is data, not a regression.
const dirs = ["fixtures", "bench/phase0/examples", "bench/guml-bench/reference"];

const files = dirs
  .map((d) => join(root, d))
  .filter((d) => existsSync(d))
  .flatMap((d) =>
    readdirSync(d)
      .filter((f) => f.endsWith(".guml"))
      .map((f) => join(d, f)),
  );

if (files.length === 0) {
  console.error("check-fixtures: found no .guml documents — the directory list is wrong");
  process.exit(1);
}

let bad = 0;
for (const file of files) {
  const label = relative(root, file).replaceAll("\\", "/");
  let tree;
  try {
    tree = execFileSync(bin, ["parse", file], {
      cwd: grammarDir,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
  } catch (err) {
    // A parse with errors exits non-zero *and* prints the tree on stdout, which is the part we want.
    tree = err.stdout ?? "";
  }
  const lines = tree.split("\n").filter((l) => /\(ERROR|MISSING/.test(l));
  if (lines.length > 0) {
    bad++;
    console.error(`${label}: ${lines.length} bad node(s)`);
    for (const l of lines.slice(0, 5)) console.error(`  ${l.trim()}`);
  } else {
    console.log(`${label}: ok`);
  }
}

if (bad > 0) {
  console.error(
    `\n${bad} of ${files.length} documents do not parse. The compiler accepts all of them, so the ` +
      `grammar is what is wrong — see the rule at the top of grammar.js.`,
  );
  process.exit(1);
}
console.log(`\n${files.length} documents, no ERROR or MISSING nodes.`);
