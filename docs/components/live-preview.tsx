"use client";

import { Guml, useGumlTree } from "guml/react";
import type { Diagnostic } from "guml";
import { Loader2 } from "lucide-react";
import { useState } from "react";
import { cn } from "@/lib/utils";

/** Mock rows so a `data` resource has something to render without a server. */
export const MOCK_DATA = {
  tasks: [
    { id: "1", title: "Freeze the v0.1 spec", done: true },
    { id: "2", title: "Write ten Phase 0 specs", done: false },
    { id: "3", title: "Count tokens with the real tokenizer", done: false },
  ],
};

/**
 * Renders GUML with the real compiler, in the browser.
 *
 * The preview is rendered from the compiler's own UI tree, so the classes here are
 * byte-identical to what `guml build` writes — a preview that could drift from the
 * emitted code would be the most misleading thing this site could show.
 *
 * The surface is white on purpose: the compiler's default design system targets a
 * light app, and showing it on the site's dark ink would misrepresent the output.
 */
export function LivePreview({
  source,
  className,
  data = MOCK_DATA,
  label = "live · compiled in your browser",
}: {
  source: string;
  className?: string;
  data?: Record<string, unknown[]>;
  label?: string;
}) {
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);
  const errors = diagnostics.filter((d) => d.severity === "error");

  return (
    <div className={cn("overflow-hidden rounded-card border border-white/8", className)}>
      <div className="flex items-center justify-between gap-3 border-b border-white/8 bg-white/[0.02] px-4 py-2">
        <span className="label">{label}</span>
        {errors.length > 0 ? (
          <span className="font-mono text-[0.7rem] text-ember">
            {errors.length} error{errors.length > 1 ? "s" : ""}
          </span>
        ) : diagnostics.length > 0 ? (
          <span className="font-mono text-[0.7rem] text-fog-dim">
            {diagnostics.length} warning{diagnostics.length > 1 ? "s" : ""}
          </span>
        ) : null}
      </div>

      <div className="bg-white p-5">
        <Guml
          source={source}
          data={data}
          onDiagnostics={setDiagnostics}
          fallback={
            <div className="flex items-center gap-2 py-8 text-sm text-slate-400">
              <Loader2 className="size-3.5 animate-spin" />
              loading the compiler…
            </div>
          }
        />
        {errors.length > 0 && (
          <ul className="space-y-1 font-mono text-xs text-red-600">
            {errors.map((d, i) => (
              <li key={i}>
                {d.id} line {d.span.line}: {d.message}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

/** Compiler-version readout, so a reader knows what produced the preview. */
export function CompilerBadge({ source }: { source: string }) {
  const { status, diagnostics } = useGumlTree(source);
  return (
    <span className="font-mono text-[0.7rem] text-fog-dim">
      {status === "loading"
        ? "compiling…"
        : status === "invalid"
          ? `${diagnostics.filter((d) => d.severity === "error").length} errors`
          : "compiled"}
    </span>
  );
}
