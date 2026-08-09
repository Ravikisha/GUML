/**
 * Accessibility rules over the *rendered* docs site.
 *
 * # Why this exists
 *
 * The compiler enforces accessibility contracts on the HTML it emits — a theme without a focus
 * treatment is refused, a control without an accessible name is a diagnostic, `render-emitted.mjs`
 * parses every fixture and checks the rules. The *documentation site for that compiler* had never been
 * checked at all. That is an awkward gap to have shipped.
 *
 * # What it checks, and what it cannot
 *
 * These are the faults that are decidable from static HTML: a missing `lang`, an image with no `alt`,
 * a control with no accessible name, a heading level skipped, a duplicate `id`, a link whose only text
 * is "here". They are also most of what a generated or hand-built page actually gets wrong.
 *
 * It cannot see contrast (needs computed styles), focus order (needs a layout), or anything that only
 * exists after hydration. A browser-based axe-core run would cover those and needs a headless browser
 * in CI; this needs `fetch` and runs in two seconds, so it is the version that will actually be kept.
 * The two are complementary rather than alternatives.
 *
 * Runs against a *built* site, not the source: a rule about rendered output must read rendered output,
 * and a component that looks fine in TSX can still emit a nameless button.
 */

import process from "node:process";
import { parse } from "node-html-parser";

const BASE = process.env.A11Y_BASE ?? "http://localhost:3995";

/** Every routed page. Kept explicit so a new page is a deliberate addition to the audit. */
const ROUTES = [
  "/",
  "/docs",
  "/docs/install",
  "/docs/quickstart",
  "/docs/status",
  "/docs/python",
  "/docs/mcp",
  "/docs/library",
  "/docs/compiler/config",
  "/docs/compiler/themes",
  "/docs/compiler/backends",
  "/docs/language/syntax",
  "/docs/language/registry",
  "/research",
  "/research/measurements",
  "/examples",
  "/playground",
  "/chat",
  "/privacy",
];

/** Text that identifies a control to a screen reader, if there is any. */
function accessibleName(el) {
  const aria = el.getAttribute("aria-label");
  if (aria?.trim()) return aria.trim();
  if (el.getAttribute("aria-labelledby")?.trim()) return "(labelledby)";
  if (el.getAttribute("title")?.trim()) return el.getAttribute("title").trim();
  const text = el.text.replace(/\s+/g, " ").trim();
  if (text) return text;
  // An icon-only control is named by whatever its `<img>`/`<svg>` exposes.
  const img = el.querySelector("img[alt]");
  if (img?.getAttribute("alt")?.trim()) return img.getAttribute("alt").trim();
  return "";
}

const VAGUE = new Set(["here", "click here", "read more", "more", "link", "this"]);

