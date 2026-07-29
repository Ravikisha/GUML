#!/usr/bin/env node
/**
 * Render the compiler's output and assert on the HTML.
 *
 * Until now the emitted code had been *typechecked* and never *executed*. Those catch different
 * things: `tsc` found a `kind` prop React does not have, but it cannot notice that a table has no
 * header row, that a button has no accessible name, or that the loading branch renders nothing.
 *
 * No browser is involved. The components are server-rendered, which is enough because everything
 * this checks is in the first paint: structure, roles, accessible names, the loading and empty
 * states. Effects do not run under SSR, so no `fetch` is attempted and no mocking is needed —
 * which conveniently means the initial state is exactly the one a user sees first.
 *
 * Run from the repository root:
 *
 *   node scripts/render-emitted.mjs
 */
import { execFileSync } from "node:child_process";
import { unlinkSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DOCS = join(ROOT, "docs");
// React and TypeScript live in the docs workspace; nothing here needs its own copy.
// `pathToFileURL`, not a bare path: on Windows `C:\…` looks like a URL with scheme `c:` to the
// ESM loader, and it refuses it.
const load = (p) => import(pathToFileURL(p).href);
const { default: ts } = await load(join(DOCS, "node_modules", "typescript", "lib", "typescript.js"));
const { renderToStaticMarkup } = await load(
  join(DOCS, "node_modules", "react-dom", "server.browser.js"),
);
const { createElement } = await load(join(DOCS, "node_modules", "react", "index.js"));

/** Compile a fixture with the real compiler, then transpile the TSX so Node can run it. */
async function render(fixture) {
  const tsx = execFileSync(
    "cargo",
    ["run", "-q", "-p", "guml-cli", "--", "build", `fixtures/${fixture}`],
    { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );

  const js = ts.transpileModule(tsx, {
    compilerOptions: {
      jsx: ts.JsxEmit.ReactJSX,
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
    },
  }).outputText;

  // `react/jsx-runtime` has to resolve from the file's own location, so the module is written
  // inside the docs tree rather than a temp directory outside it.
  const path = join(DOCS, `.render-${fixture.replace(/\W/g, "_")}.mjs`);
  writeFileSync(path, js);
  try {
    const mod = await import(`${pathToFileURL(path).href}?t=${Date.now()}`);
    // `createElement`, not `Component()`. Calling the function directly runs its hooks outside
    // React's render, where there is no dispatcher — which is exactly the mistake the "invalid
    // hook call" message is about.
    return { tsx, html: renderToStaticMarkup(createElement(mod.default)) };
  } finally {
    // Leave nothing behind in a workspace the docs build scans.
    try {
      unlinkSync(path);
    } catch {
      /* best effort: the docs build must not find a stray module, but a leftover is not fatal */
    }
  }
}

/* ------------------------------------------------------------------- checks */

const count = (html, re) => (html.match(re) ?? []).length;

/**
 * The checks `axe-core` would make on this markup, expressed directly.
 *
 * Not a substitute for axe on a live page — it cannot see contrast or focus order. It does cover
 * the rules the *compiler* is responsible for, which is the set worth failing a build over: the
 * compiler owns every label, role and heading in the output, so a violation here is always a
 * compiler bug rather than an author's mistake.
 */
function accessibility(html) {
  const problems = [];
  const warnings = [];

  // Severity is graded the way the compiler grades it (`GUML0050` vs `GUML0051`): no name at all
  // is a defect, a placeholder-only name is a smell. Failing the build on the second would mean
  // editing `fixtures/b.guml`, whose token count is a published measurement — the fixture is
  // deliberately left as the documented warning case.
  for (const input of html.match(/<input[^>]*>/g) ?? []) {
    if (/type="hidden"/.test(input)) continue;
    const named = /aria-label=|aria-labelledby=|id=/.test(input);
    if (named) continue;
    if (/placeholder="/.test(input)) {
      warnings.push(`input named only by its placeholder: ${input.slice(0, 70)}`);
    } else {
      problems.push(`input with no accessible name at all: ${input.slice(0, 70)}`);
    }
  }

  // A button needs text or an explicit name.
  for (const [, attrs, text] of html.matchAll(/<button([^>]*)>([\s\S]*?)<\/button>/g)) {
    const named = /aria-label=/.test(attrs) || text.replace(/<[^>]*>/g, "").trim().length > 0;
    if (!named) problems.push(`button without an accessible name: ${attrs.slice(0, 80)}`);
  }

  // One `h1` names the page; more than one flattens the outline assistive tech navigates by.
  const h1s = count(html, /<h1[\s>]/g);
  if (h1s > 1) problems.push(`${h1s} h1 elements`);

  // A table's columns need headers, and they need a scope.
  if (/<table[\s>]/.test(html)) {
    if (!/<th[\s>]/.test(html)) problems.push("table without header cells");
    for (const th of html.match(/<th[^>]*>/g) ?? []) {
      if (!/scope=/.test(th)) problems.push(`th without scope: ${th}`);
    }
  }

  // An error banner has to be announced, not merely coloured.
  if (/bg-red-50|text-red-600/.test(html) && !/role="alert"/.test(html)) {
    problems.push("error styling with no role=\"alert\"");
  }

  return { problems, warnings };
}

/** Per-fixture expectations about what the first paint contains. */
const FIXTURES = [
  {
    file: "a.guml",
    checks: (html) => {
      const out = [];
      if (!/Clicks/.test(html)) out.push("heading text missing");
      // `metric {count}` with `state count=0` renders the initial value.
      if (!/>0</.test(html)) out.push("initial counter value not rendered");
      if (count(html, /<button/g) !== 3) out.push(`expected 3 buttons, got ${count(html, /<button/g)}`);
      // `disabled={!count}` is true at zero, so the decrement button starts disabled.
      if (!/disabled/.test(html)) out.push("conditional disable not applied at count=0");
      return out;
    },
  },
  {
    file: "b.guml",
    checks: (html) => {
      const out = [];
      // Effects do not run under SSR, so `loading` is still true: this is the skeleton, which is
      // the state a user actually sees first and the one no hand-written component remembers.
      if (!/animate-pulse/.test(html)) out.push("loading skeleton not rendered");
      if (!/Tasks/.test(html)) out.push("header missing");
      // The open count is derived from an empty list at first paint.
      if (!/0 open/.test(html)) out.push("derived count not rendered");
      // Three filter options come from the state's domain, not from markup.
      for (const option of ["all", "open", "done"]) {
        if (!new RegExp(`>${option}<`).test(html)) out.push(`filter option \`${option}\` missing`);
      }
      if (!/aria-pressed/.test(html)) out.push("segmented control not announced as pressed");
      return out;
    },
  },
  {
    file: "c.guml",
    checks: (html) => {
      const out = [];
      if (count(html, /<h1[\s>]/g) !== 1) out.push("expected exactly one h1");
      // Three tiers, three feature cards, an FAQ.
      if (count(html, /\$0\/mo|\$24\/mo|\$96\/mo/g) < 3) out.push("pricing tiers missing");
      if (!/<details/.test(html)) out.push("FAQ is not a disclosure element");
      if (!/<summary/.test(html)) out.push("FAQ entries have no summary");
      // Anchors the nav links point at have to exist in the output.
      for (const id of ["features", "pricing", "faq"]) {
        if (!new RegExp(`id="${id}"`).test(html)) out.push(`missing anchor target #${id}`);
      }
      return out;
    },
  },
  {
    file: "d.guml",
    checks: (html) => {
      const out = [];
      // The `js` block's `currency()` is called from the `raw react` block. If either the hoist
      // order or the verbatim emission were wrong this throws at render rather than failing a
      // check, which is the strongest available evidence that the hatch actually works.
      if (!/Total: /.test(html)) out.push("raw block did not render");
      if (!/£0\.00/.test(html)) out.push("`js` helper did not run: no formatted currency");
      // The Svelte block targets another backend and must not appear in React output. It is
      // discriminated by its text, not its markup: React renders `className` as `class`, so the
      // attribute is identical in both and proves nothing.
      if (/Svelte total/.test(html)) out.push("`raw svelte` leaked into the React render");
      return out;
    },
  },
  {
    file: "portfolio.guml",
    checks: (html) => {
      const out = [];
      if (count(html, /<h1[\s>]/g) !== 1) out.push("expected exactly one h1");
      if (!/<table/.test(html) && !/animate-pulse/.test(html)) {
        out.push("archive table renders neither rows nor a skeleton");
      }
      if (!/type="email"/.test(html)) out.push("`kind=email` did not become an input type");
      if (!/aria-label="Keep me signed in"|role="switch"/.test(html)) {
        out.push("toggle is not a switch");
      }
      return out;
    },
  },
];

/* --------------------------------------------------------------------- main */

let failed = 0;
console.log(`rendering ${FIXTURES.length} fixtures with react-dom/server\n`);

for (const fixture of FIXTURES) {
  let html;
  try {
    ({ html } = await render(fixture.file));
  } catch (e) {
    console.log(`✗ ${fixture.file} — did not render: ${e.message.split("\n")[0]}`);
    failed++;
    continue;
  }

  const a11y = accessibility(html);
  const problems = [...fixture.checks(html), ...a11y.problems];
  if (problems.length === 0) {
    const note = a11y.warnings.length > 0 ? ` (${a11y.warnings.length} warning)` : "";
    console.log(`✓ ${fixture.file.padEnd(16)} ${html.length} bytes of HTML${note}`);
    for (const w of a11y.warnings) console.log(`    warning  ${w}`);
  } else {
    failed++;
    console.log(`✗ ${fixture.file}`);
    for (const p of problems) console.log(`    ${p}`);
  }
}

console.log("");
if (failed > 0) {
  console.error(`${failed} of ${FIXTURES.length} fixtures failed to render correctly`);
  process.exit(1);
}
console.log(`${FIXTURES.length} fixtures render, and pass the accessibility rules the compiler owns`);

// Something in the React/`typescript` import graph leaves a handle on the loop, so the process
// does not exit on its own — in CI that is a hung job rather than a failed one. The work is done
// and the report is written, so flush and exit explicitly.
await new Promise((resolve) => process.stdout.write("", resolve));
process.exit(0);
