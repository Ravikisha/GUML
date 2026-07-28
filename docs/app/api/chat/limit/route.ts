import { check, GLOBAL_DAILY_LIMIT, limitHeaders, PER_IDENTITY_LIMIT } from "@/lib/rate-limit";

/**
 * How many generations this visitor has left.
 *
 * Read-only: `check` reserves nothing, so polling this never costs anyone a generation. It
 * exists so the page can show "2 left" before the first send, instead of discovering the
 * limit by hitting it.
 *
 * It does issue the cookie on a first visit, which is the point at which a browser becomes
 * identifiable to this deployment at all.
 */
export const dynamic = "force-dynamic";
export const runtime = "nodejs";

export async function GET(request: Request) {
  const d = check(request);
  return Response.json(
    {
      remaining: d.remaining,
      limit: PER_IDENTITY_LIMIT,
      globalLimit: GLOBAL_DAILY_LIMIT,
      resetAt: d.resetAt,
      blocked: !d.allowed,
      reason: d.reason ?? null,
    },
    {
      headers: {
        ...limitHeaders(d),
        ...(d.setCookie ? { "Set-Cookie": d.setCookie } : {}),
        "Cache-Control": "no-store",
      },
    },
  );
}
