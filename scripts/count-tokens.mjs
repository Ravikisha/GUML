#!/usr/bin/env node
/**
 * Count tokens the way a claim requires, not the way that is convenient.
 *
 * Three counters, and the difference between them is the whole point:
 *
 *  - **`count_tokens`** (Anthropic API) — the only figure that may go in a README or a paper
 *    when the target model is Claude. Needs `ANTHROPIC_API_KEY`.
 *  - **`cl100k_base` / `o200k_base`** (tiktoken) — OpenAI tokenizers. They undercount Claude by
 *    roughly 15–20% on prose and more on code. Reported here for continuity with the figures
 *    already published, and labelled every time.
 *  - **`guml tokens`** — a ~3.6 chars/token heuristic for the dev loop. Never a claim.
 *
 * The reason this script exists rather than a note in a doc: the published figures were wrong
 * by 7 tokens for months because nobody could re-run the measurement in one command.
 *
 *   node scripts/count-tokens.mjs                       # every fixture, every counter
 *   node scripts/count-tokens.mjs fixtures/b.guml       # one file
 *   node scripts/count-tokens.mjs --json                # machine-readable
 */
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const JSON_OUT = process.argv.includes("--json");
const args = process.argv.slice(2).filter((a) => !a.startsWith("--"));

/** The paired artifacts behind the published ratios. */
function defaultFiles() {
  const dir = join(ROOT, "fixtures");
  return readdirSync(dir)
    .filter((f) => f.endsWith(".guml") || f.endsWith(".react.tsx") || f.endsWith(".spec.json"))
    .sort()
    .map((f) => join(dir, f));
}

const files = args.length > 0 ? args.map((a) => join(ROOT, a)) : defaultFiles();

/* ------------------------------------------------------------------ counters */

/**
 * The authoritative count for a Claude target.
 *
 * Uses the Anthropic SDK's `countTokens`, which is the same tokenizer the billing path uses —
 * unlike every local approximation, which is a different tokenizer wearing a similar name.
 */
async function anthropicCounts(texts, model = "claude-opus-5") {
  if (!process.env.ANTHROPIC_API_KEY) return null;
  let Anthropic;
  try {
    ({ default: Anthropic } = await import("@anthropic-ai/sdk"));
  } catch {
    console.error("! @anthropic-ai/sdk is not installed; skipping the authoritative count");
    return null;
  }

  const client = new Anthropic();
  const out = [];
  for (const text of texts) {
    // One request per artifact: `count_tokens` has no batch form, and the artifacts are small.
    const res = await client.messages.countTokens({
      model,
      messages: [{ role: "user", content: text }],
    });
    // The count includes a small fixed overhead for the message envelope. Measuring the
    // envelope once and subtracting it is what makes these comparable to a raw file count.
    out.push(res.input_tokens);
  }
  return { model, counts: out };
}

/** The envelope cost, so artifact counts can be reported without it. */
async function anthropicOverhead(model) {
  if (!process.env.ANTHROPIC_API_KEY) return 0;
  const { default: Anthropic } = await import("@anthropic-ai/sdk");
  const client = new Anthropic();
  const empty = await client.messages.countTokens({
    model,
    messages: [{ role: "user", content: "" }],
  });
  return empty.input_tokens;
}

async function tiktokenCounts(texts) {
  // Python, because the JS ports of tiktoken need a network fetch for the BPE ranks.
  const { execFileSync } = await import("node:child_process");
  const payload = JSON.stringify(texts);
  // stdin and stdout are reconfigured to UTF-8 explicitly: on Windows, Python reads stdin in
  // the console codepage, which mangled the `…` in a fixture and silently changed the count
  // by seven tokens. That is the exact failure this script was written to stop.
  const script = `
import json, sys
sys.stdin.reconfigure(encoding="utf-8")
sys.stdout.reconfigure(encoding="utf-8")
try:
    import tiktoken
except ImportError:
    print("null"); sys.exit(0)
texts = json.loads(sys.stdin.read())
out = {}
for name in ("cl100k_base", "o200k_base"):
    enc = tiktoken.get_encoding(name)
    out[name] = [len(enc.encode(t)) for t in texts]
print(json.dumps(out))
`;
  try {
    const raw = execFileSync("python", ["-c", script], { input: payload, encoding: "utf8" });
    return JSON.parse(raw.trim());
  } catch {
    return null;
  }
}

const heuristic = (text) => Math.ceil(text.length / 3.6);

/* -------------------------------------------------------------------- report */

const texts = files.map((f) => readFileSync(f, "utf8"));
const names = files.map((f) => relative(ROOT, f).replace(/\\/g, "/"));

const tik = await tiktokenCounts(texts);
const model = process.env.ANTHROPIC_MODEL || "claude-opus-5";
const anthropic = await anthropicCounts(texts, model);
const overhead = anthropic ? await anthropicOverhead(model) : 0;

const rows = names.map((name, i) => ({
  file: name,
  bytes: texts[i].length,
  heuristic: heuristic(texts[i]),
  cl100k: tik?.cl100k_base?.[i] ?? null,
  o200k: tik?.o200k_base?.[i] ?? null,
  [model]: anthropic ? anthropic.counts[i] - overhead : null,
}));

if (JSON_OUT) {
  console.log(JSON.stringify({ model, envelopeOverhead: overhead, rows }, null, 2));
} else {
  const cols = ["file", "bytes", "heuristic", "cl100k", "o200k", model];
  const width = (c) => Math.max(c.length, ...rows.map((r) => String(r[c] ?? "—").length));
  const widths = Object.fromEntries(cols.map((c) => [c, width(c)]));
  const line = (cells) => cols.map((c, i) => String(cells[i]).padEnd(widths[c])).join("  ");

  console.log(line(cols));
  console.log(cols.map((c) => "-".repeat(widths[c])).join("  "));
  for (const r of rows) console.log(line(cols.map((c) => r[c] ?? "—")));

  console.log("");
  if (!anthropic) {
    console.log(
      "No ANTHROPIC_API_KEY, so the authoritative column is empty. cl100k and o200k are OpenAI\n" +
        "tokenizers and undercount Claude — usable for continuity with the published figures,\n" +
        "not for a new claim. `heuristic` is ~3.6 chars/token and is never a claim.",
    );
  } else {
    console.log(
      `${model} counts exclude the ${overhead}-token message envelope, so they are comparable\n` +
        "to a raw artifact count. This is the column a README or a paper may quote.",
    );
  }
}
