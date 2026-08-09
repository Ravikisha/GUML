export type NavItem = { title: string; href: string; note?: string };
export type NavGroup = { title: string; items: NavItem[] };

/**
 * Docs navigation. Groups are ordered the way someone actually adopts the
 * language: get it running, learn the surface, understand the compiler.
 *
 * **Research is deliberately not in here.** It used to be a fourth group, which
 * put an untested hypothesis two clicks from an install command — where it reads
 * as a feature rather than as an open question. It lives at `/research` now,
 * reachable from the site header, with its claim discipline stated on arrival.
 *
 * What replaced it in the reading path is `/docs/status`: what is stable, what
 * may change in a 0.x release, and what the compiler deliberately does not do.
 * That is the part of "what is not finished" a user actually needs.
 */
export const NAV: NavGroup[] = [
  {
    title: "Start",
    items: [
      { title: "What GUML is", href: "/docs" },
      { title: "Install", href: "/docs/install" },
      { title: "Quickstart", href: "/docs/quickstart" },
      { title: "Playground", href: "/playground", note: "live" },
      { title: "Chat", href: "/chat", note: "ai" },
      { title: "React library", href: "/docs/library", note: "wasm" },
      { title: "Python", href: "/docs/python", note: "pip" },
      { title: "MCP server", href: "/docs/mcp", note: "tools" },
      { title: "Status & limitations", href: "/docs/status", note: "0.1" },
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
      { title: "User components", href: "/docs/language/components", note: "def" },
      { title: "Escape hatches", href: "/docs/language/escape", note: "js" },
      { title: "Conformance levels", href: "/docs/language/levels", note: "core" },
    ],
  },
  {
    title: "Compiler",
    items: [
      { title: "Architecture", href: "/docs/compiler/architecture" },
      { title: "Diagnostics", href: "/docs/compiler/diagnostics" },
      { title: "Validator", href: "/docs/compiler/validator" },
      { title: "Formatter", href: "/docs/compiler/formatter" },
      { title: "CLI reference", href: "/docs/compiler/cli" },
      { title: "Backends", href: "/docs/compiler/backends" },
      { title: "Themes", href: "/docs/compiler/themes" },
      { title: "Config & plugins", href: "/docs/compiler/config", note: "json" },
      { title: "Capabilities & CSP", href: "/docs/compiler/capabilities", note: "csp" },
      { title: "Source maps", href: "/docs/compiler/source-maps" },
      { title: "Editor support", href: "/docs/compiler/editors", note: "lsp" },
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
