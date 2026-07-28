"use client";

import * as Tabs from "@radix-ui/react-tabs";
import { highlight, type Lang, CLASS_STYLE } from "@/lib/highlight";
import { cn, commas, reduction } from "@/lib/utils";
import { CopyButton } from "./copy-button";

type Pane = {
  id: string;
  label: string;
  lang: Lang;
  code: string;
  tokens?: number;
  /** Shown under the tab strip: what this representation costs you. */
  note?: string;
};

/**
 * Side-by-side representations of the same app. The tab strip is the argument,
 * so each tab carries its own token count and the saving is stated once, in
 * mint, next to the pane that earned it.
 */
export function CodeCompare({
  panes,
  baseline,
  className,
  maxHeight = 460,
}: {
  panes: Pane[];
  /** Which pane's token count everything else is measured against. */
  baseline?: string;
  className?: string;
  maxHeight?: number;
}) {
  const base = panes.find((p) => p.id === baseline) ?? panes[0];

  return (
    <Tabs.Root
      defaultValue={panes[0].id}
      className={cn("overflow-hidden rounded-panel border border-line bg-code code-surface", className)}
    >
      <Tabs.List className="flex flex-wrap items-center gap-1 border-b border-line bg-chalk/[0.02] p-1.5">
        {panes.map((pane) => {
          const cut =
            pane.tokens && base.tokens && pane.id !== base.id
              ? reduction(base.tokens, pane.tokens)
              : null;
          return (
            <Tabs.Trigger
              key={pane.id}
              value={pane.id}
              className={cn(
                "group flex items-center gap-2 rounded-chip px-3 py-2 font-mono text-xs text-fog transition-colors",
                "hover:text-chalk data-[state=active]:bg-chalk/8 data-[state=active]:text-chalk",
              )}
            >
              <span>{pane.label}</span>
              {pane.tokens ? (
                <span className="tabular-nums text-fog-dim group-data-[state=active]:text-fog">
                  {commas(pane.tokens)}t
                </span>
              ) : null}
              {cut && cut > 0 ? <span className="tabular-nums text-mint">−{cut}%</span> : null}
            </Tabs.Trigger>
          );
        })}
      </Tabs.List>

      {panes.map((pane) => {
        const rows = highlight(pane.code, pane.lang);
        return (
          <Tabs.Content key={pane.id} value={pane.id} className="relative outline-none">
            <div className="flex items-center justify-between gap-4 border-b border-line px-4 py-2">
              <span className="label">
                {rows.length} lines
                {pane.note ? <span className="ml-3 normal-case tracking-normal">{pane.note}</span> : null}
              </span>
              <CopyButton text={pane.code} />
            </div>
            <div className="overflow-auto" style={{ maxHeight }}>
              <pre className="w-max min-w-full px-4 py-4 font-mono text-[0.8rem] leading-[1.65]">
                <code>
                  {rows.map((row, i) => (
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
                  ))}
                </code>
              </pre>
            </div>
          </Tabs.Content>
        );
      })}
    </Tabs.Root>
  );
}
