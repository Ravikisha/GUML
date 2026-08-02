/**
 * Every package the docs name must exist on npm, and the sizes quoted must be the sizes npm reports.
 *
 * Two failures this prevents, both of which are invisible to a build:
 *
 *   * **A link to a 404.** `lib/packages.ts` is the only place a package name is written, and `<Pkg>`
 *     throws on an unknown one — but neither knows whether the name was ever *published*. A typo in
 *     the data file produces a page that builds, typechecks, renders, and sends readers to a registry
 *     page saying the package does not exist.
 *   * **A stale size.** The whole argument for splitting `@guml/fmt` and `@guml/highlight` out of the
 *     core is a size comparison. Republish either and the numbers on the page keep asserting the old
 *     ones with no indication anything moved — and a wrong measured number is exactly what this
 *     project's claim discipline forbids shipping.
 *
 * Sizes are compared with a tolerance rather than exactly: the docs round to whole kilobytes, and a
 * patch release that changes a comment should not fail the build. A 5% drift is real and worth a
 * failure; a 200-byte one is not.
 *
 * Network-dependent, so it degrades to a skip when the registry is unreachable. That is the right
 * trade for a check that protects against staleness rather than against a bug: an offline contributor
 * should not be blocked, and CI has a network.
 */

import process from "node:process";

const TOLERANCE = 0.05;

const packages = await import("../lib/packages.ts").then((m) => m.PACKAGES);

/** Registry metadata for a package, or null if the network is unavailable. */
async function fetchMeta(name) {
  const res = await fetch(`https://registry.npmjs.org/${name.replace("/", "%2f")}`);
  if (res.status === 404) return { missing: true };
  if (!res.ok) throw new Error(`registry returned ${res.status} for ${name}`);
  const body = await res.json();
  const latest = body["dist-tags"]?.latest;
  const version = body.versions?.[latest];
  return { version: latest, unpackedSize: version?.dist?.unpackedSize };
}

const kb = (bytes) => bytes / 1024;
const parseKb = (s) => Number.parseFloat(String(s).replace(/[^\d.]/g, ""));

let failures = 0;
let checked = 0;

for (const p of packages) {
  let meta;
  try {
    meta = await fetchMeta(p.name);
  } catch (e) {
    console.log(`  skip   ${p.name} — registry unreachable (${e.message})`);
    continue;
  }

  if (meta.missing) {
    console.error(`  FAIL   ${p.name} is not published — the docs link to a 404`);
    failures++;
    continue;
  }

  checked++;
  const actual = kb(meta.unpackedSize);
  const claimed = parseKb(p.size);
  const drift = Math.abs(actual - claimed) / actual;

  if (drift > TOLERANCE) {
    console.error(
      `  FAIL   ${p.name}@${meta.version} is ${actual.toFixed(0)} KB, ` +
        `lib/packages.ts says ${p.size} (${(drift * 100).toFixed(0)}% out)`,
    );
    failures++;
  } else {
    console.log(`  ok     ${p.name}@${meta.version} ${actual.toFixed(0)} KB`);
  }
}

if (failures) {
  console.error(`\n${failures} package(s) wrong. Update docs/lib/packages.ts to match the registry.`);
  process.exit(1);
}

console.log(
  checked
    ? `\n${checked} package(s) published, and the sizes the docs quote match the registry`
    : "\nno packages checked — the registry was unreachable",
);
