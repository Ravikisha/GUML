"use client";

import { useEffect } from "react";
import { ButtonLink } from "@/components/ui";

/**
 * The route-level error boundary.
 *
 * Distinct from `not-found.tsx` in the way that matters to a reader: a 404 is a permanent answer, so
 * that page offers a route list and no retry. This one is a *transient* failure — the compiler wasm
 * failed to instantiate, the chat endpoint timed out — so it offers a retry first and says plainly
 * that reloading may work, which on a 404 would be a lie.
 *
 * `digest` is the only server-side detail Next exposes to the client, deliberately: the real message and
 * stack stay in the server log. It is printed here because it is the string that connects what the
 * reader saw to what the log recorded, and without it a report is unactionable.
 */
export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    // Reaches the platform's log drain. Replace with an error tracker when one is wired up.
    console.error("unhandled error", error);
  }, [error]);

  return (
    <main className="mx-auto flex min-h-[70vh] max-w-3xl flex-col justify-center px-6 py-24">
      <p className="label">error</p>
      <h1 className="display-narrow mt-4 text-4xl font-medium text-chalk md:text-5xl">
        Something failed on this page
      </h1>
      <p className="mt-5 max-w-xl leading-relaxed text-fog">
        Unlike a missing page, this one may well be temporary — try again first. If it persists, the
        playground and the compiler itself run entirely in your browser, so the rest of the site is
        probably fine.
      </p>

      {error.digest ? (
        <p className="mt-6 font-mono text-xs text-fog-dim">
          reference <span className="text-fog">{error.digest}</span> — quote this if you report it
        </p>
      ) : null}

      <div className="mt-10 flex flex-wrap gap-3">
        <button
          type="button"
          onClick={reset}
          className="inline-flex items-center gap-2 rounded-md bg-chalk px-4 py-2 text-sm font-medium text-ink transition-colors hover:bg-chalk/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-iris"
        >
          Try again
        </button>
        <ButtonLink href="/docs" variant="outline">
          Documentation
        </ButtonLink>
        <ButtonLink href="/" variant="outline">
          Home
        </ButtonLink>
      </div>
    </main>
  );
}
