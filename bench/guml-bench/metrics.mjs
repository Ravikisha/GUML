/**
 * GUML-Bench metrics.
 *
 * # What is measured here, and what is deliberately not
 *
 * This module measures everything that can be measured **without an API key and without a browser**:
 * token counts, compile and parse rates, emitted-bundle size, and the accessibility and structural
 * properties the compiler claims to guarantee. Those are the numbers a reviewer can reproduce from this
 * repository, so they are the numbers this file is careful about.
 *
 * What it does not measure, and says so rather than approximating:
 *
 * * **USD and latency.** Both are properties of a generation, and a generation needs an API key.
 * * **Visual similarity.** Needs a browser and reference screenshots.
 * * **Playwright interaction pass rate.** Needs a browser.
 * * **Lighthouse.** Needs a browser.
 *
 * Each of those returns `null` with a stated reason rather than a plausible-looking substitute. A
 * benchmark that reports an estimate where it promised a measurement is worse than one with a gap in it,
 * because the gap is visible and the estimate is not.
 *
 * # Tokens
 *
 * `approxTokens` is a ~3.6 chars/token heuristic and is labelled `approx` at every call site and in every
 * output field. The project's rule (`CLAUDE.md`, claim discipline) is that a published figure comes from
 * the target model's own tokenizer, and `tiktoken` is an OpenAI tokenizer that undercounts Claude — so it
 * is not used here at all, not even as a cross-check, because a number in a JSON file gets quoted.
 *
 * The authoritative counter is Anthropic's `count_tokens`, which needs a key; `run.mjs` fills the exact
 * columns in when it has one, and `report.mjs` prints which counter produced each figure.
 */
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export const APPROX_CHARS_PER_TOKEN = 3.6;

/** A ~3.6 chars/token estimate. Never a published figure — see the module docs. */
export function approxTokens(text) {
  return Math.ceil(text.length / APPROX_CHARS_PER_TOKEN);
}

/** Something this harness cannot measure, with the reason attached. */
export function unmeasured(reason) {
  return { value: null, reason };
}

/**
 * Run the compiler over a document and collect everything that does not need a browser.
 *
 * `root` is the repository root. Shelling out to the real CLI rather than linking the compiler: the number
 * that matters is what *the shipped tool* does with the document, and a harness that used a private API
 * could report a compile success the CLI would not reproduce.
 */
export function compileMetrics(root, source, { backend = "react", registry = null } = {}) {
  const scratch = mkdtempSync(join(tmpdir(), "guml-bench-"));
  const file = join(scratch, "doc.guml");
  writeFileSync(file, source.endsWith("\n") ? source : `${source}\n`);
  try {
    const guml = (args) => {
      try {
        return {
          ok: true,
          out: execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", ...args], {
            cwd: root,
            encoding: "utf8",
            stdio: ["ignore", "pipe", "pipe"],
          }),
        };
      } catch (e) {
        // A non-zero exit means diagnostics; stdout still holds the answer.
        return { ok: false, out: e.stdout?.toString() ?? "" };
      }
    };

    const registryArgs = registry ? ["--registry", registry] : [];
    const checked = guml(["check", file, "--format", "json", ...registryArgs]);
    const diagnostics = JSON.parse(checked.out.trim() || "[]");
    const errors = diagnostics.filter((d) => d.severity === "error");
    const warnings = diagnostics.filter((d) => d.severity === "warning");

    // The escape-hatch rate. A rising number is the early warning that the vocabulary is hitting an
            // expressiveness cliff, and it is only a signal if it is recorded per task rather than in aggregate.
    const escapes = diagnostics.filter((d) => d.id === "GUML0090").length;

    const built = errors.length === 0 ? guml(["build", file, "--backend", backend, ...registryArgs]) : null;
    const emitted = built?.ok ? built.out : "";

    return {
      parses: errors.length === 0,
      errorCount: errors.length,
      warningCount: warnings.length,
      // Codes rather than messages: a code is stable across releases and a message is not, so a run from
      // six months ago stays comparable.
      errorCodes: [...new Set(errors.map((d) => d.id))].sort(),
      escapeHatches: escapes,
      approxSourceTokens: approxTokens(source),
      approxEmittedTokens: emitted ? approxTokens(emitted) : null,
      emittedBytes: emitted.length || null,
      emittedLines: emitted ? emitted.split("\n").length : null,
      // The output itself, so a caller can assert on it without compiling twice. `claimedGuarantees` wants
      // the HTML text, and re-running the compiler to get it would double the report's runtime for nothing.
      emitted: emitted || null,
    };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

