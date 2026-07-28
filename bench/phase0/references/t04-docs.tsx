export default function CachingHeadersDoc() {
  return (
    <article className="mx-auto max-w-2xl px-6 py-12 text-slate-700">
      <h1 className="text-3xl font-semibold tracking-tight text-slate-900">
        HTTP caching headers
      </h1>
      <p className="mt-5 leading-relaxed">
        Caching is the cheapest performance work available to a web application, and the headers
        that drive it are a small, stable vocabulary. This page covers the four things worth
        knowing: how <code>Cache-Control</code> expresses a policy, how validators let a client
        re-use a stale copy without downloading it again, why <code>Vary</code> quietly multiplies
        your cache entries, and what to check when a response is not being cached.
      </p>

      <h2 className="mt-10 text-xl font-semibold text-slate-900">Cache-Control</h2>
      <p className="mt-4 leading-relaxed">
        <code>Cache-Control</code> is a list of directives that applies to both shared caches (a
        CDN) and private ones (the browser). The ones that matter most in practice are{" "}
        <code>max-age</code>, which gives a freshness lifetime in seconds; <code>s-maxage</code>,
        which overrides it for shared caches only; <code>private</code>, which forbids a shared
        cache from storing the response at all; and <code>immutable</code>, which tells the client
        not to revalidate even on a reload.
      </p>
      <p className="mt-4 leading-relaxed">
        The pairing that does the most work is a long <code>max-age</code> on fingerprinted asset
        URLs plus a short one on the HTML that references them. The HTML stays close to fresh, and
        the assets — whose URL changes whenever their content does — never need revalidating.{" "}
        <code>stale-while-revalidate</code> extends this: the cache may serve a stale copy
        immediately while it refreshes in the background, which trades a small amount of staleness
        for a large reduction in latency.
      </p>

      <h2 className="mt-10 text-xl font-semibold text-slate-900">
        ETag and conditional requests
      </h2>
      <p className="mt-4 leading-relaxed">
        Freshness lifetimes eventually expire, and a validator is what makes the follow-up request
        cheap. The server sends an <code>ETag</code> — an opaque token identifying that exact
        response body. When the cached copy goes stale, the client repeats the request with{" "}
        <code>If-None-Match</code> set to the token it holds.
      </p>
      <p className="mt-4 leading-relaxed">
        If the token still matches, the server replies <code>304 Not Modified</code> with no body,
        and the client keeps what it already had. That turns a full download into a round trip of
        headers. <code>Last-Modified</code> with <code>If-Modified-Since</code> does the same job
        at one-second granularity; prefer <code>ETag</code> when a resource can change more than
        once a second, or when its bytes can change without its timestamp doing so.
      </p>

      <h2 className="mt-10 text-xl font-semibold text-slate-900">Vary</h2>
      <p className="mt-4 leading-relaxed">
        A cache key is not just the URL. <code>Vary</code> names the request headers that were part
        of the decision about what to send, and a shared cache must store a separate entry per
        distinct combination of those headers. <code>Vary: Accept-Encoding</code> is nearly always
        correct, because a gzip and a brotli copy of the same resource are different bytes.
      </p>
      <p className="mt-4 leading-relaxed">
        The failure mode is over-varying. <code>Vary: User-Agent</code> effectively disables shared
        caching, since the header has a near-unbounded value space and each variant gets its own
        entry. Vary on the smallest set of headers that genuinely changes the response, and if
        content differs per user, prefer <code>Cache-Control: private</code> over trying to encode
        identity into the cache key.
      </p>

      <div className="mt-8 rounded-xl border border-amber-300 bg-amber-50 p-5">
        <p className="font-medium text-amber-900">no-cache does not mean no store</p>
        <p className="mt-2 text-sm leading-relaxed text-amber-900">
          <code>no-cache</code> permits a cache to store the response, but requires it to
          revalidate with the origin before re-using it. The directive that forbids storage
          entirely is <code>no-store</code>. If you are protecting sensitive data, only{" "}
          <code>no-store</code> does what you mean.
        </p>
      </div>

      <h2 className="mt-10 text-xl font-semibold text-slate-900">Troubleshooting</h2>
      <ul className="mt-4 space-y-2">
        {[
          "A Set-Cookie on the response — many shared caches refuse to store it.",
          "Authorization on the request, which stops a shared cache without an explicit public directive.",
          "A Vary header naming something high-cardinality, so your hit rate collapses to near zero.",
          "A request method other than GET or HEAD, which is not cacheable without explicit freshness information.",
          "A hard reload in devtools, which sends no-cache and hides the behaviour you are trying to observe.",
        ].map((item) => (
          <li key={item} className="flex gap-2 leading-relaxed">
            <span aria-hidden className="mt-2 h-1 w-1 shrink-0 rounded-full bg-slate-400" />
            {item}
          </li>
        ))}
      </ul>

      <footer className="mt-12 border-t border-slate-200 pt-6 text-sm">
        <p>Normative references:</p>
        <ul className="mt-2 space-y-1">
          <li>
            <a
              className="text-blue-700 underline"
              href="https://www.rfc-editor.org/rfc/rfc9111"
              target="_blank"
              rel="noreferrer"
            >
              RFC 9111 — HTTP Caching
            </a>
          </li>
          <li>
            <a
              className="text-blue-700 underline"
              href="https://www.rfc-editor.org/rfc/rfc9110"
              target="_blank"
              rel="noreferrer"
            >
              RFC 9110 — HTTP Semantics
            </a>
          </li>
        </ul>
      </footer>
    </article>
  );
}