function audit(html, route) {
  const doc = parse(html);
  const problems = [];
  const at = (rule, detail) => problems.push({ route, rule, detail });

  // A document with no language makes a screen reader guess pronunciation, and it is the first thing
  // every checker looks for.
  const lang = doc.querySelector("html")?.getAttribute("lang");
  if (!lang) at("html-lang", "no lang attribute on <html>");

  if (!doc.querySelector("title")?.text?.trim()) at("title", "no non-empty <title>");

  // Exactly one `main`. Zero means no skip target; two is ambiguous.
  const mains = doc.querySelectorAll("main");
  if (mains.length !== 1) at("main-landmark", `${mains.length} <main> elements`);

  const h1s = doc.querySelectorAll("h1");
  if (h1s.length !== 1) at("one-h1", `${h1s.length} <h1> elements`);

  // Heading levels must not skip: an h2 followed by an h4 tells a screen-reader user a level is
  // missing and they have lost their place.
  let previous = 0;
  for (const h of doc.querySelectorAll("h1, h2, h3, h4, h5, h6")) {
    const level = Number(h.tagName[1]);
    if (previous && level > previous + 1) {
      at("heading-skip", `h${previous} -> h${level} at "${h.text.trim().slice(0, 40)}"`);
    }
    previous = level;
  }

  const ids = doc.querySelectorAll("[id]").map((e) => e.getAttribute("id"));
  const duplicates = [...new Set(ids.filter((i) => ids.filter((x) => x === i).length > 1))];
  if (duplicates.length) at("duplicate-id", duplicates.slice(0, 5).join(", "));

  for (const img of doc.querySelectorAll("img")) {
    if (img.getAttribute("alt") === null) {
      at("img-alt", `<img src="${(img.getAttribute("src") ?? "").slice(0, 60)}">`);
    }
  }

  for (const el of doc.querySelectorAll("button, a[href]")) {
    // `aria-hidden` content is deliberately not exposed, and a decorative element is allowed to be
    // nameless — that is what the attribute means.
    if (el.getAttribute("aria-hidden") === "true") continue;
    const name = accessibleName(el);
    if (!name) {
      at("control-name", `<${el.tagName.toLowerCase()}> with no accessible name`);
    } else if (el.tagName === "A" && VAGUE.has(name.toLowerCase())) {
      at("link-text", `link text "${name}" says nothing out of context`);
    }
  }

  for (const input of doc.querySelectorAll("input, select, textarea")) {
    if (input.getAttribute("type") === "hidden") continue;
    const id = input.getAttribute("id");
    const labelled =
      input.getAttribute("aria-label")?.trim() ||
      input.getAttribute("aria-labelledby")?.trim() ||
      (id && doc.querySelector(`label[for="${id}"]`));
    if (!labelled) at("field-label", `<${input.tagName.toLowerCase()}> with no label`);
  }

  // A positive tabindex overrides the document's natural focus order for the whole page, which is
  // almost never what was meant.
  for (const el of doc.querySelectorAll("[tabindex]")) {
    if (Number(el.getAttribute("tabindex")) > 0) {
      at("tabindex", `tabindex="${el.getAttribute("tabindex")}" reorders focus`);
    }
  }

  return problems;
}

const all = [];
let checked = 0;

for (const route of ROUTES) {
  let html;
  try {
    const res = await fetch(BASE + route);
    if (!res.ok) {
      all.push({ route, rule: "status", detail: `HTTP ${res.status}` });
      continue;
    }
    html = await res.text();
  } catch (e) {
    console.error(`cannot reach ${BASE}${route} — is the site running? (${e.message})`);
    process.exit(1);
  }
  checked++;
  all.push(...audit(html, route));
}

// ---------------------------------------------------------------------------------------------
// Every page audited here must also be discoverable.
//
// `sitemap.ts` derives from `FLAT_NAV`, so a page reachable some other way — the footer, a banner, a
// link in prose — is invisible to it. That has now dropped real pages twice: three research pages when
// the nav group was removed, then `/privacy`. Both times the page worked perfectly and was simply
// never indexed, which is the kind of failure nothing surfaces.
//
// This route list already exists for the audit, so reusing it costs nothing and closes the class.
try {
  const xml = await (await fetch(`${BASE}/sitemap.xml`)).text();
  const missing = ROUTES.filter((r) => !xml.includes(r === "/" ? "app</loc>" : `${r}<`));
  if (missing.length) {
    console.error(`
${missing.length} page(s) are not in the sitemap and will never be indexed:`);
    for (const r of missing) console.error(`    ${r}`);
    console.error("  add them to `outsideTheDocsNav` in app/sitemap.ts");
    all.push({ route: "sitemap", rule: "coverage", detail: missing.join(", ") });
  } else {
    console.log(`all ${ROUTES.length} audited pages are in the sitemap`);
  }
} catch (e) {
  console.error(`could not read the sitemap: ${e.message}`);
}

if (all.length) {
  console.error(`${all.length} accessibility problem(s) across ${checked} pages:\n`);
  const byRule = {};
  for (const p of all) (byRule[p.rule] ??= []).push(p);
  for (const [rule, items] of Object.entries(byRule)) {
    console.error(`  ${rule} (${items.length})`);
    for (const i of items.slice(0, 6)) console.error(`    ${i.route}  ${i.detail}`);
    if (items.length > 6) console.error(`    … and ${items.length - 6} more`);
  }
  process.exit(1);
}

console.log(`${checked} pages pass the accessibility rules that static HTML can decide`);
