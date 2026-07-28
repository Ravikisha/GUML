"use client";

import type { Diagnostic } from "guml";
import { applyAllSuggestions, check, format } from "guml";
import { Guml } from "guml/react";
import { AlignLeft, ArrowUp, Eraser, Loader2, Square, Wand2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CLASS_STYLE, highlight } from "@/lib/highlight";
import { SYSTEM_PROMPT_EST_TOKENS } from "@/lib/prompt.generated";
import { cn, commas } from "@/lib/utils";
import { CopyButton } from "./copy-button";
import { MOCK_DATA } from "./live-preview";
import { Badge } from "./ui";

/* --------------------------------------------------------------------------
   A chatbot that generates interfaces rather than describing them.

   The model returns GUML, the compiler turns it into a UI tree in the browser, and the
   preview is rendered from that tree. So the thing on screen is the compiler's actual
   output — not a mock of it, and not a second renderer that could disagree.

   The layout is one column on a phone with a tab switch, and two columns from `lg` up.
   Chat and result want to be side by side when there is room and stacked when there is
   not; nothing here needs a breakpoint beyond that.
   -------------------------------------------------------------------------- */

type Role = "user" | "assistant";
type Message = { role: Role; content: string };

const PROMPTS = [
  "A task list with add, tick off, delete and a filter. It should feel instant and roll back if the server rejects it.",
  "A pricing page: three tiers, the middle one highlighted, four features each, and an FAQ about billing.",
  "A settings screen with a profile card and notification switches that save as soon as they change.",
  "A support dashboard: four KPI tiles and a ticket table I can resolve rows from.",
];

/** Models occasionally wrap output in a fence despite being told not to. Take the code. */
function extractGuml(text: string): string {
  const fenced = /```(?:guml)?\s*\n([\s\S]*?)(?:```|$)/.exec(text);
  const body = fenced ? fenced[1] : text;
  return body.trim();
}

