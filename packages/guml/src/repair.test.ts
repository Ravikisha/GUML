/**
 * Tests for the free repair pipeline, through the published surface.
 *
 * These exercise the real wasm compiler rather than a re-implementation, so what is being checked is
 * that the package *exposes* the layers — not that the layers work, which the Rust suite already pins
 * in `crates/guml-compiler/src/repair.rs`.
 *
 * That distinction is the reason this file exists. `sanitize` and the format layer lived only in
 * `bench/gen/lib/pipeline.mjs`, so the numbers the project quotes about repair came from a pipeline the
 * npm package could not run. Anyone calling `check()` on raw model output got a parse error on line 1
 * for a ``` fence the benchmark had already discounted.
 *
 * Run with `node --test` (Node strips the types natively).
 */

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { before, describe, it } from "node:test";
// `./index.ts`, not `./index.js`: Node runs this file directly with type stripping and does not
// rewrite specifiers. `tsconfig.build.json` sets `rewriteRelativeImportExtensions`, so the published
// ESM still says `.js`.
import { check, init, repair } from "./index.ts";

// The wasm has to be handed over as bytes here. The build targets `web`, so with no argument the glue
// `fetch`es the `.wasm` beside itself — and `fetch` on a `file:` URL is not implemented in Node, which
// is why `init` accepts a `BufferSource`.
before(async () => {
  await init(await readFile(new URL("../wasm/guml_bg.wasm", import.meta.url)));
});

/** What a model actually returns when it ignores "emit GUML only, no fence, no prose". */
const RAW = `Here is the page you asked for:

\`\`\`guml
page P
div
  span Hello
  button Save primry
\`\`\`

This page shows a greeting and a save button.
`;

describe("repair", () => {
  it("repairs a fenced, prose-wrapped, HTML-shaped generation with no model call", async () => {
    const before = await check(RAW);
    assert.ok(before.errorCount > 0, "the raw generation should not be valid");

    const out = await repair(RAW);
    assert.equal(out.ok, true, `still broken: ${JSON.stringify(out.report)}`);
    assert.equal(out.errorsAfter, 0);
    assert.ok(out.errorsBefore > 0);

    // The fence was packaging, not document.
    assert.equal(out.sanitize.fence, true);
    // HTML habits are renames the compiler knows: `div`→`col`, `span`→`text`, `button`→`btn`. Edit
    // distance reaches none of them, which is why the habit table exists.
    assert.match(out.text, /^page P/);
    assert.match(out.text, /\bcol\b/);
    assert.match(out.text, /text Hello/);
    assert.match(out.text, /btn Save primary/);
    // And the trailing sentence is gone.
    assert.doesNotMatch(out.text, /This page shows/);
  });

  it("names the layer that did the work", async () => {
    // "The repair loop works" is not a claim worth making; which layer handled what is.
    const out = await repair(RAW);
    assert.ok(out.report.length > 0, "no layer reported doing anything");
    assert.ok(
      out.report.some((line) => line.startsWith("sanitize:")),
      `expected a sanitize line: ${JSON.stringify(out.report)}`,
    );
    assert.ok(
      out.applied.length > 0,
      `expected applied diagnostic codes: ${JSON.stringify(out.applied)}`,
    );
  });

  it("leaves a valid document untouched", async () => {
    // It runs on every document, so being a no-op on a correct one is the property that matters most.
    const src = "page P\nstate count=0\n\ncard sm center\n  h Clicks\n  metric {count}\n";
    const out = await repair(src);
    assert.equal(out.text, src);
    assert.equal(out.changed, false);
    assert.equal(out.ok, true);
    assert.deepEqual(out.report, []);
  });

  it("is idempotent", async () => {
    // A pipeline that repairs on save must not rewrite the same file forever.
    const once = await repair(RAW);
    const twice = await repair(once.text);
    assert.equal(twice.text, once.text);
    assert.equal(twice.changed, false);
  });

  it("never makes a document worse", async () => {
    // The rule the measured model-round layer already uses, applied to the free layers too: an attempt
    // is kept only if it does not raise the error count.
    for (const src of ["This is prose, not a document.\n", "", "page\n", "```\n```\n"]) {
      const out = await repair(src);
      assert.ok(
        out.errorsAfter <= out.errorsBefore,
        `repair made it worse for ${JSON.stringify(src)}: ${out.errorsBefore} -> ${out.errorsAfter}`,
      );
    }
  });

  it("does not delete repairable GUML as if it were commentary", async () => {
    // The bug this guards: "drop the last line that has an error" ate a document one line per round,
    // reducing four fixable HTML habits to `page P` and reporting success.
    const out = await repair("page P\ndiv\n  span Hello\n  button Save\n  hr\n");
    assert.equal(out.sanitize.trailing, 0, "repairable GUML was dropped as commentary");
    assert.equal(out.ok, true);
    for (const want of ["col", "text Hello", "btn Save", "divider"]) {
      assert.ok(out.text.includes(want), `\`${want}\` missing from:\n${out.text}`);
    }
  });
});
