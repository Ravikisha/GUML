# Deploying the docs site

## It cannot be a static export

Three API routes exist (`/api/chat`, `/api/chat/limit`, `/api/chat/models`), so `output: "export"` and
plain static hosting are ruled out. It needs a Node runtime.

## The build context is the repository root, not `docs/`

`next.config.ts` sets `turbopack.root` to the parent directory and `transpilePackages: ["guml"]`,
because the site imports the `guml` workspace package — TypeScript source plus a wasm module — rather
than a published build. So `pnpm install` has to run at the root for the workspace link to exist.

`vercel.json` at the repository root encodes exactly that:

```json
{
  "installCommand": "pnpm install --frozen-lockfile",
  "buildCommand": "pnpm --filter docs build",
  "outputDirectory": "docs/.next"
}
```

**Vercel project setting: Root Directory must be `.` (the repository root), not `docs`.** Setting it to
`docs` is the obvious choice and the wrong one — the install would run inside `docs/`, the workspace
link to `guml` would not exist, and the build fails on an import that resolves fine locally.

For any other platform: `output: "standalone"` plus a Dockerfile whose build context is the repository
root.

## Environment variables

All are server-side. None is `NEXT_PUBLIC_`, and none should become one.

| Variable | Required | What it does |
|---|---|---|
| `NVIDIA_API_KEY` | yes, for `/chat` | Upstream model credential. **This is the metered one.** |
| `NVIDIA_BASE_URL` | yes, for `/chat` | Upstream endpoint |
| `NVIDIA_MODEL` | yes, for `/chat` | Model identifier |
| `UPSTASH_REDIS_REST_URL` | strongly recommended | Shared rate-limit counter |
| `UPSTASH_REDIS_REST_TOKEN` | strongly recommended | " |
| `DEMO_COOKIE_SECRET` | **yes in production** | Signs the identity cookie the per-identity quota is counted against |
| `NEXT_PUBLIC_GUML_VERSION` | no | Version the deployment documents; defaults to `0.1.0` |
| `NEXT_PUBLIC_GA_ID` | no | GA4 measurement ID; defaults to `G-TVB6WV9HVT` |

`DEMO_COOKIE_SECRET` **fails closed** if absent in production: the rate limiter throws rather than
signing with a fallback. It used to fall back to the literal string `"guml-demo"`, which is fine for
`pnpm dev` and a hole in production — the signing key would be a constant published in a public
repository, so anyone could mint an identity cookie and spend the daily cap. Generate one with
`openssl rand -hex 32`.

**Note where it fails: at request time, not build time.** The secret is read per request, so a
deployment missing it builds and deploys cleanly and then returns 500 from `/api/chat` on the first
call. Safe, but it will not be caught by CI — which is why the smoke test below includes actually
sending one chat message.

Without Upstash the rate limit degrades to per-instance memory, which on serverless means effectively
no shared limit at all. Set it before the site is public.

## Cost and abuse

`/chat` calls a paid model. Three controls exist, and the first two are in the code:

1. **Per-identity quota**, counted server-side by network address *and* cookie — a private window does
   not reset it.
2. **A global daily cap**, with a graceful "this demo has hit its daily generation cap, it resets
   at …" rather than an error.
3. **A hard spend cap on the upstream key itself.** This one is not in the code and cannot be — set it
   in the provider's console. Treat the first two as the things that keep the bill reasonable and this
   as the thing that keeps it bounded.

`robots.txt` disallows `/api/`, so a crawler will not walk the endpoints. That is politeness, not
enforcement; the quota is the enforcement.

## Analytics

Google Analytics 4, `G-TVB6WV9HVT`, loaded from the root layout. Two things about how it is wired:

**It only loads in production.** `analyticsEnabled` gates on `NODE_ENV`, so a `pnpm dev` session does
not report page views into the same property as real traffic. There is no way to separate those
retroactively, which is why it is a gate rather than a note in a README.

