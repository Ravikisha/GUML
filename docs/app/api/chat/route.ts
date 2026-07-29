import { SYSTEM_PROMPT } from "@/lib/prompt.generated";
import { checkShared, commitShared, limitHeaders, PER_IDENTITY_LIMIT } from "@/lib/rate-limit";
import { DEFAULT_MODEL, NVIDIA_BASE, VERIFIED_MODELS } from "./models";

/**
 * Generation endpoint for the GUML chatbot.
 *
 * The NVIDIA key lives here and only here. This handler is the reason it never reaches the
 * browser: the client posts a conversation, this posts it onward with the credential
 * attached, and streams plain text back. Nothing in `components/chat.tsx` knows the key
 * exists, and no `NEXT_PUBLIC_` variable holds it.
 *
 * The system prompt is generated at build time from `bench/phase0/lib/prompt.mjs` — the
 * same assembly the Phase 0 study measures. A product that prompts differently from the
 * experiment makes the experiment decorative.
 */

// Streams, so it must not be prerendered or cached.
export const dynamic = "force-dynamic";
export const runtime = "nodejs";
/** A large page can take a while on a busy NIM endpoint. */
export const maxDuration = 120;

type ChatMessage = { role: "user" | "assistant"; content: string };

type Body = {
  messages: ChatMessage[];
  model?: string;
  /** The document currently in the editor, so a follow-up can edit rather than restart. */
  current?: string;
};

const MAX_MESSAGES = 24;
const MAX_CHARS = 24_000;

function bad(message: string, status = 400, headers?: Record<string, string>) {
  return Response.json({ error: message }, { status, headers });
}

export async function POST(request: Request) {
  const key = process.env.NVIDIA_API_KEY;
  if (!key) {
    return bad(
      "NVIDIA_API_KEY is not set. Copy docs/.env.example to docs/.env.local and add a key from build.nvidia.com.",
      503,
    );
  }

  let body: Body;
  try {
    body = (await request.json()) as Body;
  } catch {
    return bad("expected a JSON body");
  }

  // The quota is enforced here rather than in the browser, so `curl` and a private window
  // are subject to it too. Nothing is spent yet — see `commit` below.
  const quota = await checkShared(request);
  const quotaHeaders = { ...limitHeaders(quota), ...(quota.setCookie ? { "Set-Cookie": quota.setCookie } : {}) };

  if (!quota.allowed) {
    const resets = new Date(quota.resetAt).toISOString();
    return bad(
      quota.reason === "global"
        ? `This demo has hit its daily generation cap. It resets at ${resets}.`
        : `This demo allows ${PER_IDENTITY_LIMIT} generations. Yours reset at ${resets}. A private window will not help — the limit is counted server-side, by network address as well as by cookie.`,
      429,
      quotaHeaders,
    );
  }

  const messages = Array.isArray(body.messages) ? body.messages : [];
  if (messages.length === 0) return bad("no messages");
  if (messages.length > MAX_MESSAGES) return bad("conversation too long");

  const total = messages.reduce((n, m) => n + (m.content?.length ?? 0), 0);
  if (total > MAX_CHARS) return bad("conversation too large");

  // The current document is context, not history: sending it as its own turn keeps the
  // cacheable prefix stable while still letting "make the button red" mean something.
  const context = body.current?.trim()
    ? [
        {
          role: "user" as const,
          content: `The document currently in the editor:\n\n${body.current.trim()}\n\nApply the next request to it and return the whole document.`,
        },
      ]
    : [];

  const wanted = body.model || process.env.NVIDIA_MODEL || DEFAULT_MODEL;

  // Hosted NIM functions flip in and out of service: a model that answered in 488 ms can
  // return `DEGRADED` a few minutes later. One hard-coded model would therefore be broken
  // for reasons the reader cannot see, so the request walks a chain and reports which model
  // actually answered.
  const chain = [wanted, ...VERIFIED_MODELS.filter((m) => m !== wanted)];
  const failures: string[] = [];

  for (const model of chain) {
    const upstream = await fetch(`${NVIDIA_BASE}/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${key}`,
        Accept: "text/event-stream",
      },
      body: JSON.stringify({
        model,
        messages: [{ role: "system", content: SYSTEM_PROMPT }, ...context, ...messages],
        temperature: 0.2,
        max_tokens: 2048,
        stream: true,
      }),
    }).catch((e: unknown) => e as Error);

    if (upstream instanceof Error) {
      failures.push(`${model}: ${upstream.message}`);
      continue;
    }
    if (!upstream.ok || !upstream.body) {
      failures.push(`${model}: ${await describe(upstream)}`);
      continue;
    }

    // Spent only now: a `DEGRADED` upstream must not cost one of someone's three.
    await commitShared(quota.ticket);

    return new Response(toTextStream(upstream.body), {
      headers: {
        ...quotaHeaders,
        "X-RateLimit-Remaining": String(Math.max(0, quota.remaining - 1)),
        "Content-Type": "text/plain; charset=utf-8",
        "Cache-Control": "no-store",
        "X-Accel-Buffering": "no",
        // Which model produced this, since it may not be the one that was asked for.
        "X-Guml-Model": model,
      },
    });
  }

  const degraded = failures.every((f) => /DEGRADED|has reached end of life|404/i.test(f));
  return bad(
    degraded
      ? `Every model tried is unavailable to this key right now. build.nvidia.com marks a NIM \`DEGRADED\` when the account is out of credits or the function is offline — check the credit balance at build.nvidia.com, then retry. Tried: ${chain.join(", ")}.`
      : `build.nvidia.com would not generate. ${failures.join(" · ")}`,
    502,
    // The attempt failed, so the quota is untouched and the client is told so.
    quotaHeaders,
  );
}

/** Upstream error detail, without echoing the request back to the client. */
async function describe(res: Response): Promise<string> {
  const text = await res.text().catch(() => "");
  try {
    const parsed = JSON.parse(text);
    return `${res.status} ${parsed?.detail ?? parsed?.error?.message ?? parsed?.message ?? ""}`.trim();
  } catch {
    return `${res.status} ${text.slice(0, 200)}`.trim();
  }
}

/**
 * OpenAI-style SSE → plain text deltas.
 *
 * The client gets text rather than re-parsed JSON because everything downstream of here
 * treats the answer as a document: the compiler, the preview and the token counter all take
 * a string. Reasoning traces (some NIM models emit `reasoning_content`) are dropped — they
 * are not part of the artifact and would corrupt the token measurement.
 */
function toTextStream(body: ReadableStream<Uint8Array>): ReadableStream<Uint8Array> {
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  const reader = body.getReader();
  let buffer = "";

  return new ReadableStream({
    async pull(controller) {
      const { value, done } = await reader.read();
      if (done) {
        controller.close();
        return;
      }

      buffer += decoder.decode(value, { stream: true });
      // SSE events are separated by a blank line; a chunk can split one in half.
      const events = buffer.split("\n\n");
      buffer = events.pop() ?? "";

      for (const event of events) {
        for (const line of event.split("\n")) {
          if (!line.startsWith("data:")) continue;
          const payload = line.slice(5).trim();
          if (!payload || payload === "[DONE]") continue;
          try {
            const json = JSON.parse(payload);
            const delta = json?.choices?.[0]?.delta?.content;
            if (typeof delta === "string" && delta) controller.enqueue(encoder.encode(delta));
          } catch {
            // A partial frame: ignore it rather than killing the stream.
          }
        }
      }
    },
    cancel() {
      void reader.cancel();
    },
  });
}
