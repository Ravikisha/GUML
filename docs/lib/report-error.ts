/**
 * Where a client-side error goes.
 *
 * # Why this is not just `console.error`
 *
 * `error.tsx` and `global-error.tsx` had a bare `console.error`, which reaches a browser console
 * nobody is looking at. In production that is indistinguishable from having no error handling: the
 * reader sees a fallback page, and the only record of *why* exists on their machine.
 *
 * # Why it posts to our own route rather than importing a vendor SDK
 *
 * Three reasons, in order of how much they mattered:
 *
 * 1. **The CSP.** `connect-src` is this origin plus the analytics collector. A vendor SDK would need
 *    its host added, which widens the policy for every page — and the policy is the thing that makes
 *    "no third-party trackers" enforced rather than promised.
 * 2. **No new dependency, and no new consent question.** An error report carries a stack trace and a
 *    URL, which is operational data about our own service, not behavioural data about a reader. Adding
 *    a third-party processor would change that answer and put another entry on the privacy page.
 * 3. It is about thirty lines.
 *
 * `/api/error` decides what to do with it. Today that is a structured server log, which the platform
 * already aggregates; pointing it at Sentry later is a change in one file.
 *
 * # What it deliberately does not send
 *
 * No cookies, no storage, no identifiers. `keepalive` so a report survives the navigation that often
 * accompanies a crash, and a hard failure here is swallowed — an error reporter that throws inside an
 * error boundary produces an infinite loop, which is a worse outcome than a lost report.
 */

export type ErrorContext = {
  /** Where it happened, so a report is actionable without asking. */
  boundary: "route" | "root";
  /** Next's server-side error id, the only handle connecting this to a server log. */
  digest?: string;
};

export function reportError(error: Error, context: ErrorContext): void {
  // Always, so a developer with the console open still sees it immediately.
  console.error(`[${context.boundary}]`, error);

  if (typeof window === "undefined") return;

  try {
    const body = JSON.stringify({
      message: error.message,
      // Truncated: a stack is unbounded, and the frames that identify a fault are at the top.
      stack: error.stack?.slice(0, 4000),
      digest: context.digest,
      boundary: context.boundary,
      url: window.location.pathname + window.location.search,
      userAgent: navigator.userAgent,
    });

    // `keepalive` lets the request outlive the page. A crash is frequently followed by a reload, and
    // without it the report is cancelled in flight — losing exactly the errors worth having.
    void fetch("/api/error", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body,
      keepalive: true,
      // Nothing here needs a cookie, and sending one would make an operational ping into something
      // the privacy page would have to describe differently.
      credentials: "omit",
    }).catch(() => {
      // Offline, blocked, or rate-limited. Nothing useful to do, and rethrowing inside an error
      // boundary is how a broken page becomes a loop.
    });
  } catch {
    // Serialising failed — a circular reference in a thrown object, most likely. Still not worth
    // taking the page down over.
  }
}