export function Chat() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [source, setSource] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Keyed by the source it came from, so a result that lands after another keystroke is
  // ignored rather than shown against a document it did not describe.
  const [checked, setChecked] = useState<{ src: string; diags: Diagnostic[] }>({
    src: "",
    diags: [],
  });
  const [pane, setPane] = useState<"chat" | "result">("chat");
  const [catalogue, setCatalogue] = useState<{ verified: string[]; slow: string[]; catalogue: string[] }>({
    verified: [],
    slow: [],
    catalogue: [],
  });
  const [model, setModel] = useState<string>("");
  const [configured, setConfigured] = useState<boolean | null>(null);
  /** Set while a request is taking long enough that the reader deserves an explanation. */
  const [slowWarning, setSlowWarning] = useState(false);
  /** Demo quota, read from the server — the browser is not the authority on it. */
  const [quota, setQuota] = useState<{ remaining: number; limit: number; resetAt: number } | null>(
    null,
  );

  const abort = useRef<AbortController | null>(null);
  const log = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let live = true;
    fetch("/api/chat/models")
      .then((r) => r.json())
      .then(
        (data: {
          configured: boolean;
          verified: string[];
          slow: string[];
          catalogue: string[];
          default: string;
        }) => {
          if (!live) return;
          setConfigured(data.configured);
          setCatalogue({
            verified: data.verified ?? [],
            slow: data.slow ?? [],
            catalogue: data.catalogue ?? [],
          });
          setModel(data.default);
        },
      )
      .catch(() => live && setConfigured(false));

    fetch("/api/chat/limit")
      .then((r) => r.json())
      .then((d: { remaining: number; limit: number; resetAt: number }) => live && setQuota(d))
      .catch(() => {
        /* the server enforces it regardless; the counter is a courtesy */
      });

    return () => {
      live = false;
    };
  }, []);

  // Keep the newest turn in view while tokens arrive.
  useEffect(() => {
    log.current?.scrollTo({ top: log.current.scrollHeight, behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    if (!source.trim()) return;
    let live = true;
    check(source).then((r) => {
      if (live) setChecked({ src: source, diags: r.diagnostics });
    });
    return () => {
      live = false;
    };
  }, [source]);

  // Derived, not mirrored: no state to clear, so nothing to clear at the wrong moment.
  const diagnostics = useMemo(
    () => (checked.src === source ? checked.diags : []),
    [checked, source],
  );
  const errors = useMemo(() => diagnostics.filter((d) => d.severity === "error"), [diagnostics]);
  const fixable = useMemo(() => diagnostics.filter((d) => d.suggestion), [diagnostics]);
  const rows = useMemo(() => (source ? highlight(source, "guml") : []), [source]);

  const send = useCallback(
    async (text: string) => {
      const prompt = text.trim();
      if (!prompt || streaming) return;

      const history: Message[] = [...messages, { role: "user", content: prompt }];
      setMessages([...history, { role: "assistant", content: "" }]);
      setInput("");
      setError(null);
      setStreaming(true);
      setPane("result");

      const controller = new AbortController();
      abort.current = controller;
      setSlowWarning(false);
      const slowTimer = setTimeout(() => setSlowWarning(true), 20_000);

      try {
        const res = await fetch("/api/chat", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ messages: history, model, current: source }),
          signal: controller.signal,
        });

        // The server is the authority on what is left; the header is read on every reply,
        // including refusals.
        const left = res.headers.get("X-RateLimit-Remaining");
        const limit = res.headers.get("X-RateLimit-Limit");
        const reset = res.headers.get("X-RateLimit-Reset");
        if (left !== null && limit !== null) {
          setQuota({
            remaining: Number(left),
            limit: Number(limit),
            resetAt: Number(reset ?? 0) * 1000,
          });
        }

        if (!res.ok || !res.body) {
          const detail = await res.json().catch(() => ({ error: `request failed: ${res.status}` }));
          throw new Error(detail.error ?? `request failed: ${res.status}`);
        }

        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let acc = "";
        for (;;) {
          const { value, done } = await reader.read();
          if (done) break;
          acc += decoder.decode(value, { stream: true });
          // The document updates as it streams: the preview builds up line by line, which
          // is the whole argument for a small representation made visible.
          setMessages((prev) => {
            const next = [...prev];
            next[next.length - 1] = { role: "assistant", content: acc };
            return next;
          });
          setSource(extractGuml(acc));
        }

        // Some NIMs answer with reasoning only, or with nothing at all. Silence looks like
        // a bug in this page, so name what happened and point at the fix.
        if (!acc.trim()) {
          setError(
            `${model} returned an empty completion. Some catalogue models accept the request and produce no content — pick one from the verified group.`,
          );
        }
      } catch (e) {
        if ((e as Error).name !== "AbortError") {
          setError(e instanceof Error ? e.message : "something went wrong");
        }
      } finally {
        clearTimeout(slowTimer);
        setSlowWarning(false);
        setStreaming(false);
        abort.current = null;
      }
    },
    [messages, model, source, streaming],
  );

  const estTokens = Math.ceil(source.length / 3.6);
  const spent = quota !== null && quota.remaining <= 0;

  return (
    <div className="mx-auto grid w-full max-w-7xl gap-4 px-4 pb-10 md:px-6 lg:grid-cols-[minmax(0,26rem)_minmax(0,1fr)] lg:gap-6">
      {/* Pane switch: only needed while the two columns are stacked. */}
      <div
        role="tablist"
        aria-label="Chat or result"
        className="flex gap-1 rounded-full border border-line p-1 lg:hidden"
      >
        {(["chat", "result"] as const).map((p) => (
          <button
            key={p}
            role="tab"
            aria-selected={pane === p}
            type="button"
            onClick={() => setPane(p)}
            className={cn(
              "flex-1 rounded-full px-3 py-1.5 font-mono text-xs transition-colors",
              pane === p ? "bg-chalk text-ink" : "text-fog hover:text-chalk",
            )}
          >
            {p === "chat" ? "conversation" : "result"}
          </button>
        ))}
      </div>

      {/* ------------------------------------------------------------- chat */}
      <section
        className={cn(
          "flex min-h-[32rem] flex-col rounded-panel border border-line lg:h-[calc(100vh-11rem)]",
          pane === "chat" ? "flex" : "hidden lg:flex",
        )}
      >
        <header className="flex items-center justify-between gap-3 border-b border-line px-4 py-2.5">
          <span className="flex items-center gap-2">
            <span className="label">describe an interface</span>
            {quota !== null && (
              <span
                className={cn(
                  "rounded-full border px-2 py-0.5 font-mono text-[0.65rem]",
                  spent ? "border-ember/40 text-ember" : "border-line-strong text-fog",
                )}
                title="Demo quota, counted on the server"
              >
                {quota.remaining}/{quota.limit} left
              </span>
            )}
          </span>
          {catalogue.verified.length + catalogue.catalogue.length > 0 ? (
            <label className="flex items-center gap-2">
              <span className="sr-only">Model</span>
              <select
                value={model}
                onChange={(e) => setModel(e.target.value)}
                className="max-w-[13rem] truncate rounded-chip border border-line bg-transparent px-2 py-1 font-mono text-[0.7rem] text-fog"
              >
                {/* Grouped because the account lists ~100 ids and most of them 404 when
                    called. A flat list would be a menu of mostly-broken options. */}
                <optgroup label="verified">
                  {catalogue.verified.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </optgroup>
                {catalogue.slow.length > 0 && (
                  <optgroup label="slow (minutes, not seconds)">
                    {catalogue.slow.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </optgroup>
                )}
                {catalogue.catalogue.length > 0 && (
                  <optgroup label="catalogue — many are unavailable">
                    {catalogue.catalogue.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </optgroup>
                )}
              </select>
            </label>
          ) : null}
        </header>

        <div ref={log} className="flex-1 space-y-4 overflow-y-auto px-4 py-4">
          {configured === false && (
            <div className="rounded-card border border-ember/30 bg-ember/[0.06] p-4 text-sm text-fog">
              <p className="font-mono text-chalk">No API key configured</p>
              <p className="mt-2 leading-relaxed">
                Copy <code className="font-mono text-chalk">docs/.env.example</code> to{" "}
                <code className="font-mono text-chalk">docs/.env.local</code> and set{" "}
                <code className="font-mono text-chalk">NVIDIA_API_KEY</code> to a key from
                build.nvidia.com, then restart the dev server. The key stays on the server.
              </p>
            </div>
          )}

          {messages.length === 0 && configured !== false && !spent && (
            <div className="space-y-3">
              <p className="text-sm leading-relaxed text-fog">
                Ask for an interface. The model answers in GUML, the compiler builds it in your
                browser, and the panel beside this one is the compiler&rsquo;s own output.
              </p>
              <div className="grid gap-2">
                {PROMPTS.map((p) => (
                  <button
                    key={p}
                    type="button"
                    onClick={() => send(p)}
                    className="rounded-card border border-line px-3 py-2.5 text-left text-sm leading-relaxed text-fog transition-colors hover:border-chalk/30 hover:text-chalk"
                  >
                    {p}
                  </button>
                ))}
              </div>
            </div>
          )}

          {messages.map((m, i) => (
            <div key={i} className={cn("flex", m.role === "user" ? "justify-end" : "justify-start")}>
              <div
                className={cn(
                  "max-w-[85%] rounded-card px-3.5 py-2.5 text-sm leading-relaxed",
                  m.role === "user"
                    ? "bg-chalk/10 text-chalk"
                    : "border border-line text-fog",
                )}
              >
                {m.role === "user" ? (
                  m.content
                ) : m.content ? (
                  // The assistant's turn is a document, not prose: showing it as a wall of
                  // text would bury the thing the user asked for.
                  <p className="font-mono text-xs text-fog-dim">
                    {extractGuml(m.content).split("\n").length} lines of GUML →{" "}
                    <span className="text-mint">see the result</span>
                  </p>
                ) : (
                  <span className="inline-flex items-center gap-2 text-fog-dim">
                    <Loader2 className="size-3.5 animate-spin" /> thinking
                  </span>
                )}
              </div>
            </div>
          ))}

          {spent && (
            <div className="rounded-card border border-ember/30 bg-ember/[0.06] p-4 text-sm text-fog">
              <p className="font-mono text-chalk">Demo quota reached</p>
              <p className="mt-2 leading-relaxed">
                This demo allows {quota?.limit} generations per visitor
                {quota?.resetAt ? `, resetting ${new Date(quota.resetAt).toLocaleString()}` : ""}. The
                count is kept on the server against your network address as well as a cookie, so a
                private window will not reset it.
              </p>
              <p className="mt-2 leading-relaxed">
                The <a className="text-iris underline" href="/playground">playground</a> has no limit —
                the compiler runs entirely in your browser. Only generation costs anything.
              </p>
            </div>
          )}

          {slowWarning && streaming && (
            <p role="status" className="rounded-card border border-line px-3 py-2 text-sm text-fog">
              Still waiting on <span className="font-mono text-chalk">{model}</span>. The larger
              hosted models can take minutes to return a first token on a shared endpoint —
              the verified 8B model answers in about a second.
            </p>
          )}

          {error && (
            <p role="alert" className="rounded-card border border-ember/30 bg-ember/[0.06] px-3 py-2 text-sm text-ember">
              {error}
            </p>
          )}
        </div>

        <form
          className="border-t border-line p-3"
          onSubmit={(e) => {
            e.preventDefault();
            send(input);
          }}
        >
          <div className="flex items-end gap-2">
            <label htmlFor="chat-input" className="sr-only">
              Describe the interface you want
            </label>
            <textarea
              id="chat-input"
              rows={2}
              value={input}
              disabled={configured === false || spent}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                // Enter sends, Shift+Enter breaks the line — the convention every chat UI
                // has, and the one people's fingers already know.
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  send(input);
                }
              }}
              placeholder={spent ? "Demo quota reached" : "A task list with optimistic updates…"}
              className="min-h-[3.25rem] flex-1 resize-none rounded-card border border-line bg-transparent px-3 py-2 text-sm text-chalk outline-none placeholder:text-fog-dim focus:border-chalk/30 disabled:opacity-40"
            />
            {streaming ? (
              <button
                type="button"
                onClick={() => abort.current?.abort()}
                aria-label="Stop generating"
                className="inline-flex size-10 shrink-0 items-center justify-center rounded-full border border-line text-fog transition-colors hover:border-chalk/30 hover:text-chalk"
              >
                <Square className="size-3.5" />
              </button>
            ) : (
              <button
                type="submit"
                disabled={!input.trim() || configured === false || spent}
                aria-label="Send"
                className="inline-flex size-10 shrink-0 items-center justify-center rounded-full bg-chalk text-ink transition-opacity hover:opacity-90 disabled:opacity-30"
              >
                <ArrowUp className="size-4" />
              </button>
            )}
          </div>
          <p className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-[0.68rem] text-fog-dim">
            <span>enter sends · shift+enter newline</span>
            <span aria-hidden>·</span>
            <span>~{commas(SYSTEM_PROMPT_EST_TOKENS)} token spec, cached per request</span>
            {quota !== null && (
              <>
                <span aria-hidden>·</span>
                <span>{quota.limit}-generation demo limit</span>
              </>
            )}
          </p>
        </form>
      </section>

      {/* ----------------------------------------------------------- result */}
      <section
        className={cn(
          "flex min-h-[32rem] flex-col gap-4 lg:h-[calc(100vh-11rem)]",
          pane === "result" ? "flex" : "hidden lg:flex",
        )}
      >
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-panel border border-line">
          <header className="flex items-center justify-between gap-3 border-b border-line px-4 py-2.5">
            <span className="label">rendered from the compiler&rsquo;s UI tree</span>
            <span className="flex items-center gap-2">
              {streaming && <Loader2 className="size-3.5 animate-spin text-fog-dim" />}
              {errors.length > 0 ? (
                <Badge tone="ember">{errors.length} errors</Badge>
              ) : source ? (
                <Badge tone="mint">compiles</Badge>
              ) : null}
            </span>
          </header>
          <div className="min-h-0 flex-1 overflow-auto bg-white p-5">
            {source ? (
              errors.length > 0 ? (
                <ul className="space-y-1 font-mono text-xs text-red-600">
                  {errors.map((d, i) => (
                    <li key={i}>
                      {d.id} line {d.span.line}: {d.message}
                    </li>
                  ))}
                </ul>
              ) : (
                <Guml source={source} data={MOCK_DATA} />
              )
            ) : (
              <p className="text-sm text-slate-400">The generated interface appears here.</p>
            )}
          </div>
        </div>

        <div className="flex min-h-0 flex-[1.1] flex-col overflow-hidden rounded-panel border border-line bg-code code-surface">
          <header className="flex flex-wrap items-center justify-between gap-2 border-b border-line px-4 py-2">
            <span className="label">
              {source ? `${source.split("\n").length} lines · ~${commas(estTokens)} tokens` : "guml"}
            </span>
            <span className="flex items-center gap-1.5">
              {fixable.length > 0 && (
                <button
                  type="button"
                  onClick={() => setSource((s) => applyAllSuggestions(s, diagnostics))}
                  className="inline-flex items-center gap-1.5 rounded-full border border-line px-2.5 py-1 font-mono text-[0.68rem] text-fog transition-colors hover:border-chalk/30 hover:text-chalk"
                >
                  <Wand2 className="size-3" /> apply {fixable.length}
                </button>
              )}
              <button
                type="button"
                disabled={!source}
                onClick={async () => setSource((await format(source)).text)}
                className="inline-flex items-center gap-1.5 rounded-full border border-line px-2.5 py-1 font-mono text-[0.68rem] text-fog transition-colors hover:border-chalk/30 hover:text-chalk disabled:opacity-30"
              >
                <AlignLeft className="size-3" /> format
              </button>
              <button
                type="button"
                disabled={!source && messages.length === 0}
                onClick={() => {
                  setMessages([]);
                  setSource("");
                  setError(null);
                }}
                className="inline-flex items-center gap-1.5 rounded-full border border-line px-2.5 py-1 font-mono text-[0.68rem] text-fog transition-colors hover:border-chalk/30 hover:text-chalk disabled:opacity-30"
              >
                <Eraser className="size-3" /> clear
              </button>
              <CopyButton text={source} />
            </span>
          </header>
          <div className="min-h-0 flex-1 overflow-auto">
            <pre className="w-max min-w-full px-4 py-3 font-mono text-[0.8rem] leading-[1.6]">
              <code>
                {rows.length === 0 ? (
                  <span className="text-fog-dim">
                    {streaming ? "generating…" : "// the model's answer lands here"}
                  </span>
                ) : (
                  rows.map((row, i) => (
                    <span key={i} className="block">
                      {row.length === 0 ? (
                        <span> </span>
                      ) : (
                        row.map((tok, j) => (
                          <span key={j} className={CLASS_STYLE[tok.cls]}>
                            {tok.text}
                          </span>
                        ))
                      )}
                    </span>
                  ))
                )}
              </code>
            </pre>
          </div>
        </div>
      </section>
    </div>
  );
}
