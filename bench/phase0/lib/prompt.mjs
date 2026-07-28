import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
export const ROOT = join(HERE, "..", "..", ".."); // repo root
export const BENCH = join(HERE, "..");

/** The three in-context examples. Deliberately none of them is a task's answer. */
export const EXAMPLE_ORDER = ["e1-counter.guml", "e2-signin.guml", "e3-invoices.guml"];

export function readSpec() {
  return readFileSync(join(ROOT, "spec", "GUML-SPEC.md"), "utf8");
}

export function readExamples(n) {
  if (n === 0) return [];
  const files = EXAMPLE_ORDER.slice(0, n);
  const present = new Set(readdirSync(join(BENCH, "examples")));
  for (const f of files) {
    if (!present.has(f)) throw new Error(`missing example: ${f}`);
  }
  return files.map((f) => ({ name: f, source: readFileSync(join(BENCH, "examples", f), "utf8") }));
}

/**
 * A registry slice, from the compiler itself rather than a copy that can drift.
 *
 * The slice is a variable, not a constant: a task-specific slice that happens to
 * contain exactly the tags the answer needs is a hint the full registry does not
 * give. Both are runnable, and `score.mjs` reports them separately.
 */
export function registrySlice(tags, { full = false } = {}) {
  const args = ["run", "-q", "-p", "guml-cli", "--", "registry"];
  if (!full && tags) args.push("--tags", tags);
  return execFileSync("cargo", args, { cwd: ROOT, encoding: "utf8" }).trim();
}

const GUML_RULES = `Output rules:
- Emit GUML only. No prose before or after, no markdown code fence, no explanation.
- Use only tags from the vocabulary above. An unknown tag is a compile error.
- Indent with two spaces. Never a tab.
- \`>\` must be the last thing on a line: it consumes the rest of it.
- Do not hand-write loading, empty, error, optimistic-update or rollback logic. The
  compiler generates those from \`data\`. Writing them yourself is wrong, not thorough.
- \`//\` starts a comment line. Comments are free: the lexer drops them.
- If the task needs something the vocabulary cannot express, emit the rest of the
  document and add a final line \`// UNSUPPORTED: <what you could not express>\`.`;

const REACT_RULES = `Output rules:
- Emit one React function component in TypeScript with Tailwind classes.
- No prose before or after, no markdown code fence, no explanation.
- A single self-contained file with a default export. Assume React 19.
- No component library imports: Tailwind utility classes only.`;

/**
 * Cache-friendly layout: everything stable (spec, registry, examples) precedes
 * everything task-specific, so the whole prefix is one cache hit across the ten
 * tasks of a run. The prompt tax is only defensible if it is actually cached, and
 * ordering is what decides that.
 */
/**
 * The stable prefix: spec, registry slice, examples, output rules. Exported on its own
 * because the docs chatbot uses the *same* prompt this study measures — if the product and
 * the experiment prompt differently, the experiment stops predicting the product.
 */
export function systemPrompt({ tags = null, examples = 0, fullRegistry = false } = {}) {
  const parts = [readSpec().trim()];
  parts.push(`## Available tags\n\n${registrySlice(tags, { full: fullRegistry || !tags })}`);

  const ex = readExamples(examples);
  if (ex.length > 0) {
    const blocks = ex
      .map((e) => `### ${e.name.replace(/\.guml$/, "")}\n\n${e.source.trim()}`)
      .join("\n\n");
    parts.push(`## Examples of valid GUML\n\n${blocks}`);
  }
  parts.push(GUML_RULES);
  return parts.join("\n\n---\n\n");
}

export function buildPrompt({ task, arm, examples = 0, fullRegistry = false }) {
  if (arm === "react") {
    return {
      system: [
        {
          type: "text",
          text: `You write production React for a senior team. Accessibility, loading, error and empty states are part of the job, not extras.\n\n${REACT_RULES}`,
        },
      ],
      user: task.prompt,
      // Nothing stable to cache in this arm beyond the system prompt; that
      // asymmetry is real and is reported, not hidden.
      cachedSegments: 0,
    };
  }

  const parts = [];
  parts.push(readSpec().trim());
  parts.push(`## Available tags for this task\n\n${registrySlice(task.tags, { full: fullRegistry })}`);

  const ex = readExamples(examples);
  if (ex.length > 0) {
    parts.push(
      `## Examples of valid GUML\n\n${ex
        .map((e) => `### ${e.name.replace(/\.guml$/, "")}\n\n${e.source.trim()}`)
        .join("\n\n")}`,
    );
  }
  parts.push(GUML_RULES);

  const prefix = parts.join("\n\n---\n\n");

  return {
    system: [
      {
        type: "text",
        text: `You write GUML, a compact declarative UI representation. The specification follows.\n\n${prefix}`,
        // The cache breakpoint sits at the end of the stable prefix.
        cache_control: { type: "ephemeral" },
      },
    ],
    user: task.prompt,
    cachedSegments: 1,
  };
}

/** Models under test. Capability is a first-class variable, not a footnote. */
export const MODELS = {
  haiku: "claude-haiku-4-5",
  sonnet: "claude-sonnet-5",
  opus: "claude-opus-5",
};

export function runId({ task, arm, model, examples, repeat }) {
  return `${task}__${arm}__${model}__ex${examples}__r${repeat}`;
}
