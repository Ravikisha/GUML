#!/usr/bin/env node
/**
 * Phase 0 runner.
 *
 * 10 tasks × 3 models × ({0, 3} examples for `guml`, one condition for `react`) = **90 generations** at
 * one repeat, which is the number `ROADMAP.md` commits to. This comment said 120 for a while: it multiplied
 * the two example-counts across both arms, and the React arm sees no spec and no examples, so it has one
 * condition rather than two. A planning number nobody checks is how a sweep's cost gets misquoted.
 *
 * Every run is written to its own file under results/raw and a **successful** run is skipped on a re-run, so
 * the sweep is resumable and a crash costs one generation rather than the batch.
 *
 * Thinking is off deliberately. Phase 0 measures the tokens of the *artifact*; if
 * extended thinking is enabled, thinking tokens land in the same output counter and
 * the headline number stops meaning what it says. If a later phase wants thinking,
 * it is a separate arm with its own column, not a silent change here.
 *
 *   node run.mjs --dry-run                 # assemble prompts, no API calls, no key
 *   node run.mjs                           # full sweep
 *   node run.mjs --tasks t01-crud --models sonnet --examples 3
 *   node run.mjs --repeats 3               # variance across identical prompts
 */
import { mkdirSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { BENCH, MODELS, buildPrompt, runId } from "./lib/prompt.mjs";
import { TASKS } from "./tasks/index.mjs";

const MAX_TOKENS = 8000;
const TEMPERATURE = 0; // reproducibility beats variety here; --repeats measures variance

function parseArgs(argv) {
  const args = { dryRun: false, repeats: 1, fullRegistry: false, out: join(BENCH, "results", "raw") };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--dry-run") args.dryRun = true;
    else if (a === "--full-registry") args.fullRegistry = true;
    else if (a === "--tasks") args.tasks = next().split(",");
    else if (a === "--models") args.models = next().split(",");
    else if (a === "--examples") args.examples = next().split(",").map(Number);
    else if (a === "--arms") args.arms = next().split(",");
    else if (a === "--repeats") args.repeats = Number(next());
    else if (a === "--out") args.out = next();
    else throw new Error(`unknown flag: ${a}`);
  }
  return args;
}

const args = parseArgs(process.argv.slice(2));
const tasks = args.tasks ? TASKS.filter((t) => args.tasks.includes(t.id)) : TASKS;
const models = args.models ?? Object.keys(MODELS);
const exampleCounts = args.examples ?? [0, 3];
const arms = args.arms ?? ["guml", "react"];

if (tasks.length === 0) throw new Error("no tasks selected");
for (const m of models) if (!MODELS[m]) throw new Error(`unknown model alias: ${m}`);

mkdirSync(args.out, { recursive: true });

/** The React arm sees no spec and no examples, so it has one condition, not two. */
function conditions() {
  const out = [];
  for (const task of tasks) {
    for (const arm of arms) {
      const counts = arm === "react" ? [0] : exampleCounts;
      for (const examples of counts) {
        for (const model of models) {
          for (let repeat = 1; repeat <= args.repeats; repeat++) {
            out.push({ task, arm, model, examples, repeat });
          }
        }
      }
    }
  }
  return out;
}

const plan = conditions();
console.log(`${plan.length} runs planned (${tasks.length} tasks, ${models.length} models, arms: ${arms.join("+")})`);

