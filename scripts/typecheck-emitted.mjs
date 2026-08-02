#!/usr/bin/env node
/**
 * Typecheck the compiler's own output.
 *
 * The strongest available test of a code generator: emit every fixture, then run `tsc --strict` over
 * the result. Real bugs found this way that no Rust-side assertion could have caught — a missing JSX
 * fragment around multi-root pages, a layout attribute emitted as a DOM prop, and `<div>…</dl>` from a
 * mismatched open/close pair.
 *
 * # Why this replaced the shell script
 *
 * `scripts/typecheck-emitted.sh` needed bash, `TMPDIR`, and `wc`. On a Windows checkout it simply did
 * not run, which meant the single strongest check on the code generator was silently absent for anyone
 * developing there — and this project's own CI runs tests on Windows. Node is already a hard dependency
 * (the docs site, the npm package, `render-emitted.mjs`), so requiring it costs nothing that was not
 * already required.
 *
 * Driving the TypeScript compiler through its API rather than shelling out to `tsc -p` also removes the
 * `cd docs && --typeRoots …` dance the shell version needed to find its `@types`.
 *
 * Run from the repository root:
 *
 *   node scripts/typecheck-emitted.mjs
 */
import { execFileSync } from "node:child_process";
import { mkdirSync, readdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DOCS = join(ROOT, "docs");

// `pathToFileURL`, not a bare path: on Windows `C:\…` looks to the ESM loader like a URL whose scheme
// is `c:`, and it refuses it. Same reasoning as `render-emitted.mjs`.
const { default: ts } = await import(
  pathToFileURL(join(DOCS, "node_modules", "typescript", "lib", "typescript.js")).href
);

// Emitted inside the docs tree, not in a temp directory.
//
// TypeScript resolves `import … from "react"` by walking up from the *importing file's* own directory,
// so a component in `TMPDIR` finds no `node_modules` at all and every prop degrades to an implicit
// `any` — 50 spurious errors that say nothing about the generated code. `render-emitted.mjs` writes
// into the docs tree for exactly this reason. The shell version papered over it with
// `cd docs && --typeRoots node_modules/@types`, which worked only because `@types/react` then loaded as
// an *ambient* declaration; that is a fragile way to get types for a module the resolver never found.
const out = join(DOCS, ".guml-emitted");
try {
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  // `bench/guml-bench/reference` as well as `fixtures`, and that is where this check earns the most.
  //
  // A reference answer is a *whole task*, so it uses constructs no fixture does — a `js` block whose
  // `const` a binding reads, a state interpolated into a request URL, a two-step aggregate. Every one of
  // those is emitted TypeScript the Rust tests cannot see the type of; `tsc --strict` can. The URL
  // interpolation bug (`?channel={channel}` reaching `fetch` with its braces intact) was invisible to the
  // compiler, and so was the dependency array that then went stale.
  const sources = [
    ["fixtures", readdirSync(join(ROOT, "fixtures"))],
    ["bench/guml-bench/reference", readdirSync(join(ROOT, "bench", "guml-bench", "reference"))],
  ].flatMap(([dir, names]) =>
    names.filter((f) => f.endsWith(".guml")).map((f) => join(dir, f)),
  );
  if (sources.length === 0) throw new Error("no .guml documents found");

  for (const source of sources) {
    execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", "build", source, "-o", out], {
      cwd: ROOT,
      encoding: "utf8",
      stdio: ["ignore", "ignore", "inherit"],
    });
  }

  const emitted = readdirSync(out).filter((f) => f.endsWith(".tsx"));
  if (emitted.length === 0) throw new Error("the compiler emitted no .tsx files");
  console.log(`typechecking ${emitted.length} emitted components…`);

  // `strict` is the whole point: it is what caught a missing JSX fragment and a layout attribute
  // emitted as a DOM prop. `skipLibCheck` applies to React's own declarations, not to the emitted code.
  const options = {
    target: ts.ScriptTarget.ES2022,
    lib: ["lib.es2022.d.ts", "lib.dom.d.ts"],
    jsx: ts.JsxEmit.ReactJSX,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Bundler,
    strict: true,
    noEmit: true,
    skipLibCheck: true,
  };

  const program = ts.createProgram(
    emitted.map((f) => join(out, f)),
    options,
  );
  const diagnostics = ts.getPreEmitDiagnostics(program);

  if (diagnostics.length > 0) {
    const host = {
      getCanonicalFileName: (f) => f,
      getCurrentDirectory: () => ROOT,
      getNewLine: () => "\n",
    };
    console.error(ts.formatDiagnosticsWithColorAndContext(diagnostics, host));
    console.error(`${diagnostics.length} type error(s) in emitted output`);
    process.exit(1);
  }

  console.log("emitted output typechecks under --strict");
} finally {
  rmSync(out, { recursive: true, force: true });
}