/**
 * The accessibility and structural properties the compiler *claims* to guarantee, checked on emitted
 * HTML.
 *
 * Not a substitute for axe-core, and the field names say so. axe-core needs a DOM; these are the specific
 * promises this project makes in prose, turned into assertions — which is the more interesting measurement
 * anyway, because a generic a11y audit does not know that GUML claims a `btn` can never lack a name.
 */
export function claimedGuarantees(html) {
  const count = (re) => (html.match(re) ?? []).length;
  const buttons = count(/<button\b/g);
  const namedButtons = count(/<button\b[^>]*aria-label=/g);
  // A button with text content is named by it, so an unnamed button is one with neither.
  const emptyButtons = count(/<button\b[^>]*>\s*<\/button>/g);
  const inputs = count(/<input\b/g);
  const namedInputs = count(/<input\b[^>]*aria-label=/g);
  const images = count(/<img\b/g);
  const altImages = count(/<img\b[^>]*\balt=/g);

  return {
    // Every control has an accessible name. `GUML0050`/`GUML0051` make this a compile error, so a failure
    // here is a codegen bug rather than a model failure — which is exactly why it is worth measuring.
    controlsNamed: inputs === 0 || namedInputs === inputs,
    buttonsNamed: buttons === 0 || emptyButtons === 0,
    imagesHaveAlt: images === 0 || altImages === images,
    // One h1 per page. `GUML0073` warns on more.
    singleH1: count(/<h1\b/g) <= 1,
    // A landmark a screen reader can navigate by. The static-HTML backend shipped without any for a while,
    // because three backends had drifted apart on which element a `nav` becomes.
    hasLandmark: /<(nav|header|main|footer|aside)\b/.test(html),
    counts: { buttons, inputs, images },
  };
}

/** Emitted-bundle size, from the compiler's own output. */
export function bundleSize(root, source, backend) {
  const m = compileMetrics(root, source, { backend });
  return m.emittedBytes;
}

/** Every `.guml` under a directory, sorted, so a run is reproducible. */
export function gumlFiles(dir) {
  return readdirSync(dir)
    .filter((f) => f.endsWith(".guml"))
    .sort()
    .map((f) => ({ name: f, source: readFileSync(join(dir, f), "utf8") }));
}

/**
 * Inter-run variance for one arm.
 *
 * The report asks for this and it is easy to skip. Without it, "GUML parsed 92% of the time" is a number
 * with no error bar, and a reviewer cannot tell 92% from 92% ± 15%. Returns nulls rather than zeros for a
 * single sample: a variance of zero over one run is a false statement, not a small one.
 */
export function spread(values) {
  const n = values.length;
  if (n === 0) return { n: 0, mean: null, min: null, max: null, stdev: null };
  const mean = values.reduce((a, b) => a + b, 0) / n;
  if (n === 1) {
    return { n, mean, min: values[0], max: values[0], stdev: null };
  }
  const variance = values.reduce((a, v) => a + (v - mean) ** 2, 0) / (n - 1);
  return {
    n,
    mean,
    min: Math.min(...values),
    max: Math.max(...values),
    stdev: Math.sqrt(variance),
  };
}

/**
 * The median, which is what the report's gate is stated in.
 *
 * A mean over a handful of tasks moves with one outlier, and the compression gate ("median output-token
 * reduction ≥3× on structure-heavy tasks") was written as a median for that reason.
 */
export function median(values) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}
