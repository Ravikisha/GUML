"use client";

import type { Diagnostic } from "guml";
import { applyAllSuggestions, check, compile } from "guml";
import { Guml } from "guml/react";
import { Check, Loader2, Play, RotateCcw, Wand2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { highlight } from "@/lib/highlight";
import { cn, commas } from "@/lib/utils";
import { CopyButton } from "./copy-button";
import { MOCK_DATA } from "./live-preview";
import { Badge } from "./ui";

export type Sample = { id: string; label: string; source: string };

/**
 * A GUML playground: edit on the left, real compiler output on the right.
 *
 * Everything here runs in the browser through the wasm build of the same Rust
 * compiler the CLI uses — the diagnostics are the real ones, and the emitted React
 * is byte-for-byte what `guml build` would write.
 */
export function Playground({ samples }: { samples: Sample[] }) {
  const [source, setSource] = useState(samples[0].source);
  const [active, setActive] = useState(samples[0].id);
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);
  const [emitted, setEmitted] = useState<string>("");
  const [pending, setPending] = useState(true);
  const [pane, setPane] = useState<"preview" | "react" | "tree">("preview");
  const [tree, setTree] = useState<string>("");
  const editor = useRef<HTMLTextAreaElement>(null);

  // Recompile on a short debounce: fast enough to feel live, slow enough not to
  // recompile mid-keystroke.
  useEffect(() => {
    let cancelled = false;
    // The pending flag is set inside the timer rather than in the effect body:
    // a synchronous setState in an effect triggers a cascading render, and React
    // 19's lint rule is right to flag it.
    const timer = setTimeout(() => {
      setPending(true);
      void Promise.all([check(source), compile(source, "react"), compile(source, "json")])
        .then(([checked, react, json]) => {
          if (cancelled) return;
          setDiagnostics(checked.diagnostics);
          setEmitted(react.files[0]?.contents ?? "");
          setTree(json.files[0]?.contents ?? "");
        })
        .catch(() => {})
        .finally(() => !cancelled && setPending(false));
    }, 180);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [source]);

  const errors = diagnostics.filter((d) => d.severity === "error");
  const warnings = diagnostics.filter((d) => d.severity === "warning");
  const fixable = diagnostics.filter((d) => d.suggestion).length;

  const load = useCallback((sample: Sample) => {
    setActive(sample.id);
    setSource(sample.source);
  }, []);

  const lines = useMemo(() => highlight(source, "guml"), [source]);

  // Jump the caret to a diagnostic's span, so a click on an error lands on it.
  const goTo = useCallback((d: Diagnostic) => {
    const el = editor.current;
    if (!el) return;
    el.focus();
    el.setSelectionRange(d.span.start, Math.max(d.span.end, d.span.start + 1));
  }, []);

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      {/* ---------------------------------------------------------- editor */}
      <div className="flex min-w-0 flex-col gap-3">
        <div className="flex flex-wrap items-center gap-1.5">
          {samples.map((s) => (
            <button
              key={s.id}
              type="button"
              onClick={() => load(s)}
              className={cn(
                "rounded-full border px-3 py-1.5 font-mono text-xs transition-colors",
                active === s.id
                  ? "border-white/25 bg-white/10 text-chalk"
                  : "border-white/8 text-fog hover:border-white/20 hover:text-chalk",
              )}
            >
              {s.label}
            </button>
          ))}
          <button
            type="button"
            onClick={() => load(samples.find((s) => s.id === active) ?? samples[0])}
            className="ml-auto inline-flex items-center gap-1.5 rounded-full border border-white/8 px-3 py-1.5 font-mono text-xs text-fog transition-colors hover:border-white/20 hover:text-chalk"
          >
            <RotateCcw className="size-3" /> reset
          </button>
        </div>

        <div className="relative overflow-hidden rounded-panel border border-white/8 bg-[#06060a]">
          <div className="flex items-center justify-between border-b border-white/8 px-4 py-2">
            <span className="label">
              {source.split("\n").length} lines · ~{commas(Math.ceil(source.length / 3.6))} tokens
            </span>
            <div className="flex items-center gap-2">
              {pending ? <Loader2 className="size-3 animate-spin text-fog-dim" /> : null}
              <CopyButton text={source} />
            </div>
          </div>

          {/* A highlighted layer under a transparent textarea: real editing
              behaviour (selection, undo, IME) with GUML's own colours. */}
          <div className="relative min-h-[26rem] font-mono text-[0.82rem] leading-[1.65]">
            <pre aria-hidden className="pointer-events-none absolute inset-0 overflow-auto px-4 py-4">
              <code>
                {lines.map((row, i) => (
                  <span key={i} className="block">
                    {row.length === 0 ? (
                      <span> </span>
                    ) : (
                      row.map((tok, j) => (
                        <span key={j} className={tok.cls}>
                          {tok.text}
                        </span>
                      ))
                    )}
                  </span>
                ))}
              </code>
            </pre>
            <textarea
              ref={editor}
              value={source}
              onChange={(e) => setSource(e.target.value)}
              spellCheck={false}
              aria-label="GUML source"
              className="absolute inset-0 h-full w-full resize-none overflow-auto bg-transparent px-4 py-4 font-mono text-[0.82rem] leading-[1.65] text-transparent caret-ember outline-none"
            />
          </div>
        </div>

        {/* ------------------------------------------------------ diagnostics */}
        <div className="rounded-card border border-white/8">
          <div className="flex items-center justify-between border-b border-white/8 px-4 py-2">
            <span className="label">diagnostics</span>
            <div className="flex items-center gap-2">
              {errors.length === 0 && warnings.length === 0 && !pending ? (
                <span className="inline-flex items-center gap-1.5 font-mono text-[0.7rem] text-mint">
                  <Check className="size-3" /> clean
                </span>
              ) : null}
              {errors.length ? <Badge tone="ember">{errors.length} error</Badge> : null}
              {warnings.length ? <Badge>{warnings.length} warning</Badge> : null}
              {fixable > 0 ? (
                <button
                  type="button"
                  onClick={() => setSource((s) => applyAllSuggestions(s, diagnostics))}
                  className="inline-flex items-center gap-1.5 rounded-full border border-mint/30 bg-mint/10 px-2.5 py-1 font-mono text-[0.7rem] text-mint"
                >
                  <Wand2 className="size-3" /> apply {fixable} fix{fixable > 1 ? "es" : ""}
                </button>
              ) : null}
            </div>
          </div>
          <div className="max-h-44 overflow-y-auto p-2">
            {diagnostics.length === 0 ? (
              <p className="px-2 py-3 font-mono text-xs text-fog-dim">
                No problems. Try deleting a label, or renaming a state, to see the compiler answer.
              </p>
            ) : (
              <ul className="space-y-1">
                {diagnostics.map((d, i) => (
                  <li key={i}>
                    <button
                      type="button"
                      onClick={() => goTo(d)}
                      className="w-full rounded-chip px-2 py-1.5 text-left transition-colors hover:bg-white/5"
                    >
                      <span className="font-mono text-xs">
                        <span className={d.severity === "error" ? "text-ember" : "text-fog-dim"}>
                          {d.id}
                        </span>{" "}
                        <span className="text-fog-dim">
                          {d.span.line}:{d.span.col}
                        </span>{" "}
                        <span className="text-chalk">{d.message}</span>
                      </span>
                      {d.help ? (
                        <span className="mt-0.5 block font-mono text-[0.7rem] text-fog-dim">
                          = {d.help}
                        </span>
                      ) : null}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </div>

      {/* ----------------------------------------------------------- output */}
      <div className="flex min-w-0 flex-col gap-3">
        <div className="flex items-center gap-1.5">
          {(
            [
              ["preview", "preview"],
              ["react", "emitted React"],
              ["tree", "UI tree"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              onClick={() => setPane(id)}
              className={cn(
                "rounded-full border px-3 py-1.5 font-mono text-xs transition-colors",
                pane === id
                  ? "border-white/25 bg-white/10 text-chalk"
                  : "border-white/8 text-fog hover:border-white/20 hover:text-chalk",
              )}
            >
              {label}
            </button>
          ))}
          <span className="ml-auto font-mono text-[0.7rem] text-fog-dim">
            {pane === "react" && emitted
              ? `~${commas(Math.ceil(emitted.length / 3.6))} tokens out`
              : "wasm · in-browser"}
          </span>
        </div>

        {pane === "preview" ? (
          <div className="overflow-hidden rounded-panel border border-white/8">
            <div className="border-b border-white/8 px-4 py-2">
              <span className="label">rendered from the compiler&rsquo;s UI tree</span>
            </div>
            <div className="min-h-[26rem] overflow-auto bg-white p-6">
              {errors.length > 0 ? (
                <p className="font-mono text-sm text-red-600">
                  {errors.length} error{errors.length > 1 ? "s" : ""} — fix them to see the preview.
                </p>
              ) : (
                <Guml
                  source={source}
                  data={MOCK_DATA}
                  fallback={
                    <div className="flex items-center gap-2 text-sm text-slate-400">
                      <Loader2 className="size-3.5 animate-spin" /> loading the compiler…
                    </div>
                  }
                />
              )}
            </div>
          </div>
        ) : (
          <div className="overflow-hidden rounded-panel border border-white/8 bg-[#06060a]">
            <div className="flex items-center justify-between border-b border-white/8 px-4 py-2">
              <span className="label">
                {pane === "react" ? "Counter.tsx — real compiler output" : "ui.json"}
              </span>
              <CopyButton text={pane === "react" ? emitted : tree} />
            </div>
            <pre className="max-h-[26rem] overflow-auto px-4 py-4 font-mono text-[0.78rem] leading-[1.6]">
              <code>
                {(pane === "react" ? emitted : tree) ||
                  (errors.length ? "// fix the errors to see output" : "// compiling…")}
              </code>
            </pre>
          </div>
        )}

        <p className="font-mono text-[0.7rem] leading-relaxed text-fog-dim">
          <Play className="mr-1 inline size-3" />
          Token figures here are the ~3.6 chars/token estimate the CLI prints. The measured
          numbers elsewhere on this site were counted with a real tokenizer.
        </p>
      </div>
    </div>
  );
}
