import type { Metadata } from "next";
import { ButtonLink } from "@/components/ui";
import { FLAT_NAV } from "@/lib/nav";

export const metadata: Metadata = {
  title: "Not found",
};

/**
 * A 404 that does something useful.
 *
 * The interesting property of this site is that the compiler runs in the browser, so the most
 * helpful thing a dead link can offer is the playground rather than an apology. The route list
 * comes from the navigation, so it cannot list pages that no longer exist.
 */
export default function NotFound() {
  return (
    <main className="mx-auto flex min-h-[70vh] max-w-3xl flex-col justify-center px-6 py-24">
      <p className="label">404</p>
      <h1 className="display-narrow mt-4 text-4xl font-extrabold tracking-tight text-chalk md:text-5xl">
        No page at this address
      </h1>
      <p className="mt-5 max-w-xl leading-relaxed text-fog">
        The link is wrong or the page has moved. Nothing here is generated dynamically, so this
        is not a temporary failure — one of the routes below is the one you wanted.
      </p>

      <div className="mt-8 flex flex-wrap gap-3">
        <ButtonLink href="/docs">Read the docs</ButtonLink>
        <ButtonLink href="/playground" variant="outline">
          Open the playground
        </ButtonLink>
      </div>

      <nav aria-label="All pages" className="mt-12 border-t border-line pt-6">
        <p className="label">every page</p>
        <ul className="mt-4 grid gap-x-8 gap-y-2 sm:grid-cols-2">
          {FLAT_NAV.map((item) => (
            <li key={item.href}>
              <a
                href={item.href}
                className="font-mono text-sm text-fog transition-colors hover:text-chalk"
              >
                {item.title}
              </a>
            </li>
          ))}
        </ul>
      </nav>
    </main>
  );
}
