import type { Metadata, Viewport } from "next";
import { Geist_Mono, Manrope } from "next/font/google";
import { SiteFooter } from "@/components/site-footer";
import { SiteNav } from "@/components/site-nav";
import {
  analyticsEnabled,
  analyticsScript,
  GA_MEASUREMENT_ID,
  GA_SCRIPT_ORIGIN,
  themeScript,
} from "@/lib/inline-scripts";
import "./globals.css";

/**
 * One face for the entire voice, at 400/450/500 and no heavier.
 *
 * Manrope is a geometric grotesque with a real weight axis in that range, which is what this
 * direction needs: hierarchy is carried by size and tight leading, never by boldness. Loading a
 * single family also means the 155px hero and a 12px label are unmistakably the same typeface —
 * the thing that makes a page read as designed rather than assembled.
 */
const manrope = Manrope({
  variable: "--font-manrope",
  subsets: ["latin"],
  display: "swap",
  weight: ["400", "500"],
});

/**
 * The one exception, and it is about content rather than voice: GUML's syntax *is* indentation,
 * so a source listing has to be set in a face where columns line up. A proportional code sample
 * would misrepresent the language the site documents.
 */
const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
  display: "swap",
});

export const metadata: Metadata = {
  metadataBase: new URL("https://guml.vercel.app"),
  title: {
    default: "GUML — write less, ship the same app",
    template: "%s · GUML",
  },
  description:
    "GUML is a token-efficient intermediate representation and compiler for LLM-generated web applications: 178 tokens of markup in place of 1,441 tokens of React.",
  keywords: [
    "GUML",
    "generative UI",
    "LLM code generation",
    "intermediate representation",
    "compiler",
    "token efficiency",
    "React codegen",
  ],
  openGraph: {
    title: "GUML — Generative UI Markup Language",
    description:
      "An intermediate representation and compiler for LLM-generated interfaces. Measured 4.4–8.3× fewer output tokens than hand-written React.",
    type: "website",
  },
};

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#f6f7f9" },
    { media: "(prefers-color-scheme: dark)", color: "#08080c" },
  ],
  colorScheme: "dark light",
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    // data-scroll-behavior: as of Next 16 the framework no longer overrides CSS
    // smooth scrolling during route transitions unless asked, and instant
    // scroll-to-top on navigation is the behaviour this site wants.
    <html
      lang="en"
      data-scroll-behavior="smooth"
      className={`${manrope.variable} ${geistMono.variable}`}
      suppressHydrationWarning
    >
      <head>
        {/* Applies the stored theme before first paint; a deferred script would
            let a dark frame flash for a reader who chose light.

            Both scripts here are allowed by the CSP through a SHA-256 hash of their
            exact contents, computed in `next.config.ts` from the same strings. Edit
            one anywhere other than `lib/inline-scripts.ts` and the browser will
            refuse to run it — silently, since a blocked script is not an error. */}
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />

        {analyticsEnabled ? (
          <>
            <script async src={`${GA_SCRIPT_ORIGIN}/gtag/js?id=${GA_MEASUREMENT_ID}`} />
            <script dangerouslySetInnerHTML={{ __html: analyticsScript }} />
          </>
        ) : null}
      </head>
      <body className="flex min-h-dvh flex-col antialiased">
        <a
          href="#content"
          className="sr-only focus:not-sr-only focus:fixed focus:top-4 focus:left-4 focus:z-100 focus:rounded-full focus:bg-chalk focus:px-4 focus:py-2 focus:text-sm focus:text-ink"
        >
          Skip to content
        </a>
        <SiteNav />
        <main id="content" className="flex-1">
          {children}
        </main>
        <SiteFooter />
      </body>
    </html>
  );
}
