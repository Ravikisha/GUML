import type { MetadataRoute } from "next";
import { FLAT_NAV } from "@/lib/nav";

/**
 * Every route, derived from the navigation rather than listed again.
 *
 * A hand-maintained sitemap goes stale the first time a page is added, and the failure is
 * invisible — the page simply never gets indexed. `FLAT_NAV` is already the single source of
 * truth for the sidebar, so it is the source of truth here too.
 */
export default function sitemap(): MetadataRoute.Sitemap {
  const base = "https://guml.dev";
  // One timestamp for the whole build: the pages are generated together, and pretending to
  // know when each one last changed would be a fabricated signal.
  const lastModified = new Date();

  const routes = ["/", "/examples", ...FLAT_NAV.map((item) => item.href)];

  return [...new Set(routes)].map((href) => ({
    url: `${base}${href === "/" ? "" : href}`,
    lastModified,
    changeFrequency: "weekly",
    // The landing page and the two interactive pages are the entry points; everything else
    // is reference material reached from them.
    priority: href === "/" ? 1 : ["/playground", "/chat", "/docs"].includes(href) ? 0.8 : 0.6,
  }));
}
