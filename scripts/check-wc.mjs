#!/usr/bin/env node
/**
 * Check the Web Components backend's output by **running it**.
 *
 * # Why not a DOM library
 *
 * jsdom or happy-dom would be the obvious choice and would mean adding a dependency to test one
 * backend. It turns out not to be necessary: the generated code touches a small, known surface —
 * `HTMLElement`, `customElements.define`, `querySelector`, `innerHTML`, `addEventListener` — and every
 * DOM write is guarded by `if (el)`. A shim of that surface is enough to execute `connectedCallback`,
 * `#update` and every action body for real.
 *
 * What that catches, which a syntax check alone would not:
 *
 * * A binding lowered with the *wrong* dialect. The first version of this backend emitted
 *   `String({s.count} ?? "")` — JSX interpolation syntax in a plain-JavaScript file. It is a syntax
 *   error, so the whole module fails to parse, and the only reason it shipped for ten minutes is that
 *   nothing executed it.
 * * A state read that resolves to nothing. `draft.trim()` without the `this.#state.` prefix is a
 *   `ReferenceError` at update time, not at parse time.
 * * An action that does not change what it claims to. Running the dispatcher and asserting on state is
 *   the only way to know `>count++` increments.
 *
 * What it deliberately does not check: layout, and anything that needs real HTML parsing. `innerHTML`
 * here is a string. The structural assertions below cover what can be checked without a parser, and the
 * honest limit is stated rather than papered over.
 *
 * Run from the repository root:
 *
 *   node scripts/check-wc.mjs
 */
