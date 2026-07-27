export type NavItem = { title: string; href: string; note?: string };
export type NavGroup = { title: string; items: NavItem[] };

/**
 * Docs navigation. Groups are ordered the way someone actually adopts the
 * language: get it running, learn the surface, understand the compiler, then
 * judge the evidence.
 */
export const NAV: NavGroup[] = [
  {
    title: "Start",
    items: [
      { title: "What GUML is", href: "/docs" },
      { title: "Install", href: "/docs/install" },
      { title: "Quickstart", href: "/docs/quickstart" },
    ],
  },
  {
    title: "Language",
    items: [
      { title: "Syntax", href: "/docs/language/syntax" },
      { title: "Directives", href: "/docs/language/directives" },
      { title: "Elements", href: "/docs/language/elements" },
      { title: "Modifiers", href: "/docs/language/modifiers" },
      { title: "Bindings & actions", href: "/docs/language/bindings" },
      { title: "Component registry", href: "/docs/language/registry" },
    ],
  },
  {
    title: "Compiler",
    items: [
      { title: "Architecture", href: "/docs/compiler/architecture" },
      { title: "Diagnostics", href: "/docs/compiler/diagnostics" },
      { title: "CLI reference", href: "/docs/compiler/cli" },
      { title: "Backends", href: "/docs/compiler/backends" },
    ],
  },
  {
    title: "Research",
    items: [
      { title: "Measurements", href: "/docs/research/measurements" },
      { title: "Phase 0 gate", href: "/docs/research/phase0", note: "open" },
      { title: "Roadmap", href: "/docs/research/roadmap" },
      { title: "Prior art", href: "/docs/research/prior-art" },
    ],
  },
];

export const FLAT_NAV: NavItem[] = NAV.flatMap((g) => g.items);

export function neighbours(pathname: string) {
  const i = FLAT_NAV.findIndex((item) => item.href === pathname);
  return {
    prev: i > 0 ? FLAT_NAV[i - 1] : undefined,
    next: i >= 0 && i < FLAT_NAV.length - 1 ? FLAT_NAV[i + 1] : undefined,
  };
}
