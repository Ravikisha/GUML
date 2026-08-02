import { createHmac, randomUUID, timingSafeEqual } from "node:crypto";

/**
 * Demo quota for the generation endpoint.
 *
 * # What this can and cannot do
 *
 * The request was "3 per system, and no bypass via incognito". Those two goals pull apart,
 * so it is worth being exact about which layer stops what:
 *
 * | layer | survives incognito? | survives cookie wipe? | defeated by |
 * |---|---|---|---|
 * | Signed cookie | **no** — a private window has an empty jar | no | opening incognito |
 * | IP counter | **yes** | **yes** | VPN, mobile data, a different network |
 * | Global daily cap | yes | yes | nothing — it is the cost ceiling |
 *
 * So the cookie is not the limit; the **IP counter is**, and the cookie only catches the
 * same browser when its address changes (phone roaming, office → home). A request is
 * refused when *either* counter is spent, which is what makes a private window pointless:
 * it drops the cookie but keeps the address.
 *
 * Deliberately **not** used: canvas/font fingerprinting. It is the usual answer to this
 * question and it does not hold — Safari, Firefox and Brave actively randomise those
 * signals — and it is covert tracking of people who only came to read the docs. The honest
 * airtight version of "3 per person" is a sign-in, which is a bigger decision than a demo
 * needs. The global cap is what actually protects the API bill.
 *
 * # Storage
 *
 * In memory by default, which is correct on one machine and leaky on several: each instance
 * keeps its own tally and a restart forgives everyone. Setting `UPSTASH_REDIS_REST_URL` and
 * `UPSTASH_REDIS_REST_TOKEN` makes it durable and shared, over plain `fetch` against the REST
 * API — no client library, and it works on a runtime that cannot hold a TCP connection.
 *
 * The two paths are not either/or: the in-process counter always runs, and the shared counter
 * *raises* the observed usage when it is higher. A Redis outage therefore degrades to the
 * local limit rather than removing the limit.
 */

export const PER_IDENTITY_LIMIT = 3;

/** The real cost control: no VPN hop can push spend past this. */
export const GLOBAL_DAILY_LIMIT = 120;

const WINDOW_MS = 24 * 60 * 60 * 1000;
const COOKIE = "guml_demo";

type Bucket = { used: number; resetAt: number };

/**
 * In-process counters. Correct on one machine; on several instances each keeps its own tally
 * and a restart forgives everyone.
 *
 * `SHARED` below replaces them when a Redis is configured. It speaks Upstash's REST API over
 * plain `fetch` rather than a client library, so making the limit durable adds a URL and a
 * token — no dependency, and it works on a serverless runtime where a TCP client would not.
 */
const store = {
  byIp: new Map<string, Bucket>(),
  byCookie: new Map<string, Bucket>(),
  global: { used: 0, resetAt: Date.now() + WINDOW_MS } as Bucket,
};

const REDIS_URL = process.env.UPSTASH_REDIS_REST_URL;
const REDIS_TOKEN = process.env.UPSTASH_REDIS_REST_TOKEN;

export const isDurable = Boolean(REDIS_URL && REDIS_TOKEN);

/**
 * `INCR` plus `EXPIRE` on first write, which is the standard fixed-window counter. Returns
 * `null` on any failure so the caller falls back to the in-process count: a Redis outage must
 * not take the demo down, and it must not silently remove the limit either.
 */
async function redisIncr(key: string, ttlSeconds: number): Promise<number | null> {
  if (!isDurable) return null;
  try {
    const res = await fetch(`${REDIS_URL}/pipeline`, {
      method: "POST",
      headers: { Authorization: `Bearer ${REDIS_TOKEN}`, "Content-Type": "application/json" },
      body: JSON.stringify([
        ["INCR", key],
        ["EXPIRE", key, String(ttlSeconds), "NX"],
      ]),
      cache: "no-store",
    });
    if (!res.ok) return null;
    const out = (await res.json()) as Array<{ result?: number }>;
    return typeof out?.[0]?.result === "number" ? out[0].result : null;
  } catch {
    return null;
  }
}

/** Read a counter without incrementing it, for the read-only limit endpoint. */
async function redisGet(key: string): Promise<number | null> {
  if (!isDurable) return null;
  try {
    const res = await fetch(`${REDIS_URL}/get/${encodeURIComponent(key)}`, {
      headers: { Authorization: `Bearer ${REDIS_TOKEN}` },
      cache: "no-store",
    });
    if (!res.ok) return null;
    const out = (await res.json()) as { result?: string | null };
    return out.result == null ? 0 : Number(out.result);
  } catch {
    return null;
  }
}

