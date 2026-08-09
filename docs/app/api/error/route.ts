import { NextResponse } from "next/server";

/**
 * Receives client-side error reports.
 *
 * A structured line on stderr, which the hosting platform already aggregates and makes searchable.
 * That is deliberately the whole implementation: pointing it at Sentry or a log drain later is a
 * change to the one `console.error` below, and until someone is actually watching a dashboard, adding
 * a vendor would be adding a dependency, a CSP entry and a privacy-page line for no gain.
 *
 * # It is a public unauthenticated endpoint, so it is written like one
 *
 * Anyone can POST here. The controls are proportionate to what that is worth abusing:
 *
 * * **A size cap.** A stack trace is small; a body that is not is not a stack trace.
 * * **Field truncation.** What is logged is bounded regardless of what was sent, so this cannot be
 *   used to write arbitrary volumes into our logs.
 * * **Always 204.** The response says nothing about whether it was accepted, stored or dropped —
 *   there is nothing to learn by probing it.
 *
 * Not rate-limited by IP, unlike `/api/chat`: this costs a log line rather than a model call, and the
 * size cap already bounds the damage. If that stops being true, the limiter in `lib/rate-limit.ts` is
 * the thing to reach for.
 */

export const runtime = "nodejs";

/** Comfortably larger than a truncated stack, far smaller than anything worth logging. */
const MAX_BODY_BYTES = 16 * 1024;

const clamp = (value: unknown, max: number): string | undefined =>
  typeof value === "string" && value.length > 0 ? value.slice(0, max) : undefined;

export async function POST(request: Request) {
  // Reported as accepted either way. A client that cannot report an error must not then have to
  // handle an error from the reporter.
  const ok = () => new NextResponse(null, { status: 204 });

  const length = Number(request.headers.get("content-length") ?? 0);
  if (length > MAX_BODY_BYTES) return ok();

  let payload: unknown;
  try {
    const text = await request.text();
    if (text.length > MAX_BODY_BYTES) return ok();
    payload = JSON.parse(text);
  } catch {
    return ok();
  }

  if (typeof payload !== "object" || payload === null) return ok();
  const r = payload as Record<string, unknown>;

  console.error(
    JSON.stringify({
      level: "error",
      source: "client",
      boundary: clamp(r.boundary, 16),
      message: clamp(r.message, 500),
      digest: clamp(r.digest, 64),
      url: clamp(r.url, 500),
      userAgent: clamp(r.userAgent, 300),
      stack: clamp(r.stack, 4000),
      at: new Date().toISOString(),
    }),
  );

  return ok();
}
