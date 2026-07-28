import type { Metadata, Viewport } from "next";
import { Bricolage_Grotesque, Geist, Geist_Mono } from "next/font/google";
import { SiteFooter } from "@/components/site-footer";
import { themeScript } from "@/components/theme-toggle";
import { SiteNav } from "@/components/site-nav";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
  display: "swap",
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
  display: "swap",
});

/**
 * Bricolage Grotesque is here for one reason: it carries a width axis, and the
 * hero narrows that axis while the token counter falls, so the display type
 * compresses along with the data it describes.
 */
const bricolage = Bricolage_Grotesque({
  variable: "--font-bricolage",
  subsets: ["latin"],
  display: "swap",
  axes: ["opsz", "wdth"],
});

export const metadata: Metadata = {
  metadataBase: new URL("https://guml.dev"),
  title: {
    default: "GUML — write less, ship the same app",
    template: "%s · GUML",
  },
  description:
    "GUML is a token-efficient intermediate representation and compiler for LLM-generated web applications: 173 tokens of markup in place of 1,434 tokens of React.",
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
      className={`${geistSans.variable} ${geistMono.variable} ${bricolage.variable}`}
      suppressHydrationWarning
    >
      <head>
        {/* Applies the stored theme before first paint; a deferred script would
            let a dark frame flash for a reader who chose light. */}
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
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
