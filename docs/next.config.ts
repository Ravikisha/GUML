import { fileURLToPath } from "node:url";
import type { NextConfig } from "next";
import { GA_SCRIPT_ORIGIN } from "./lib/inline-scripts";

/**
 * Google Analytics 4 needs three separate allowances, and missing any one of them fails differently:
 *
 *   * `script-src`  — to load `gtag.js` at all
 *   * `connect-src` — the `fetch`/`sendBeacon` the collector actually uses
 *   * `img-src`     — the pixel fallback, used when a beacon is unavailable (older browsers, and during
 *                     page unload on some platforms). Omitting it loses a fraction of events rather
 *                     than all of them, which is the hardest version of this to notice.
 *
 * The wildcards are Google's own documented regional collector hosts (`region1.google-analytics.com`
 * and similar), not a convenience.
 */
const GA_HOSTS = {
  script: [GA_SCRIPT_ORIGIN],
  connect: [
    "https://www.google-analytics.com",
    "https://*.google-analytics.com",
    "https://*.analytics.google.com",
    GA_SCRIPT_ORIGIN,
  ],
  img: ["https://www.google-analytics.com", "https://*.google-analytics.com", GA_SCRIPT_ORIGIN],
};

/**
 * Security headers.
 *
 * The CSP is the one that took thought, because this site does something most documentation sites do
 * not: the playground compiles GUML in the browser and renders the result. That means two things a
 * default-deny policy would block outright.
 *
 * `'wasm-unsafe-eval'` — the compiler is a WebAssembly module. Instantiating it counts as evaluation
 * under CSP, and this is the *narrow* directive for it: it permits WebAssembly compilation and nothing
 * else, unlike `'unsafe-eval'`, which would also re-enable `eval()` and `new Function()` for every
 * script on the page. Preferring the narrow one is the whole point.
 *
 * `'unsafe-inline'` on `style-src` — Next injects inline styles, and there is no nonce-based path for
 * them that does not require making every page dynamic. Styles cannot execute, so the exposure is
 * defacement rather than code execution. Scripts do *not* get the same treatment.
 *
 * `frame-ancestors 'none'` matters more here than the generic advice suggests: a playground that runs
 * code and a chat endpoint that costs money per call are both worth clickjacking.
 *
 * Note this is the policy for *this site*. `guml capabilities` emits a policy for a *compiled document*,
 * derived from what that document actually does — a different question, and a stricter answer, because
 * a document with no `js` block needs no script evaluation at all.
 *
 * **Why `script-src` allows `'unsafe-inline'`, which is not the obvious choice.**
 *
 * The first attempt hashed each inline script — `'sha256-…'` computed from the exact string the layout
 * renders — on the reasoning that `'unsafe-inline'` gives up most of what a script-src is for. That is
 * sound reasoning and it does not survive contact with the App Router.
 *
 * Next emits the React Server Components payload as roughly thirty inline
 * `self.__next_f.push([1,"…"])` scripts per page, and their contents are the serialised page: different
 * on every route, and regenerated whenever any content changes. There is no fixed string to hash. A
 * hash-only policy therefore blocks the entire hydration payload, and the failure is quiet in the worst
 * way — the HTML still renders, the page still looks finished, and nothing works. No theme toggle, no
 * playground, no chat. It was verified happening here before it was fixed.
 *
 * The two real options are:
 *
 *   1. **A per-request nonce via middleware.** Correct, and Next supports it. The cost is that reading a
 *      nonce forces dynamic rendering on every page, so a documentation site that is almost entirely
 *      static becomes a serverless invocation per page view. Caching the response would defeat it,
 *      because a cached nonce is a shared nonce.
 *   2. **`'unsafe-inline'`**, accepting inline scripts and keeping the rest of the policy tight.
 *
 * Two is chosen here, deliberately, because the pages are static and should stay cacheable. What is
 * given up is narrower than it sounds: `object-src 'none'`, `base-uri 'self'`, `frame-ancestors 'none'`,
 * `form-action 'self'` and a `connect-src` limited to this origin plus the analytics collector all still
 * apply, and those close the injection paths that do not require executing an inline script.
 *
 * **Do not add a hash or a nonce to `script-src` without switching to option one.** Per CSP Level 3, a
 * policy containing any hash or nonce causes `'unsafe-inline'` to be *ignored* — so adding one back as
 * an apparent improvement silently reinstates exactly the breakage described above.
 *
 * `'wasm-unsafe-eval'` stays: the playground instantiates the compiler as WebAssembly, and this is the
 * narrow directive for that, permitting WebAssembly and not `eval()`.
 */
const csp = [
  "default-src 'self'",
  `script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval' ${GA_HOSTS.script.join(" ")}`,
  "style-src 'self' 'unsafe-inline'",
  `img-src 'self' data: blob: ${GA_HOSTS.img.join(" ")}`,
  "font-src 'self' data:",
  // Same-origin, plus the analytics collector. The chat route talks to the upstream model from the
  // server and never from the browser, so no model host belongs here.
  `connect-src 'self' ${GA_HOSTS.connect.join(" ")}`,
  "worker-src 'self' blob:",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'self'",
  "frame-ancestors 'none'",
  "upgrade-insecure-requests",
].join("; ");

const securityHeaders = [
  { key: "Content-Security-Policy", value: csp },
  // Two years, and a prerequisite for preloading. Only set this once the domain is HTTPS-only and you
  // are prepared for that to be true of every subdomain.
  { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "X-Frame-Options", value: "DENY" },
  // Nothing here uses a camera, a microphone or a location, so say so rather than leaving it implicit.
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=(), interest-cohort=()" },
  { key: "Cross-Origin-Opener-Policy", value: "same-origin" },
];

const nextConfig: NextConfig = {
  // This app lives inside the GUML Rust repo, so Turbopack would otherwise walk
  // up and pick a lockfile from a parent directory as the workspace root.
  turbopack: {
    // The docs app lives inside the GUML repo next to the `guml` workspace
    // package, so the root has to include both.
    root: fileURLToPath(new URL("..", import.meta.url)),
  },

  // `guml` ships TypeScript source plus a wasm module, so Next compiles it
  // rather than treating it as a prebuilt dependency.
  transpilePackages: ["@guml/core"],

  // No `typescript`/`eslint` escape hatches here: this Next version dropped both keys from the config
  // type, and type checking is a separate `pnpm typecheck` step in CI rather than something a build
  // flag could quietly disable.

  // The version this deployment documents, so the running site can state it rather than the reader
  // having to guess which release the docs describe.
  env: {
    NEXT_PUBLIC_GUML_VERSION: process.env.NEXT_PUBLIC_GUML_VERSION ?? "0.1.0",
  },

  async headers() {
    return [{ source: "/:path*", headers: securityHeaders }];
  },
};

export default nextConfig;