if (args.dryRun) {
  const dir = join(BENCH, "results", "prompts");
  mkdirSync(dir, { recursive: true });
  const seen = new Set();
  let inputChars = 0;
  for (const c of plan) {
    const key = `${c.task.id}__${c.arm}__ex${c.examples}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const p = buildPrompt({
      task: c.task,
      arm: c.arm,
      examples: c.examples,
      fullRegistry: args.fullRegistry,
    });
    const text = `=== SYSTEM ===\n${p.system.map((s) => s.text).join("\n")}\n\n=== USER ===\n${p.user}\n`;
    inputChars += text.length;
    writeFileSync(join(dir, `${key}.txt`), text);
  }
  console.log(`wrote ${seen.size} distinct prompts to results/prompts`);
  console.log(`~${Math.round(inputChars / 1000)}k chars of prompt across them`);
  console.log("dry run: no API calls made, no generations produced");
  process.exit(0);
}

if (!process.env.ANTHROPIC_API_KEY) {
  console.error("ANTHROPIC_API_KEY is not set. Use --dry-run to assemble prompts without it.");
  process.exit(1);
}

const { default: Anthropic } = await import("@anthropic-ai/sdk");
const client = new Anthropic();

let done = 0;
let skipped = 0;
for (const c of plan) {
  const id = runId({ task: c.task.id, arm: c.arm, model: c.model, examples: c.examples, repeat: c.repeat });
  const file = join(args.out, `${id}.json`);
  // Resume past *successful* runs only.
  //
  // This was `existsSync(file)` alone, which is wrong in the direction that costs the most: a failed run
  // writes its error to the same path, so re-running the sweep skipped it forever. A rate limit, a dropped
  // connection or an expired card would be baked into the results as a permanent hole — and the scoring step
  // would then report a smaller `n` than the sweep was supposed to produce, with nothing saying why.
  //
  // Found by the first live call: the key authenticated and the account had no credit, so every run would
  // have written an error file and a later retry with credit would have skipped all 90 of them.
  if (existsSync(file)) {
    let previous = null;
    try {
      previous = JSON.parse(readFileSync(file, "utf8"));
    } catch {
      // An unreadable or truncated file is not a result. Re-run it.
    }
    if (previous && !previous.error) {
      skipped++;
      continue;
    }
    console.log(`  retrying ${id} (previous attempt failed)`);
  }

  const prompt = buildPrompt({
    task: c.task,
    arm: c.arm,
    examples: c.examples,
    fullRegistry: args.fullRegistry,
  });
  const model = MODELS[c.model];
  const messages = [{ role: "user", content: prompt.user }];

  // Counted before the call so the prompt tax is recorded even if generation fails.
  let inputEstimate = null;
  try {
    inputEstimate = await client.messages.countTokens({ model, system: prompt.system, messages });
  } catch (e) {
    console.warn(`  count_tokens failed for ${id}: ${e.message}`);
  }

  const started = Date.now();
  let result;
  try {
    // Streaming: an 8k-token React generation can exceed the non-streaming timeout.
    const stream = client.messages.stream({
      model,
      max_tokens: MAX_TOKENS,
      temperature: TEMPERATURE,
      system: prompt.system,
      messages,
    });
    result = await stream.finalMessage();
  } catch (e) {
    writeFileSync(
      file,
      JSON.stringify({ id, ...describe(c), error: String(e), latencyMs: Date.now() - started }, null, 2),
    );
    console.error(`✗ ${id}: ${e.message}`);
    // Some failures will not clear by trying the next condition, and grinding through 89 more of them wastes
    // wall-clock and fills `results/raw` with files a human then has to delete. Credit and authentication are
    // both account-level: stop and say so.
    if (/credit balance|authentication_error|invalid x-api-key|permission/i.test(e.message)) {
      console.error(
        "\nthis is an account-level failure, not a transient one — the remaining runs would all fail the " +
          "same way.\nfix the account and re-run: completed generations are skipped, failed ones are retried.",
      );
      process.exit(1);
    }
    continue;
  }
  const latencyMs = Date.now() - started;

  const text = result.content
    .filter((b) => b.type === "text")
    .map((b) => b.text)
    .join("");

  writeFileSync(
    file,
    JSON.stringify(
      {
        id,
        ...describe(c),
        settings: { model, maxTokens: MAX_TOKENS, temperature: TEMPERATURE, thinking: "off" },
        usage: result.usage,
        inputTokensCounted: inputEstimate?.input_tokens ?? null,
        stopReason: result.stop_reason,
        latencyMs,
        output: text,
      },
      null,
      2,
    ),
  );

  done++;
  console.log(
    `✓ ${id}  out=${result.usage.output_tokens} in=${result.usage.input_tokens} ` +
      `cacheRead=${result.usage.cache_read_input_tokens ?? 0} ${latencyMs}ms`,
  );
}

function describe(c) {
  return {
    task: c.task.id,
    category: c.task.category,
    arm: c.arm,
    modelAlias: c.model,
    examples: c.examples,
    repeat: c.repeat,
    fullRegistry: args.fullRegistry,
  };
}

console.log(`\n${done} runs completed, ${skipped} skipped (already present)`);
console.log("next: node score.mjs");
