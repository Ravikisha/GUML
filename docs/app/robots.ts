import type { MetadataRoute } from "next";

/**
 * Crawlers get the documentation and nothing else.
 *
 * `/api/` is disallowed because the chat endpoints are stateful, rate-limited and cost money
 * per call — a crawler walking them would spend the demo's daily cap on nobody. The quota
 * enforces that regardless; this just avoids the pointless traffic.
 */
export default function robots(): MetadataRoute.Robots {
  return {
    rules: {
      userAgent: "*",
      allow: "/",
      disallow: ["/api/"],
    },
    sitemap: "https://guml.dev/sitemap.xml",
  };
}
