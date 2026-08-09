"use client";

import { useSyncExternalStore } from "react";
import { CONSENT_KEY } from "@/lib/inline-scripts";

/**
 * `localStorage` as an external store, which is what it is.
 *
 * The obvious shape — `useState(null)` plus an effect that reads storage — sets state synchronously
 * during the effect, so every reader pays a second render before the first paint. `useSyncExternalStore`
 * is the API for exactly this: React uses `serverSnapshot` for SSR *and for the hydration render*, then
 * re-renders with the client value, so the no-flash property the effect version was written for is kept
 * without the cascading render.
 *
 * `dismissed` is not redundant with the stored key. If storage throws — private mode, a locked-down
 * browser — the write in `choose` silently does nothing, and a snapshot reading only storage would put
 * the banner straight back on screen the moment someone answered it.
 */
let dismissed = false;
let listeners: Array<() => void> = [];

function subscribe(onChange: () => void) {
  listeners.push(onChange);
  return () => {
    listeners = listeners.filter((l) => l !== onChange);
  };
}

/** True when the banner should stay hidden. */
function snapshot(): boolean {
  if (dismissed) return true;
  try {
    return localStorage.getItem(CONSENT_KEY) !== null;
  } catch {
    // Storage disabled. Treat it as undecided rather than as consent, and accept that the banner
    // reappears — the alternative is measuring someone who never agreed because their browser would
    // not let us remember that they didn't.
    return false;
  }
}

/** The server cannot see the decision, so it renders nothing rather than flashing at someone who answered. */
function serverSnapshot(): boolean {
  return true;
}

/**
 * The analytics consent gate.
 *
 * # What it is actually gating
 *
 * Not "whether we measure" — Consent Mode is already denied by default in the bootstrap, so before
 * anyone touches this GA4 sets **no cookie and no identifier** and sends only a cookieless ping. What
 * accepting changes is `analytics_storage`, which is what lets GA distinguish a returning reader from
 * a new one. That is the thing that needs permission, and it is the only thing this asks for.
 *
 * Saying so on the banner matters: a banner that implies all measurement stops on "decline" is
 * describing something that is not true, and the honest version is shorter anyway.
 *
 * # Why declining is remembered
 *
 * Storing the choice is itself storage, which is why the key is same-origin `localStorage` rather than
 * a cookie: it is never transmitted, to us or to Google. A decline that is not remembered means the
 * banner reappears on every page, which reads as being asked until you give the right answer.
 *
 * # Why it renders nothing until mounted
 *
 * The decision lives in `localStorage`, which the server cannot see. Rendering the banner during SSR
 * and hiding it on hydration would flash it at every reader who already answered — so it renders
 * nothing until the client has read the stored value, which is also why this cannot be a server
 * component.
 */
export function ConsentBanner() {
  const decided = useSyncExternalStore(subscribe, snapshot, serverSnapshot);

  const choose = (granted: boolean) => {
    try {
      localStorage.setItem(CONSENT_KEY, granted ? "granted" : "denied");
    } catch {
      // Not fatal: the update below still applies for this page view.
    }
    // `gtag` exists because the bootstrap defined it before this component ever mounted. Guarded
    // anyway — an ad blocker can remove the script and leave the page working, and a banner that
    // throws on click would be worse than no banner.
    const w = window as typeof window & { gtag?: (...args: unknown[]) => void };
    w.gtag?.("consent", "update", { analytics_storage: granted ? "granted" : "denied" });
    dismissed = true;
    for (const l of listeners) l();
  };

  if (decided) return null;

  return (
    <div
      role="dialog"
      aria-labelledby="consent-title"
      aria-describedby="consent-body"
      className="fixed inset-x-0 bottom-0 z-50 border-t border-line bg-ink/95 px-6 py-4 backdrop-blur"
    >
      <div className="mx-auto flex max-w-(--container-page) flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <p id="consent-title" className="font-mono text-sm text-chalk">
            Analytics
          </p>
          <p id="consent-body" className="mt-1 max-w-[62ch] text-sm leading-relaxed text-fog">
            We count page views with Google Analytics. Until you choose, it stores nothing on your
            device and cannot tell a returning reader from a new one — accepting allows that. No ads,
            ever.{" "}
            <a
              href="/privacy"
              className="text-iris underline decoration-iris/40 underline-offset-2 hover:decoration-iris"
            >
              Privacy
            </a>
          </p>
        </div>
        <div className="flex shrink-0 gap-2">
          <button
            type="button"
            onClick={() => choose(false)}
            className="rounded-md border border-line px-3 py-1.5 text-sm text-fog transition-colors hover:bg-chalk/5 hover:text-chalk focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-iris"
          >
            Decline
          </button>
          <button
            type="button"
            onClick={() => choose(true)}
            className="rounded-md bg-chalk px-3 py-1.5 text-sm font-medium text-ink transition-colors hover:bg-chalk/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-iris"
          >
            Accept
          </button>
        </div>
      </div>
    </div>
  );
}
