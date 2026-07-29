/**
 * The repair pipeline, layer by layer.
 *
 * Four layers, in increasing cost. Each one only sees what the cheaper layers could not fix,
 * and each reports what it changed — which is the point. "The repair loop works" is not a
 * claim worth making; "the model round was needed for 2 of 6 documents, and here is what the
 * free layers handled" is.
 *
 *   0. sanitize   strip fences, separators, trailing commentary   — free, no compiler
 *   1. format     `guml fmt`                                      — free, fixes whitespace
 *   2. fix        `guml fix`                                      — free, applies suggestions
 *   3. repair     one model round with the diagnostics attached    — costs a generation
 *
 * Layers 0–2 are deterministic and cost nothing, so anything they handle is a repair round
 * the project never pays for. That is the number this file exists to produce.
 */
import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SCRATCH = mkdtempSync(join(tmpdir(), "guml-pipeline-"));
let seq = 0;

function guml(args) {
  try {
    return execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", ...args], {
      cwd: process.env.GUML_ROOT ?? process.cwd(),
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (e) {
    // A non-zero exit means diagnostics were found; stdout still holds the answer.
    if (e.stdout) return e.stdout.toString();
    throw e;
  }
}

function scratchFile(source) {
  const path = join(SCRATCH, `doc-${seq++}.guml`);
  writeFileSync(path, source.endsWith("\n") ? source : `${source}\n`);
  return path;
}

/** Diagnostics for a source string, via the real compiler. */
export function diagnose(source) {
  const raw = guml(["check", scratchFile(source), "--format", "json"]).trim();
  const all = JSON.parse(raw || "[]");
  return {
    all,
    errors: all.filter((d) => d.severity === "error"),
    codes: [...new Set(all.filter((d) => d.severity === "error").map((d) => d.id))].sort(),
  };
}

/* ------------------------------------------------------------------ layer 0 */

const FENCE = /```(?:guml)?\s*\n([\s\S]*?)(?:```|$)/;
const SEPARATOR = /^\s*(-{3,}|\*{3,}|_{3,})\s*$/;

/**
 * Strip what is packaging rather than document: a code fence, markdown rules, and any
 * commentary the model appended after the last real line.
 *
 * Trailing commentary is found with the compiler rather than a regex for prose. A sentence
 * like "This page uses the tabs control" is indistinguishable from GUML by pattern — `This`
 * is a plausible tag name — but the compiler knows it is not, so the rule is: while the last
 * content line is the subject of an error, drop it. Only from the end, and bounded, so a
 * mistake in the middle of a document is never silently deleted.
 */
export function sanitize(source, { maxDropped = 12 } = {}) {
  const fenced = FENCE.exec(source);
  let text = (fenced ? fenced[1] : source).trim();
  const notes = { fenced: Boolean(fenced), separators: 0, droppedLines: 0 };

  const kept = text.split("\n").filter((line) => {
    if (SEPARATOR.test(line)) {
      notes.separators += 1;
      return false;
    }
    return true;
  });
  text = kept.join("\n").trim();

  for (let i = 0; i < maxDropped; i++) {
    const lines = text.split("\n");
    // Index of the last line with content.
    let last = lines.length - 1;
    while (last >= 0 && lines[last].trim() === "") last -= 1;
    if (last <= 0) break;

    const { errors } = diagnose(text);
    if (errors.length === 0) break;
    // `line` is 1-based.
    if (!errors.some((d) => d.span.line === last + 1)) break;

    lines.splice(last, 1);
    text = lines.join("\n").trim();
    notes.droppedLines += 1;
  }

  return { text: `${text}\n`, notes };
}

/* ---------------------------------------------------------------- layers 1-2 */

/** `guml fmt` — indentation, tabs, spacing. Never changes meaning. */
export function format(source) {
  return guml(["fmt", scratchFile(source)]);
}

/** `guml fix` — apply every unambiguous suggestion. No model call. */
export function autofix(source) {
  return guml(["fix", scratchFile(source)]);
}

/* ------------------------------------------------------------------ layer 3 */

/**
 * One model round, with the diagnostics attached.
 *
 * The diagnostics go in as compact JSON rather than prose: they are already designed as a
 * machine interface, and the whole argument for stable codes and real spans is that a model
 * can act on them without a human paraphrasing first.
 */
export function repairPrompt(source, diagnostics) {
  const compact = diagnostics.map((d) => ({
    code: d.id,
    line: d.span.line,
    col: d.span.col,
    message: d.message,
    ...(d.help ? { help: d.help } : {}),
  }));

  return `The document below does not compile. The compiler's diagnostics follow it.

Fix every reported problem and return the corrected document. Return the whole document, GUML
only, no commentary, no code fence.

--- document ---
${source.trim()}

--- diagnostics ---
${JSON.stringify(compact, null, 1)}`;
}

/* ------------------------------------------------------------------ the loop */

/**
 * Run a generation through every layer, recording what each one fixed.
 *
 * `askModel` is injected so this file has no opinion about which API is in use, and so the
 * layers can be tested without spending a generation.
 */
export async function repair(raw, { askModel = null, trials = 1 } = {}) {
  const trace = [];
  const record = (layer, text, extra = {}) => {
    const { errors, codes } = diagnose(text);
    trace.push({ layer, errors: errors.length, codes, ...extra });
    return text;
  };

  let text = record("raw", raw);

  const clean = sanitize(text);
  text = record("sanitize", clean.text, clean.notes);

  text = record("format", format(text));
  text = record("fix", autofix(text));

  const before = diagnose(text);
  let modelRounds = 0;
  /** One entry per independent attempt, so a single lucky round is not reported as a rate. */
  const attempts = [];

  if (before.errors.length > 0 && askModel) {
    for (let t = 0; t < trials; t++) {
      const answer = await askModel(repairPrompt(text, before.all));
      modelRounds += 1;
      if (!answer) {
        attempts.push({ errors: null, accepted: false, note: "no answer" });
        continue;
      }
      // The model's answer goes through the same free layers: it is a generation like any
      // other, and it will wrap things in fences too.
      let repaired = sanitize(answer).text;
      repaired = autofix(format(repaired));
      const after = diagnose(repaired);
      attempts.push({ errors: after.errors.length, codes: after.codes, accepted: false });

      // Adopt the first attempt that improves on the free layers; never accept one that
      // makes things worse, which happens often enough to matter.
      if (after.errors.length < before.errors.length) {
        attempts[attempts.length - 1].accepted = true;
        text = record("model", repaired, { accepted: true, trial: t + 1 });
        break;
      }
    }
    if (!attempts.some((a) => a.accepted)) {
      record("model", text, { accepted: false, attempts: attempts.length });
    }
  }

  return { text, trace, modelRounds, attempts, errors: diagnose(text).errors };
}

/** Which codes disappeared between two layers of the trace. */
export function fixedBetween(trace, from, to) {
  const a = trace.find((t) => t.layer === from);
  const b = trace.find((t) => t.layer === to);
  if (!a || !b) return [];
  return a.codes.filter((c) => !b.codes.includes(c));
}
