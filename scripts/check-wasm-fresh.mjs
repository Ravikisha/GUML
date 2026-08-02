/**
 * The committed wasm must match the Rust source it was built from.
 *
 * `packages/guml/wasm/` is a *committed build artifact*. That is deliberate — the docs site and any
 * consumer of the npm package need the compiler without a Rust toolchain, and Vercel does not have one.
 * But a committed artifact is a second source of truth, and it drifts silently: nothing about editing
 * `crates/guml-codegen/src/react.rs` makes the wasm stale in any way a test can see, because every Rust
 * test passes against the *source* and every JS test passes against the *artifact*.
 *
 * It had already drifted when this check was written. The committed binary was five days old, the
 * codegen had changed since, and `prepublishOnly` ran `build:ts` without `build:wasm` — so
 * `npm publish` would have shipped a compiler binary that did not match its own tag, and no gate in the
 * repository would have said a word.
 *
 * Rebuild and compare bytes. `wasm-pack` output is deterministic for a given source and toolchain, so a
 * mismatch means one of two things, and the message says which to suspect:
 *
 *   * the source changed and the artifact was not rebuilt — the common case, fix by rebuilding
 *   * the toolchain differs from the one that produced the committed artifact — pin it in CI
 *
 * Skipped when `wasm-pack` is absent, so a contributor without it can still run `just ci`; CI installs
 * it, and CI is where this has to hold.
 */

import { execFileSync, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";

const COMMITTED = "packages/guml/wasm/guml_bg.wasm";

const sha = (p) => createHash("sha256").update(readFileSync(p)).digest("hex");

function have(cmd) {
  try {
    execFileSync(cmd, ["--version"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

// Needs no Rust toolchain, so it runs unconditionally — and it is the check whose failure is worse.
assertWasmIsPacked();

if (!have("wasm-pack")) {
  console.log("wasm-pack not installed — skipping the freshness half of this check");
  console.log("  install: cargo install wasm-pack");
  process.exit(0);
}

/**
 * Second, independent property: the artifact has to actually be *in* the published tarball.
 *
 * `wasm-pack` writes a `.gitignore` containing `*` into its output directory, and npm honours a nested
 * `.gitignore` when no `.npmignore` sits beside it. That one line excluded the whole `wasm/` directory
 * from the pack even though `files` listed it explicitly and git tracked every file in it — so
 * `npm publish` would have shipped a compiler package containing no compiler, with `exports["./wasm"]`
 * resolving to a file that was not there. Broken on install, for everyone, and invisible to every test
 * in the repository because they all run against the working tree.
 *
 * Freshness and presence are different failures with different causes, so they are checked separately.
 */
function assertWasmIsPacked() {
  // `execSync` with one fixed command string, not `execFileSync` with an args array. On Windows `npm` is
  // a `.cmd` shim, which Node 24 refuses to spawn without a shell; passing an args array *with* a shell
  // is deprecated because the args get concatenated rather than escaped. A constant string with no
  // interpolation sidesteps both, and there is no untrusted input here to escape.
  const raw = execSync("npm pack --dry-run --json", {
    cwd: "packages/guml",
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  const packed = JSON.parse(raw)[0].files.map((f) => f.path);
  const missing = ["wasm/guml_bg.wasm", "wasm/guml.js", "wasm/guml.d.ts"].filter(
    (f) => !packed.includes(f),
  );

  if (missing.length) {
    console.error("the published `guml` tarball would not contain the compiler\n");
    for (const f of missing) console.error(`  missing  ${f}`);
    console.error(`\n  ${packed.length} files would be packed, none of the above among them.\n`);
    console.error("Almost certainly a `.gitignore` inside `packages/guml/wasm/`: wasm-pack writes");
    console.error("one containing `*`, and npm honours it over the `files` allowlist. `build:wasm`");
    console.error("deletes it after building — check that step still runs.");
    process.exit(1);
  }
  console.log(`the tarball contains the compiler (${packed.length} files packed)`);
}

const before = sha(COMMITTED);
const out = mkdtempSync(join(tmpdir(), "guml-wasm-"));

try {
  execFileSync(
    "wasm-pack",
    ["build", "--target", "web", "--out-dir", out, "--out-name", "guml"],
    { cwd: "crates/guml-wasm", stdio: ["ignore", "ignore", "inherit"] },
  );

  const after = sha(join(out, "guml_bg.wasm"));

  if (before !== after) {
    console.error("the committed wasm does not match the Rust source\n");
    console.error(`  committed  ${COMMITTED}`);
    console.error(`             ${before}`);
    console.error(`  rebuilt    ${after}\n`);
    console.error("Rebuild it and commit the result:\n");
    console.error("  cd packages/guml && pnpm build:wasm\n");
    console.error(
      "If you did not touch any Rust, your toolchain differs from the one that built the",
    );
    console.error("committed artifact — check `rustc --version` against the pinned CI version.");
    process.exit(1);
  }

  console.log(`the committed wasm matches its source (${before.slice(0, 16)}…)`);
} finally {
  rmSync(out, { recursive: true, force: true });
}
