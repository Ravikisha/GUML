"use client";

import { Code2, Play } from "lucide-react";
import { useState } from "react";
import { CodeBlock } from "./code-block";
import { LivePreview } from "./live-preview";
import { cn } from "@/lib/utils";
import type { Lang } from "@guml/highlight";

/**
 * A GUML sample and what it renders, switchable.
 *
 * Why a toggle and not both stacked: a docs page is read top to bottom, and putting a rendered app
 * under every sample doubles the vertical distance between a paragraph and the one after it. The
 * source stays the default view, so the page reads exactly as it did before, and the preview is one
 * click away for the reader who wants proof rather than a promise.
 *
 * The preview is the real compiler — the same wasm build as the playground — so it cannot drift from
 * what `guml build` emits. That is the whole reason this exists: a hand-made screenshot of a
 * compiler's output is a claim about the compiler, and this is the compiler.
 *
 * Deliberately opt-in per sample. Plenty of docs samples are fragments, shell commands, or
 * deliberately broken code used to show a diagnostic; rendering those would either fail or show
 * something that misrepresents the point being made.
 */
export function CodePreview({
  code,
  lang = "guml",
  filename,
  meter,
  lines,
  data,
  className,
  maxHeight,
  scaffold,
  /** Start on the preview instead of the source. For pages whose subject *is* the output. */
  defaultTab = "source",
}: {
  code: string;
  lang?: Lang;
  filename?: string;
  meter?: string;
  lines?: boolean;
  data?: Record<string, unknown[]>;
  className?: string;
  maxHeight?: number;
  /**
   * Declarations prepended *for the preview only*. Most docs samples are fragments — `btn Add
   * primary disabled={!draft.trim()}` is the point of the paragraph, and a `state draft=""` line
   * above it would be noise the reader has to skip on every sample.
   *
   * A fragment cannot compile on its own, so the preview supplies what it references. The shown
   * source stays exactly as authored, and the preview says so rather than implying the sample runs
   * unaided — a preview that quietly compiled something other than what is on screen would be worse
   * than no preview.
   */
  scaffold?: string;
  defaultTab?: "source" | "preview";
}) {
  const [tab, setTab] = useState<"source" | "preview">(defaultTab);
  const [showScaffold, setShowScaffold] = useState(false);

  // A sample that declares its own page is already a whole document; anything else needs one, since
  // `page` is what names the component the preview renders.
  const needsPage = !/^\s*page\s/m.test(code);
  const preamble = [needsPage && !scaffold?.trimStart().startsWith("page") ? "page Preview" : null, scaffold?.trim()]
    .filter(Boolean)
    .join("\n");
  const previewSource = preamble ? `${preamble}\n\n${code}` : code;
  const addedLines = preamble ? preamble.split("\n").length : 0;

  return (
    <div className={cn("not-prose", className)}>
      <div className="mb-2 flex items-center gap-1">
        {(
          [
            ["source", Code2, "source"],
            ["preview", Play, "preview"],
          ] as const
        ).map(([value, Icon, label]) => (
          <button
            key={value}
            type="button"
            onClick={() => setTab(value)}
            aria-pressed={tab === value}
            className={cn(
              "flex items-center gap-1.5 rounded-chip px-2.5 py-1 font-mono text-xs transition-colors",
              tab === value
                ? "bg-chalk/8 text-chalk"
                : "text-fog hover:text-chalk",
            )}
          >
            <Icon className="size-3" />
            {label}
          </button>
        ))}
      </div>

      {tab === "source" ? (
        <CodeBlock
          code={code}
          lang={lang}
          filename={filename}
          meter={meter}
          lines={lines}
          maxHeight={maxHeight}
        />
      ) : (
        <>
          <LivePreview
            source={previewSource}
            data={data}
            label={
              addedLines
                ? `live · compiled in your browser, with ${addedLines} line${addedLines > 1 ? "s" : ""} supplied`
                : "live · compiled in your browser"
            }
          />
          {addedLines ? (
            <div className="mt-2">
              <button
                type="button"
                onClick={() => setShowScaffold((v) => !v)}
                className="font-mono text-xs text-fog underline decoration-line hover:text-chalk"
              >
                {showScaffold ? "hide" : "show"} what the preview supplied
              </button>
              {showScaffold ? (
                <CodeBlock code={preamble} lang="guml" className="mt-2" filename="supplied for the preview" />
              ) : null}
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}