/** Fixed-window key, so the window rolls without needing a stored reset time. */
function windowKey(kind: string, id: string): string {
  const window = Math.floor(Date.now() / WINDOW_MS);
  return `guml:${kind}:${window}:${id}`;
}

function secret(): string {
  // A dedicated secret if configured; otherwise derived from the API key so the signature
  // is stable per deployment without asking for more configuration. Never the key itself.
  const base = process.env.DEMO_COOKIE_SECRET || process.env.NVIDIA_API_KEY;

  if (!base) {
    // **Fail closed.** This used to fall back to the literal `"guml-demo"`, which is a fine default
    // for `pnpm dev` and a hole in production: the identity cookie is what the per-identity quota is
    // counted against, so a signing key published in a public repository lets anyone mint an
    // identity and spend the demo's daily cap at will. The IP half of the quota would still apply,
    // but the cookie half — the half that survives a changing address — would not.
    //
    // A misconfigured deployment must not silently become an open one, so this throws instead.
    if (process.env.NODE_ENV === "production") {
      throw new Error(
        "DEMO_COOKIE_SECRET is not set. It signs the identity cookie the rate limit is counted " +
          "against; without it the signing key would be a constant compiled into a public repository.",
      );
    }
    // Development only, and only ever reached when neither variable is present.
    return createHmac("sha256", "guml-demo-cookie").update("guml-demo-dev").digest("hex");
  }

  return createHmac("sha256", "guml-demo-cookie").update(base).digest("hex");
}

function sign(value: string): string {
  return createHmac("sha256", secret()).update(value).digest("base64url");
}

/**
 * `id.signature`. The id is opaque and random — no address, no user agent, nothing derived
 * from the visitor — so the cookie identifies a browser to *this* deployment and says
 * nothing about the person to anyone else.
 */
function issueId(): string {
  const id = randomUUID();
  return `${id}.${sign(id)}`;
}

function readId(cookieHeader: string | null): string | null {
  if (!cookieHeader) return null;
  const raw = cookieHeader
    .split(";")
    .map((c) => c.trim())
    .find((c) => c.startsWith(`${COOKIE}=`))
    ?.slice(COOKIE.length + 1);
  if (!raw) return null;

  const [id, signature] = decodeURIComponent(raw).split(".");
  if (!id || !signature) return null;

  // A forged or edited cookie is treated as no cookie, so tampering buys nothing.
  const expected = Buffer.from(sign(id));
  const got = Buffer.from(signature);
  if (expected.length !== got.length || !timingSafeEqual(expected, got)) return null;
  return id;
}

/**
 * The caller's address, as reported by the proxy in front of this app.
 *
 * Only trustworthy behind a proxy that overwrites `x-forwarded-for` (Vercel, Cloudflare,
 * nginx with `proxy_set_header`). Exposed directly to the internet, a client can send any
 * value it likes — which is why the global cap exists and does not depend on this.
 */
function clientIp(headers: Headers): string {
  const forwarded = headers.get("x-forwarded-for");
  const ip = forwarded?.split(",")[0]?.trim() || headers.get("x-real-ip")?.trim();
  return ip || "local";
}

/** Hashed so a request log or a memory dump holds no raw addresses. */
function ipKey(headers: Headers): string {
  return createHmac("sha256", secret()).update(clientIp(headers)).digest("base64url").slice(0, 24);
}

function take(bucket: Bucket | undefined, limit: number): Bucket {
  const now = Date.now();
  if (!bucket || bucket.resetAt <= now) return { used: 0, resetAt: now + WINDOW_MS };
  return bucket.used >= limit ? bucket : bucket;
}

export type Decision = {
  allowed: boolean;
  /** Generations left for this visitor, by the strictest applicable counter. */
  remaining: number;
  limit: number;
  resetAt: number;
  reason?: "identity" | "global";
  /** Present when a new cookie should be set on the response. */
  setCookie?: string;
  /** Opaque handle for `commit`, so a failed generation costs nothing. */
  ticket: { ip: string; id: string };
};

/**
 * Does this request get a generation? Nothing is spent here.
 *
 * Checked before calling the model and committed only once the model actually starts
 * answering — a `DEGRADED` upstream should not burn one of someone's three.
 */
