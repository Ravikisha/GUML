/**
 * The `<script src>` entry point: `window.guml`, no bundler and no module syntax.
 *
 * # Why this exists when the ESM build already works from a CDN
 *
 * `import { compile } from "https://esm.sh/@guml/core"` works today, because the generated loader
 * resolves the wasm with `new URL("guml_bg.wasm", import.meta.url)` — which on a CDN is the CDN. That
 * covers anyone writing `<script type="module">`.
 *
 * It does not cover a plain `<script src>`: a classic script has no `import.meta`, and cannot use
 * `import` at all. That is still how a lot of pages are built — a CMS template, a documentation page,
 * a CodePen, an existing site with no build step. Telling those users "add a bundler" for a compiler
 * whose whole pitch is *no build step* would be a poor answer.
 *
 * # How the wasm is found
 *
 * A classic script cannot ask where it was loaded from through `import.meta.url`, but it can ask
 * `document.currentScript.src` — which is set for exactly this case, during execution, before any
 * `await`. So the URL is captured *synchronously at module scope* and the fetch started immediately.
 *
 * Reading it later would return `null`: `currentScript` is only valid while the script is executing,
 * and by the time a caller invokes `compile()` the browser has moved on. That is the whole subtlety
 * here, and it is why this file does work at import time rather than lazily.
 *
 * Kicking off the fetch eagerly is a bonus rather than the point: the 787 KB download overlaps with
 * whatever else the page is doing instead of starting when the user first clicks something.
 */

import * as api from "./index.ts";

declare global {
  interface Window {
    guml: typeof api;
  }
}

/** Where this script was loaded from, captured while `document.currentScript` is still meaningful. */
function ownUrl(): string | undefined {
  if (typeof document === "undefined") return undefined;
  const current = document.currentScript as HTMLScriptElement | null;
  return current?.src || undefined;
}

const src = ownUrl();
if (src) {
  // `dist/guml.global.js` sits beside `dist/`, and the wasm is one level up in `wasm/`. Both are in
  // the published package, so this holds on every CDN that serves a package verbatim — jsDelivr,
  // unpkg, esm.sh — and for a self-hosted copy.
  //
  // Failure here is swallowed: an eager warm-up that rejects must not become an unhandled rejection
  // on a page that has not called anything yet. A real failure resurfaces on the first actual call,
  // where the caller has a `try`/`catch` and some context for the error.
  void api.init(new URL("../wasm/guml_bg.wasm", src)).catch(() => {});
}

if (typeof window !== "undefined") {
  window.guml = api;
}

export default api;
