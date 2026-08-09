/**
 * The published npm packages, in one place.
 *
 * Written down once because the alternative is a URL typed out at every mention. There are five
 * packages named across four pages, and a registry URL is the kind of thing that is only ever noticed
 * when it 404s — which, for a link to an install page, is the worst moment to find out.
 *
 * `size` is the **unpacked** size reported by the registry, not the tarball. That is the number that
 * answers "what am I adding to my app", and it is the one the split between these packages exists to
 * change: 49 KB against 959 KB is the whole argument for `@guml/highlight` being separate.
 *
 * Sizes are measured, and go stale when a package is republished. `pnpm check:packages` compares every
 * entry here against the registry and fails on a mismatch, so they cannot drift quietly.
 */

export type Pkg = {
  /** The package name, and the key used everywhere else. */
  name: string;
  /** Which index it comes from. There are two now, and their URLs differ. */
  registry: "npm" | "pypi";
  /** One line: what you would install it *for*. */
  purpose: string;
  /** Unpacked size as the registry reports it. */
  size: string;
  /** Whether it loads outside a browser — the difference that decides `@guml/core` vs `@guml/fmt`. */
  node: boolean;
  /**
   * Live on its index yet.
   *
   * A package can be finished, tested and documented before it is published — PyPI uses trusted
   * publishing, which only fires from a tagged release. Until then the docs should *describe* it and
   * not *link* to it, because a link to a registry 404 reads as "this package does not exist" rather
   * than "this has not shipped yet". `<Pkg>` renders an unpublished entry as plain code, and
   * `check:packages` skips it instead of failing.
   */
  published?: boolean;
};

export const PACKAGES: readonly Pkg[] = [
  {
    name: "@guml/core",
    registry: "npm",
    purpose: "The compiler as WebAssembly, plus a React runtime. Compile, render, diagnose, repair.",
    size: "959 KB",
    node: false,
  },
  {
    name: "@guml/fmt",
    registry: "npm",
    purpose: "Formatter, canonical form and syntax classification. No parser, no codegen.",
    size: "231 KB",
    node: true,
  },
  {
    name: "@guml/highlight",
    registry: "npm",
    purpose: "Syntax highlighting with no WebAssembly, synchronously and during server rendering.",
    size: "49 KB",
    node: true,
  },
  {
    name: "@guml/widgets",
    registry: "npm",
    purpose: "chart, calendar, date, upload, command — the worked example registry package.",
    size: "22 KB",
    node: true,
  },
  {
    name: "@guml/shadcn",
    registry: "npm",
    purpose: "26 tags over all 61 shadcn/ui components, for what GUML has no builtin for.",
    size: "257 KB",
    node: true,
  },
  {
    name: "guml",
    registry: "pypi",
    purpose:
      "The compiler for Python: render to HTML from Flask, FastAPI or Django, and drive an LLM repair loop.",
    size: "3.1 MB",
    node: true,
  },
];

/** The index page for a package. Scoped names need the slash encoded. */
export function packageUrl(p: Pkg): string {
  return p.registry === "pypi"
    ? `https://pypi.org/project/${p.name}/`
    : `https://www.npmjs.com/package/${p.name}`;
}

export function pkg(name: string): Pkg {
  const found = PACKAGES.find((p) => p.name === name);
  if (!found) {
    // A typo'd package name would otherwise render as a link to a 404, which reads as a broken
    // registry rather than a broken page. Failing the build is the cheaper failure.
    throw new Error(`unknown package "${name}" — add it to lib/packages.ts`);
  }
  return found;
}