export function check(request: Request): Decision {
  const now = Date.now();
  const ip = ipKey(request.headers);
  const existing = readId(request.headers.get("cookie"));
  const id = existing ?? randomUUID();

  const global = take(store.global, GLOBAL_DAILY_LIMIT);
  store.global = global;

  const byIp = take(store.byIp.get(ip), PER_IDENTITY_LIMIT);
  const byCookie = take(store.byCookie.get(id), PER_IDENTITY_LIMIT);

  // The strictest counter wins: incognito resets the cookie but not the address.
  const used = Math.max(byIp.used, byCookie.used);
  const remaining = Math.max(0, PER_IDENTITY_LIMIT - used);
  const resetAt = Math.max(byIp.resetAt, byCookie.resetAt);

  const setCookie = existing ? undefined : cookieHeader(issueId());
  const ticket = { ip, id };

  if (global.used >= GLOBAL_DAILY_LIMIT) {
    return {
      allowed: false,
      remaining,
      limit: PER_IDENTITY_LIMIT,
      resetAt: global.resetAt,
      reason: "global",
      setCookie,
      ticket,
    };
  }
  if (remaining <= 0) {
    return {
      allowed: false,
      remaining: 0,
      limit: PER_IDENTITY_LIMIT,
      resetAt,
      reason: "identity",
      setCookie,
      ticket,
    };
  }

  return {
    allowed: true,
    remaining,
    limit: PER_IDENTITY_LIMIT,
    resetAt: resetAt || now + WINDOW_MS,
    setCookie,
    ticket,
  };
}

/**
 * The decision, with the shared counters folded in when they are configured.
 *
 * `check` stays synchronous for callers that cannot await; this is the version the routes use.
 * A shared count can only make the answer *stricter*: if Redis says three are spent and this
 * instance's map says none, the visitor is out. The reverse — Redis unreachable — falls back to
 * the local limit rather than removing the limit.
 */
export async function checkShared(request: Request): Promise<Decision> {
  const local = check(request);
  const shared = await sharedUsage(local.ticket);
  if (!shared) return local;

  const remaining = Math.min(local.remaining, Math.max(0, PER_IDENTITY_LIMIT - shared.identity));

  if (shared.global >= GLOBAL_DAILY_LIMIT) {
    return { ...local, allowed: false, remaining, reason: "global" };
  }
  if (remaining <= 0) {
    return { ...local, allowed: false, remaining: 0, reason: "identity" };
  }
  return { ...local, allowed: local.allowed, remaining };
}

/**
 * Spend one generation, durably when a Redis is configured.
 *
 * Async, unlike `check`: the in-process path stays synchronous and the shared path is awaited.
 * The caller spends only after the model has started answering, so the extra round trip is
 * never on the critical path of a refusal.
 */
export async function commitShared(ticket: Decision["ticket"]): Promise<void> {
  commit(ticket);
  if (!isDurable) return;
  const ttl = Math.ceil(WINDOW_MS / 1000);
  await Promise.all([
    redisIncr(windowKey("ip", ticket.ip), ttl),
    redisIncr(windowKey("id", ticket.id), ttl),
    redisIncr(windowKey("global", "all"), ttl),
  ]);
}

/**
 * The durable counts, when configured. `null` means "no shared store, or it failed" and the
 * caller keeps the in-process answer.
 */
export async function sharedUsage(
  ticket: Decision["ticket"],
): Promise<{ identity: number; global: number } | null> {
  if (!isDurable) return null;
  const [ip, id, global] = await Promise.all([
    redisGet(windowKey("ip", ticket.ip)),
    redisGet(windowKey("id", ticket.id)),
    redisGet(windowKey("global", "all")),
  ]);
  if (ip === null || id === null || global === null) return null;
  // The strictest counter wins, exactly as in the local path.
  return { identity: Math.max(ip, id), global };
}

/** Spend one generation. Called once the model has actually started answering. */
export function commit(ticket: Decision["ticket"]): void {
  const now = Date.now();
  const bump = (map: Map<string, Bucket>, key: string) => {
    const b = map.get(key);
    if (!b || b.resetAt <= now) map.set(key, { used: 1, resetAt: now + WINDOW_MS });
    else b.used += 1;
  };
  bump(store.byIp, ticket.ip);
  bump(store.byCookie, ticket.id);

  if (store.global.resetAt <= now) store.global = { used: 1, resetAt: now + WINDOW_MS };
  else store.global.used += 1;
}

function cookieHeader(value: string): string {
  const maxAge = Math.floor(WINDOW_MS / 1000);
  // `HttpOnly` so page scripts cannot read or clear it; `SameSite=Lax` so it survives normal
  // navigation. Not `Secure` on localhost, where the demo is usually run.
  const secure = process.env.NODE_ENV === "production" ? "; Secure" : "";
  return `${COOKIE}=${encodeURIComponent(value)}; Path=/; Max-Age=${maxAge}; HttpOnly; SameSite=Lax${secure}`;
}

/** `X-RateLimit-*`, so a client can show the count without a second request. */
export function limitHeaders(d: Decision): Record<string, string> {
  return {
    "X-RateLimit-Limit": String(d.limit),
    "X-RateLimit-Remaining": String(d.remaining),
    "X-RateLimit-Reset": String(Math.ceil(d.resetAt / 1000)),
  };
}
