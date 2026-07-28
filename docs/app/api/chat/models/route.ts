import { DEFAULT_MODEL, NVIDIA_BASE, SLOW_MODELS, VERIFIED_MODELS } from "../models";

/**
 * What this account can reach, split by whether it is known to work.
 *
 * `GET /v1/models` returns everything the key can *see* — 102 ids on a free tier, of which
 * most answer 404, 400 or `DEGRADED` when actually invoked. Presenting that list flat would
 * be a picker where nearly every option fails, so verified ids come back separately and the
 * rest are labelled as a catalogue.
 */
export const dynamic = "force-dynamic";
export const runtime = "nodejs";

type ModelsResponse = { data?: Array<{ id?: string }> };

export type ModelsPayload = {
  configured: boolean;
  /** Measured: callable and fast. */
  verified: string[];
  /** Callable but slow enough to feel broken. */
  slow: string[];
  /** Everything else the key lists. Many of these will not answer. */
  catalogue: string[];
  default: string;
  note?: string;
};

function payload(over: Partial<ModelsPayload>): Response {
  return Response.json({
    configured: true,
    verified: VERIFIED_MODELS,
    slow: SLOW_MODELS,
    catalogue: [],
    default: DEFAULT_MODEL,
    ...over,
  } satisfies ModelsPayload);
}

export async function GET() {
  const key = process.env.NVIDIA_API_KEY;
  if (!key) {
    return Response.json({
      configured: false,
      verified: [],
      slow: [],
      catalogue: [],
      default: DEFAULT_MODEL,
    } satisfies ModelsPayload);
  }

  try {
    const res = await fetch(`${NVIDIA_BASE}/models`, {
      headers: { Authorization: `Bearer ${key}` },
      cache: "no-store",
    });
    if (!res.ok) {
      return payload({ note: `could not list models (${res.status})` });
    }

    const json = (await res.json()) as ModelsResponse;
    const all = (json.data ?? []).map((m) => m.id).filter((id): id is string => Boolean(id));
    const known = new Set([...VERIFIED_MODELS, ...SLOW_MODELS]);

    return payload({
      verified: VERIFIED_MODELS.filter((id) => all.includes(id)),
      slow: SLOW_MODELS.filter((id) => all.includes(id)),
      catalogue: all.filter((id) => !known.has(id)).sort(),
      // Never default to something the account cannot see.
      default: all.includes(DEFAULT_MODEL) ? DEFAULT_MODEL : (all[0] ?? DEFAULT_MODEL),
    });
  } catch (e) {
    return payload({ note: e instanceof Error ? e.message : "could not list models" });
  }
}
