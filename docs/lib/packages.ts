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
  /** The npm name, and the key used everywhere else. */
  name: string;
  /** One line: what you would install it *for*. */
  purpose: string;
  /** Unpacked size as the registry reports it. */
  size: string;
  /** Whether it loads outside a browser — the difference that decides `@guml/core` vs `@guml/fmt`. */
  node: boolean;
};

export const PACKAGES: readonly Pkg[] = [
  {
    name: "@guml/core",
    purpose: "The compiler as WebAssembly, plus a React runtime. Compile, render, diagnose, repair.",
    size: "959 KB",
    node: false,
  },
  {
    name: "@guml/fmt",
    purpose: "Formatter, canonical form and syntax classification. No parser, no codegen.",
    size: "231 KB",
    node: true,
  },
  {
    name: "@guml/highlight",
    purpose: "Syntax highlighting with no WebAssembly, synchronously and during server rendering.",
    size: "49 KB",
    node: true,
  },
  {
    name: "@guml/widgets",
    purpose: "chart, calendar, date, upload, command — the worked example registry package.",
    size: "22 KB",
    node: true,
  },
  {
    name: "@guml/shadcn",
    purpose: "26 tags over all 61 shadcn/ui components, for what GUML has no builtin for.",
    size: "257 KB",
    node: true,
  },
];

/** The npm page for a package. Scoped names need the slash encoded. */
export function npmUrl(name: string): string {
  return `https://www.npmjs.com/package/${name}`;
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
