/**
 * Research lives outside `/docs` on purpose.
 *
 * The two are different kinds of writing and they were being read as one. Product documentation says
 * what the compiler does; research says what we have measured, what we have only hypothesised, and what
 * someone else found. Mixing them puts an untested hypothesis two clicks from an install command, where
 * it reads as a feature.
 *
 * So: no docs sidebar, no pager into the reference pages, and not in the docs nav. The claim discipline
 * that governs this section — measured / hypothesised / cited, never blurred — is stated once on the
 * index page and applies to everything under it.
 */
export default function ResearchLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="mx-auto max-w-(--container-page) px-6 py-12 md:px-10">
      <div className="min-w-0">{children}</div>
    </div>
  );
}
