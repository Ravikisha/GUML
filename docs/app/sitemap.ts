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
  const base = "https://guml.vercel.app";
  // One timestamp for the whole build: the pages are generated together, and pretending to
  // know when each one last changed would be a fabricated signal.
  const lastModified = new Date();

  // Research is deliberately not in `FLAT_NAV` — it sits outside the docs so that an untested
  // hypothesis is not two clicks from an install command. That makes it the one thing the derive-from-nav
  // trick above cannot see, and the failure is the silent kind this file exists to avoid: taking the
  // group out of the sidebar dropped three live pages out of the sitemap in the same commit.
  const outsideTheDocsNav = [
    "/research",
    "/research/measurements",
    "/research/prior-art",
    // Reachable from the footer and the consent banner rather than the docs nav, which is exactly the
    // shape of page this list exists for — the second time a real page was missing from the sitemap
    // for the same reason.
    "/privacy",
  ];

  const routes = ["/", "/examples", ...FLAT_NAV.map((item) => item.href), ...outsideTheDocsNav];

  return [...new Set(routes)].map((href) => ({
    url: `${base}${href === "/" ? "" : href}`,
    lastModified,
    changeFrequency: "weekly",
    // The landing page and the two interactive pages are the entry points; everything else
    // is reference material reached from them.
    priority: href === "/" ? 1 : ["/playground", "/chat", "/docs"].includes(href) ? 0.8 : 0.6,
  }));
}