**Override the ID on a fork or a preview.** `NEXT_PUBLIC_GA_ID` exists so that a fork or a staging
deployment does not report into the production property. Set it, or the default applies.

Changing anything about the tag means editing `lib/inline-scripts.ts` — the layout renders from there
and `next.config.ts` reads the origin from there, so the script and the policy that permits it stay in
agreement.

### Consent

This is not wired up, and it is worth being explicit about rather than leaving implied: GA4 sets
identifiers and, for readers in the EU/UK, that ordinarily requires consent before the tag fires under
the ePrivacy Directive and GDPR. There is no consent banner, no `gtag('consent', 'default', …)` call,
and no privacy page on the site.

If the audience includes the EU, the minimum is a consent gate that defaults to denied and a privacy
notice saying what is collected. Google Consent Mode v2 is the mechanism; the tag already loads through
a single module, so it is a contained change.

## Security headers

Set in `next.config.ts` and applied to every route. The CSP is the part worth reading before changing:

- `'wasm-unsafe-eval'` is required — the playground instantiates the compiler as WebAssembly, which
  counts as evaluation. It is the narrow directive: it permits WebAssembly and nothing else, unlike
  `'unsafe-eval'`, which would re-enable `eval()` and `new Function()` for every script on the page.
- **`'unsafe-inline'` is on `script-src`, deliberately, and removing it will break the site.** The App
  Router emits the React Server Components payload as ~30 inline `self.__next_f.push(…)` scripts per
  page, whose contents are the serialised page — different per route, so there is no fixed string to
  hash. A hash-only policy blocks hydration entirely, and does it *quietly*: the HTML renders, the page
  looks finished, and nothing works. That was verified happening here before it was fixed.

  The alternative is a per-request nonce via middleware, which is correct but forces dynamic rendering
  on every page — a documentation site that is almost entirely static becomes a serverless invocation
  per view, and caching the response would defeat the nonce anyway.

  Per CSP Level 3, **any** hash or nonce in the policy makes browsers ignore `'unsafe-inline'`. So
  adding one back as an apparent hardening silently reinstates the breakage. If you want the strict
  policy, switch to the nonce approach in full; there is no half step.
- `'unsafe-inline'` on `style-src` is the same situation and a smaller concern: styles cannot execute,
  so the exposure is defacement rather than code execution.
- `frame-ancestors 'none'` matters more than usual here: a page that runs code and an endpoint that
  costs money per call are both worth clickjacking.

Verify after deploying:

```sh
curl -sD - -o /dev/null https://your-domain/docs/status | grep -i content-security
```

This is the policy for *the site*. `guml capabilities` emits a policy for a *compiled document*, which
is a different and usually stricter question — a document with no `js` block needs no script evaluation
at all.

## Before the first deploy

- [ ] Root Directory is `.`, not `docs`
- [ ] All six environment variables set; `DEMO_COOKIE_SECRET` freshly generated
- [ ] Hard spend cap set on the upstream API key
- [ ] Custom domain and TLS; `metadataBase` in `app/layout.tsx` matches it (currently
      `https://guml.vercel.app`) — and so do the URLs in `app/sitemap.ts` and `app/robots.ts`
- [ ] `NEXT_PUBLIC_GUML_VERSION` set to the released tag
- [ ] `NEXT_PUBLIC_GA_ID` set, or the default property is the intended one
- [ ] Decide the consent question above before the site is public to an EU audience
- [ ] Smoke test: `/`, `/docs`, `/docs/status`, `/research`, `/playground` compiles a fixture, `/chat`
      answers once and then reports its remaining quota
- [ ] **Open the browser console on the deployed site and confirm there are no CSP violations.** The
      theme toggle working and the playground compiling is the practical version of the same check —
      both depend on scripts the policy has to permit

## Notes

`AGENTS.md` and `CLAUDE.md` in this directory are contributor instructions. They are not routed, so
they never render and never reach the built output — but they are also not secret, so nothing depends
on excluding them.
