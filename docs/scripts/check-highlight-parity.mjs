/**
 * Fails if the site's highlighter and the compiler's classifier disagree.
 *
 * The site cannot call wasm during server rendering of every inline snippet, so it keeps a
 * TypeScript tokeniser. That is a second implementation of a language rule, which is the
 * exact thing this project treats as a defect everywhere else — so it is held to the
 * compiler's answer mechanically instead of by review.
 *
 * The vocabulary is already generated from `guml registry`; this covers the *rules*: which
 * words are prose, where a content run starts, what an unknown tag looks like.
 *
 *   node scripts/check-highlight-parity.mjs
 */
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { highlight } from "../lib/highlight.ts";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..", "..");

/** Every GUML document in the repository, so the corpus grows with the project. */
function corpus() {
  const out = [];
  for (const [dir, filter] of [
    [join(repoRoot, "fixtures"), (f) => f.endsWith(".guml")],
    [join(repoRoot, "bench", "phase0", "examples"), (f) => f.endsWith(".guml")],
  ]) {
    let files = [];
    try {
      files = readdirSync(dir).filter(filter);
    } catch {
      continue;
    }
    for (const f of files) out.push([join(dir, f), readFileSync(join(dir, f), "utf8")]);
  }
  return out;
}

/**
 * The compiler reports *byte* offsets; JS strings are UTF-16. `—` is one JS char and three
 * bytes, so slicing the string with those offsets silently shreds every line containing an
 * em dash. Anything consuming the wasm `highlight()` spans has to do this too — which is
 * why the package also exposes pre-sliced rows.
 */
function sliceBytes(buf, start, end) {
  return buf.subarray(start, end).toString("utf8");
}

function rustSpans(path) {
  const json = execFileSync(
    "cargo",
    ["run", "-q", "-p", "guml-cli", "--", "highlight", path],
    { cwd: repoRoot, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"], maxBuffer: 8 << 20 },
  );
  return JSON.parse(json);
}

/** The TS side yields per-line runs; flatten to the same (text, class) shape. */
function tsSpans(src) {
  const out = [];
  for (const row of highlight(src.replace(/\r\n/g, "\n"), "guml")) {
    for (const tok of row) {
      // `plain` is the absence of a class — the compiler simply emits no span for the
      // whitespace between tokens.
      if (tok.cls === "plain" || tok.text.trim() === "") continue;
      out.push({ text: tok.text, class: tok.cls });
    }
  }
  return out;
}

let failures = 0;
let compared = 0;

for (const [path, src] of corpus()) {
  const buf = Buffer.from(src, "utf8");
  const rust = rustSpans(path)
    .map((s) => ({ text: sliceBytes(buf, s.start, s.end), class: s.class }))
    .filter((s) => s.text.trim() !== "");
  const ts = tsSpans(src);

  const n = Math.max(rust.length, ts.length);
  const diffs = [];
  for (let i = 0; i < n; i++) {
    const a = rust[i];
    const b = ts[i];
    if (!a || !b || a.text !== b.text || a.class !== b.class) {
      diffs.push({
        at: i,
        compiler: a ? `${a.class}(${JSON.stringify(a.text)})` : "—",
        site: b ? `${b.class}(${JSON.stringify(b.text)})` : "—",
      });
      if (diffs.length >= 5) break;
    }
  }

  compared += Math.min(rust.length, ts.length);
  if (diffs.length > 0) {
    failures++;
    console.error(`\n✗ ${path.replace(repoRoot, ".")}`);
    for (const d of diffs) {
      console.error(`   #${d.at}  compiler: ${d.compiler}\n         site:     ${d.site}`);
    }
  }
}

if (failures > 0) {
  console.error(
    `\n${failures} file(s) disagree. Fix lib/highlight.ts to match crates/guml-fmt/src/highlight.rs —` +
      " the compiler is the source of truth, not the site.",
  );
  process.exit(1);
}
console.log(`highlighter parity: ${compared} spans agree across ${corpus().length} documents`);