import { execFileSync } from "node:child_process";
import { readdirSync, rmSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DOCS = join(ROOT, "docs");
const OUT = join(DOCS, ".guml-wc");

const { default: ts } = await import(
  pathToFileURL(join(DOCS, "node_modules", "typescript", "lib", "typescript.js")).href
);

/* ------------------------------------------------------------------ the shim */

/**
 * Just enough DOM for the generated code to run.
 *
 * `querySelector` returns a fresh recording stub rather than `null`, so `#update`'s writes are actually
 * executed and captured — returning `null` would make every `if (el)` guard skip the body and the test
 * would pass on code that throws in a browser.
 */
function installDom() {
  const writes = [];
  class FakeEl {
    constructor(selector) {
      this.selector = selector;
      this.classList = { toggle: (c, on) => writes.push(["class", selector, c, on]) };
    }
    set textContent(v) {
      writes.push(["text", this.selector, v]);
    }
    set innerHTML(v) {
      writes.push(["html", this.selector, v]);
    }
    setAttribute(name, value) {
      writes.push(["attr", this.selector, name, value]);
    }
    closest() {
      return null;
    }
  }
  class HTMLElement {
    constructor() {
      this.listeners = {};
      this._html = "";
    }
    set innerHTML(v) {
      this._html = v;
    }
    get innerHTML() {
      return this._html;
    }
    addEventListener(type, fn) {
      (this.listeners[type] ??= []).push(fn);
    }
    querySelector(selector) {
      return new FakeEl(selector);
    }
  }
  const defined = new Map();
  globalThis.HTMLElement = HTMLElement;
  globalThis.customElements = { define: (name, cls) => defined.set(name, cls) };
  // A resource fetch must not reach the network. Rejecting is the honest default: it exercises the
  // generated error path, which is a branch worth executing.
  globalThis.fetch = () => Promise.reject(new Error("offline in test"));
  return { writes, defined };
}

/* ------------------------------------------------------------------ checks */

/** Structural assertions on the emitted source, for what cannot be executed. */
function structure(src, name) {
  const problems = [];

  // Every id `#update` reads must exist in the markup, or the update is a no-op nobody notices.
  const referenced = [...src.matchAll(/#el\((\d+)\)/g)].map((m) => m[1]);
  const present = new Set([...src.matchAll(/data-g-id="(\d+)"/g)].map((m) => m[1]));
  for (const id of referenced) {
    if (!present.has(id)) problems.push(`#el(${id}) has no element with data-g-id="${id}"`);
  }

  // Every action index on an element must have a case in the dispatcher, and vice versa.
  const onElements = new Set([...src.matchAll(/data-g-act="(\d+)"/g)].map((m) => m[1]));
  const inSwitch = new Set([...src.matchAll(/case "[a-z]+:(\d+)":/g)].map((m) => m[1]));
  for (const i of onElements) {
    if (!inSwitch.has(i)) problems.push(`data-g-act="${i}" has no case in #dispatch`);
  }
  for (const i of inSwitch) {
    if (!onElements.has(i)) problems.push(`#dispatch handles action ${i}, which no element triggers`);
  }

  // A bound field must carry the marker the dispatcher matches on, or typing changes nothing.
  const fieldWrites = [...src.matchAll(/el\.value = this\.#state\.(\w+)/g)].map((m) => m[1]);
  for (const field of fieldWrites) {
    if (!src.includes(`data-g-field="${field}"`)) {
      problems.push(`\`${field}\` is written at first paint but has no data-g-field marker`);
    }
  }

  // JSX interpolation leaking into a plain-JavaScript file. This is the bug that motivated the script.
  if (/String\(\{/.test(src)) problems.push("a JSX interpolation leaked into String(...)");
  if (/customElements\.define\("[a-z0-9]+"/.test(src)) {
    problems.push("custom element name has no hyphen, which the standard requires");
  }
  if (!src.includes("customElements.define")) problems.push("nothing is registered");
  if (name !== "d" && src.includes("TODO(guml)")) {
    problems.push("an unlowered construct was left in the output");
  }
  return problems;
}

/** Load the module and drive it. */
async function run(file, name) {
  const { defined } = installDom();
  const mod = await import(`${pathToFileURL(file).href}?t=${Date.now()}`);
  const problems = [];

  if (defined.size !== 1) {
    problems.push(`expected one custom element, got ${defined.size}`);
    return problems;
  }
  const [tag, Cls] = [...defined.entries()][0];
  if (!tag.includes("-")) problems.push(`custom element \`${tag}\` has no hyphen`);

  const el = new Cls();
  // The real thing: this runs the generated markup build, every first-paint field write, and `#update`
  // — which evaluates every binding expression in the document.
  el.connectedCallback();
  if (!el.innerHTML.trim()) problems.push("connectedCallback rendered nothing");

  // Idempotence. A custom element can be connected more than once (a move in the DOM does it), and
  // rebuilding the markup would discard user state. The `#painted` guard is what prevents that.
  const first = el.innerHTML;
  el.connectedCallback();
  if (el.innerHTML !== first) problems.push("a second connect rebuilt the markup");

  // Drive every action through the delegated dispatcher, exactly as a click would. The state object is
  // private, so success is "no throw" — a broken action body is a `ReferenceError` here.
  const indices = [...mod.default.toString().matchAll(/case "([a-z]+):(\d+)":/g)];
  for (const [, type, index] of indices) {
    const target = {
      dataset: { gAct: index },
      closest: (sel) => (sel === "[data-g-act]" ? target : null),
      value: "typed",
    };
    for (const fn of el.listeners[type] ?? []) {
      try {
        fn({ type, target, preventDefault() {} });
      } catch (err) {
        problems.push(`action ${type}:${index} threw: ${err.message}`);
      }
    }
  }
  return problems;
}

/* ------------------------------------------------------------------ main */

rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });
let failed = 0;
try {
  const fixtures = readdirSync(join(ROOT, "fixtures")).filter((f) => f.endsWith(".guml"));
  console.log(`checking ${fixtures.length} fixtures through the wc backend\n`);

  for (const fixture of fixtures) {
    const name = fixture.replace(/\.guml$/, "");
    const src = execFileSync(
      "cargo",
      ["run", "-q", "-p", "guml-cli", "--", "build", join("fixtures", fixture), "--backend", "wc"],
      { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    );

    // Syntax first: a parse error makes every other check meaningless.
    const parsed = ts.createSourceFile(`${name}.js`, src, ts.ScriptTarget.ES2022, true);
    const syntax = parsed.parseDiagnostics ?? [];
    if (syntax.length > 0) {
      const first = syntax[0];
      const { line } = parsed.getLineAndCharacterOfPosition(first.start);
      console.error(
        `✗ ${name.padEnd(12)} syntax error on line ${line + 1}: ${ts.flattenDiagnosticMessageText(first.messageText, " ")}`,
      );
      failed += 1;
      continue;
    }

    const problems = structure(src, name);
    const file = join(OUT, `${name}.mjs`);
    writeFileSync(file, src);
    problems.push(...(await run(file, name)));

    if (problems.length > 0) {
      console.error(`✗ ${name}`);
      for (const p of problems) console.error(`    ${p}`);
      failed += 1;
    } else {
      console.log(`✓ ${name.padEnd(12)} ${src.split("\n").length} lines, parses, runs, dispatches`);
    }
  }
} finally {
  rmSync(OUT, { recursive: true, force: true });
}

if (failed > 0) {
  console.error(`\n${failed} fixture(s) failed the wc backend check`);
  process.exit(1);
}
console.log("\nevery fixture compiles to a custom element that parses, runs and handles its actions");
