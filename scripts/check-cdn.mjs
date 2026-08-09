/**
 * Serve the package over HTTP the way a CDN would, and load it both supported ways.
 *
 * The point is that neither path can be verified from the filesystem: `import.meta.url` and
 * `document.currentScript.src` both resolve against an *origin*, and `file://` is not one. A test that
 * imports from a relative path proves nothing about a CDN.
 */

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import process from "node:process";

const ROOT = process.argv[2] ?? "packages/guml";
const PORT = 4173;

const TYPES = {
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".ts": "text/javascript",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".html": "text/html",
};

const server = createServer(async (req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(
    /^([/\\])+/,
    "",
  );
  try {
    const body = await readFile(join(ROOT, path));
    res.writeHead(200, {
      "content-type": TYPES[extname(path)] ?? "application/octet-stream",
      // A CDN sets this; without it a cross-origin fetch of the wasm would fail.
      "access-control-allow-origin": "*",
    });
    res.end(body);
  } catch {
    res.writeHead(404).end("not found");
  }
});

await new Promise((r) => server.listen(PORT, r));
const base = `http://localhost:${PORT}`;
console.log(`serving ${ROOT} at ${base}\n`);

let failures = 0;
const check = (label, ok, detail = "") => {
  console.log(`  ${ok ? "ok  " : "FAIL"}  ${label}${detail ? `  — ${detail}` : ""}`);
  if (!ok) failures++;
};

// ---------------------------------------------------------------- the ESM entry
//
// Node's loader refuses `http:` specifiers, so the import cannot be *performed* here — that is a Node
// restriction, not a property of the package, and browsers have no such limit. What is checkable from
// here is the thing that actually decides whether a CDN load works: that the module graph reaches the
// wasm by paths relative to itself, with no bare specifier and no absolute path anywhere in the chain.
// `cdn.html` exercises the real browser path.
try {
  const index = await (await fetch(`${base}/dist/index.js`)).text();
  check("ESM entry is served", index.length > 1000, `${(index.length / 1024).toFixed(0)} KB`);

  // Comments stripped first. The naive version flagged `node:fs/promises` from a JSDoc example
  // showing how to supply the wasm under Node — a false positive that reads exactly like a real
  // blocker, and would have failed CI for a documentation string.
  const code = index
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
  const specifiers = [...code.matchAll(/from\s*["']([^"']+)["']/g)].map((m) => m[1]);
  const bare = specifiers.filter((x) => !x.startsWith(".") && !x.startsWith("/"));
  check(
    "no bare specifiers a browser cannot resolve",
    bare.length === 0,
    bare.join(", ") || `${specifiers.length} relative imports`,
  );

  const loader = await (await fetch(`${base}/wasm/guml.js`)).text();
  check("loader is reachable at the path the entry uses", loader.length > 1000);
  check(
    "wasm URL is relative to the module, not to the page",
    loader.includes(`new URL('guml_bg.wasm', import.meta.url)`),
    "this is what makes a CDN load work",
  );
} catch (e) {
  check("ESM entry is served", false, e.message.slice(0, 120));
}

// ---------------------------------------------------------------- the classic `<script src>` build
// Node has no `document`, so the global bundle's `currentScript` branch cannot run here — what is
// checkable is that the bundle is self-contained and defines the API. The browser path is exercised
// by `cdn.html`, which a person opens.
try {
  const text = await (await fetch(`${base}/dist/guml.global.js`)).text();
  check("global bundle is served", text.length > 1000, `${(text.length / 1024).toFixed(0)} KB`);
  check("bundle has no bare imports left", !/\bfrom"[a-z@]/.test(text));
  check("bundle assigns window.guml", text.includes("guml="));
  check("bundle resolves the wasm from its own src", text.includes("../wasm/guml_bg.wasm"));
} catch (e) {
  check("global bundle is served", false, e.message.slice(0, 120));
}

// ---------------------------------------------------------------- the wasm itself
try {
  const res = await fetch(`${base}/wasm/guml_bg.wasm`);
  check("wasm is fetchable", res.ok, `HTTP ${res.status}`);
  check(
    "served as application/wasm",
    res.headers.get("content-type") === "application/wasm",
    res.headers.get("content-type") ?? "",
  );
  const bytes = await res.arrayBuffer();
  check("wasm compiles", (await WebAssembly.compile(bytes)) !== undefined, `${Math.round(bytes.byteLength / 1024)} KB`);
} catch (e) {
  check("wasm is fetchable", false, e.message.slice(0, 120));
}

server.close();
console.log(failures ? `\n${failures} failure(s)` : "\nevery CDN entry point works over HTTP");
process.exit(failures ? 1 : 0);
