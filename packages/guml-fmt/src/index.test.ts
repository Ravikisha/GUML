/**
 * The claim this package makes beyond `@guml/core` is that it **runs in Node**. These tests are that
 * claim, so the fact that `node --test` can execute them at all is half of what they verify.
 *
 * `@guml/core` cannot do this: its wasm is built for the web target and loads itself with `fetch`,
 * which has no `file://` support in Node. The failure is an undici stack trace with no mention of
 * WebAssembly, which reads as a broken install. If `init()` ever regresses to a plain `initWasm()`,
 * every test below fails immediately rather than in someone's pre-commit hook.
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import { canonical, format, highlight, isFormatted, version } from "./index.ts";

const MESSY = `page   "Counter"


state n:   0
col
     h1   "Count: {n}"
     btn "Add"  > set n = n + 1
`;

test("formats a document", async () => {
  const out = await format(MESSY);
  assert.ok(out.includes("page"), out);
  assert.ok(!out.includes("   \n"), "trailing whitespace should be gone");
  assert.notEqual(out, MESSY, "this input is deliberately not already formatted");
});

test("is idempotent", async () => {
  const once = await format(MESSY);
  const twice = await format(once);
  assert.equal(twice, once, "format(format(x)) must equal format(x)");
});

test("isFormatted agrees with format", async () => {
  assert.equal(await isFormatted(MESSY), false);
  assert.equal(await isFormatted(await format(MESSY)), true);
});

test("canonical strips what format keeps", async () => {
  // `//`, not `#`. GUML's comment syntax is the C-style one, and getting that wrong is how this test
  // first "failed": `# a comment` is not a comment, so canonical had no reason to remove it.
  const withComment = `// a comment\npage "X"\n\ncol\n  p Hello\n`;
  const formatted = await format(withComment);
  const canon = await canonical(withComment);
  assert.ok(formatted.includes("// a comment"), "format preserves commentary");
  assert.ok(!canon.includes("// a comment"), "canonical removes it on purpose");
});

test("canonical makes equivalent documents byte-identical", async () => {
  // Same document, different incidental formatting. That is the property the benchmark relies on.
  const a = `page "X"\ncol\n  p Hello\n`;
  const b = `// notes\npage   "X"\n\n\ncol\n     p Hello\n`;
  assert.equal(await canonical(a), await canonical(b));
});

test("highlight returns the compiler's own class names", async () => {
  const spans = await highlight(`page "Counter"\n`);
  assert.ok(spans.length > 0, "expected spans");
  const first = spans[0]!;
  assert.equal(typeof first.line, "number");
  assert.equal(typeof first.start, "number");
  assert.equal(typeof first.end, "number");
  assert.ok(
    spans.some((s) => s.class === "directive"),
    `\`page\` is a directive; got ${JSON.stringify(spans.map((s) => s.class))}`,
  );
});

test("reports a version", async () => {
  assert.match(await version(), /^\d+\.\d+\.\d+$/);
});

test("empty input does not throw", async () => {
  assert.equal(typeof (await format("")), "string");
  assert.deepEqual(await highlight(""), []);
});
