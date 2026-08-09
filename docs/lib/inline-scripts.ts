/**
 * Every inline `<script>` this site emits, in one place.
 *
 * **Why they live here rather than next to the components that use them.** The Content-Security-Policy
 * in `next.config.ts` does not allow `'unsafe-inline'` for scripts, which means every inline script has
 * to be allowed individually by the SHA-256 hash of its exact contents. That hash has to be computed
 * from the same string the browser receives — not a copy of it — or the policy silently blocks a script
 * that looks correct in the source.
 *
 * "Silently" is the operative word, and it is not hypothetical: adding the CSP broke the theme script
 * below, whose entire job is to prevent a flash of the wrong theme. Nothing failed, no build broke, no
 * test noticed. The page just flashed white for a reader who had chosen dark, which is exactly the bug
 * the script was written to prevent.
 *
 * So: this module has no imports and no `"use client"` directive, because `next.config.ts` imports it
 * from Node at build time to hash these strings, and the layout imports it to render them. One source,
 * two consumers, no way for them to drift.
 *
 * Adding an inline script means adding it to `INLINE_SCRIPTS` too. Forgetting is the failure mode this
 * whole arrangement exists to make impossible, so it is worth saying twice.
 */

/** Where the reader's explicit light/dark choice is stored. */
export const THEME_KEY = "guml-theme";

/**
 * Applies the stored theme before first paint.
 *
 * Has to be inline and synchronous in `<head>`. A deferred or external script runs after the first
 * paint, which is one frame of the wrong background — the precise thing being avoided.
 */
export const themeScript =
  `try{var t=localStorage.getItem("${THEME_KEY}");` +
  `if(t==="light"||t==="dark")document.documentElement.setAttribute("data-theme",t)}catch(e){}`;

/**
 * Google Analytics 4 measurement ID.
 *
 * Overridable so that a fork, a preview deployment or a self-hosted copy does not report into someone
 * else's property. The default is the production one.
 */
export const GA_MEASUREMENT_ID = process.env.NEXT_PUBLIC_GA_ID ?? "G-TVB6WV9HVT";

/** Where `gtag.js` is loaded from. Needed in `script-src`. */
export const GA_SCRIPT_ORIGIN = "https://www.googletagmanager.com";

/** Where the reader's analytics choice is stored. Same-origin, first-party, never sent anywhere. */
export const CONSENT_KEY = "guml-consent";

/**
 * The `gtag.js` bootstrap, with Consent Mode v2 **denied before anything else runs**.
 *
 * The ordering is the whole point and it is easy to get wrong. `gtag('consent', 'default', …)` has to
 * execute before `gtag('config', …)`, and both have to be in the page before `gtag.js` finishes
 * loading — otherwise the tag fires once with identifiers already set, and a consent banner shown
 * afterwards is asking permission for something that has already happened.
 *
 * Denied-by-default rather than granted: `analytics_storage: "denied"` means GA4 sets no cookie and no
 * identifier. It still sends a cookieless ping, which is what Consent Mode is for — Google can model
 * aggregate traffic without storing anything on the reader's device, and nothing that could identify
 * them leaves the page. A reader who accepts flips it with `gtag('consent', 'update', …)`, which is
 * what `ConsentBanner` calls.
 *
 * `ad_storage`/`ad_user_data`/`ad_personalization` are denied unconditionally and never updated. This
 * site runs no ads and has no reason to ever grant them, so they are stated rather than left to a
 * default that could change under us.
 */
export const analyticsScript =
  `window.dataLayer = window.dataLayer || [];` +
  `function gtag(){dataLayer.push(arguments);}` +
  `gtag('consent', 'default', {` +
  `'ad_storage':'denied',` +
  `'ad_user_data':'denied',` +
  `'ad_personalization':'denied',` +
  `'analytics_storage':'denied',` +
  `'wait_for_update': 500` +
  `});` +
  // A prior "accept" is restored before the tag configures, so a returning reader is not measured as
  // a denied session for the first half-second of every visit.
  `try{if(localStorage.getItem('${CONSENT_KEY}')==='granted'){` +
  `gtag('consent','update',{'analytics_storage':'granted'});}}catch(e){}` +
  `gtag('js', new Date());` +
  `gtag('config', '${GA_MEASUREMENT_ID}');`;

/**
 * Analytics is production-only.
 *
 * A `pnpm dev` session otherwise reports page views into the same property as real traffic, and
 * every number afterwards is a mix of readers and whoever was editing the site that week. There is no
 * way to separate them retroactively.
 */
export const analyticsEnabled = process.env.NODE_ENV === "production";

/**
 * Kept as the inventory of every inline script on the page, even though the CSP no longer hashes them.
 *
 * Hashing was the first design and it cannot work with the App Router: Next emits the RSC payload as
 * ~30 inline scripts whose contents are the serialised page, so there is no fixed string to hash, and a
 * hash-only policy blocks hydration entirely while the page still *looks* rendered. `next.config.ts`
 * explains the trade-off that replaced it.
 *
 * This list stays because it is the answer to "what inline scripts does this site ship", which is worth
 * being able to answer in one place the next time the policy is revisited.
 */
export const INLINE_SCRIPTS: readonly string[] = [themeScript, analyticsScript];
