"use client";

import gsap from "gsap";
import { RotateCcw } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { cn, commas } from "@/lib/utils";

/**
 * The Compressor — this page's signature.
 *
 * Every cell is 8 real tokens of the task-CRUD fixture. React is 1,441 tokens, so the grid starts
 * as 180 neutral cells; GUML is 178, so 158 burn away and 22 remain. The counter falls with them.
 *
 * It is a chart, not an ornament: the cell count comes from the measured figures in the research
 * report rather than from what looked good at this size.
 *
 * Two colours, and only two. The cells that survive are Signal Orange — the page's single accent
 * marking its single idea, what GUML costs — and the ones they replace are the warm neutral, because
 * the baseline is what is being measured, not what is being sold. An earlier version added a third
 * hue for the survivors, which meant the one moment the accent existed to explain had two voices
 * competing to explain it.
 */

/** Signal Orange: what GUML costs. */
const SIGNAL = "#f73b20";
/** Sand Wash: the tokens it replaces. Neutral on purpose — the baseline is measured, not sold. */
const SPENT = "#e7dcdb";

const TOKENS_PER_CELL = 8;
const REACT_TOKENS = 1441;
const GUML_TOKENS = 178;

const TOTAL_CELLS = Math.round(REACT_TOKENS / TOKENS_PER_CELL); // 179
const KEPT_CELLS = Math.round(GUML_TOKENS / TOKENS_PER_CELL); // 22

export function Compressor({ className }: { className?: string }) {
  const root = useRef<HTMLDivElement>(null);
  const counter = useRef<HTMLSpanElement>(null);
  const [ran, setRan] = useState(false);

  const run = useCallback((immediate = false) => {
    const el = root.current;
    if (!el) return;

    const cells = gsap.utils.toArray<HTMLElement>("[data-cell]", el);
    const burn = cells.slice(KEPT_CELLS);
    const keep = cells.slice(0, KEPT_CELLS);
    const state = { value: REACT_TOKENS };

    const write = () => {
      if (counter.current) counter.current.textContent = commas(Math.round(state.value));
    };

    gsap.killTweensOf([...cells, state]);

    if (immediate) {
      gsap.set(burn, { opacity: 0.06, scale: 0.7, backgroundColor: SPENT });
      gsap.set(keep, { opacity: 1, scale: 1, backgroundColor: SIGNAL });
      state.value = GUML_TOKENS;
      write();
      setRan(true);
      return;
    }

    gsap.set(cells, { opacity: 1, scale: 1, backgroundColor: SPENT });
    state.value = REACT_TOKENS;
    write();

    const tl = gsap.timeline({ defaults: { ease: "power2.inOut" }, onComplete: () => setRan(true) });

    tl.to(burn, {
      opacity: 0.06,
      scale: 0.7,
      backgroundColor: SPENT,
      duration: 0.5,
      stagger: { each: 0.006, from: "end" },
    })
      .to(keep, { backgroundColor: SIGNAL, duration: 0.5 }, "-=0.45")
      .to(state, { value: GUML_TOKENS, duration: 1.1, onUpdate: write }, 0.1);
  }, []);

  useEffect(() => {
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const ctx = gsap.context(() => run(reduce), root);
    return () => ctx.revert();
  }, [run]);

  return (
    <div ref={root} className={cn("relative", className)}>
      <div className="flex items-end justify-between gap-4 pb-4">
        <div>
          <p className="label mb-2">task CRUD · same app, two representations</p>
          <p className="display-wide flex items-baseline gap-2 text-heading-sm">
            <span ref={counter} className="tabular-nums text-chalk">
              {commas(REACT_TOKENS)}
            </span>
            <span className="font-mono text-xs font-normal tracking-normal text-fog">tokens</span>
          </p>
        </div>
        <button
          type="button"
          onClick={() => run(false)}
          className="inline-flex items-center gap-1.5 rounded-full border border-line-strong px-3 py-1.5 font-mono text-[0.7rem] text-fog transition-colors hover:border-chalk/30 hover:text-chalk"
        >
          <RotateCcw className="size-3" />
          replay
        </button>
      </div>

      {/* The cell field. aria-hidden because the figures are stated in text
          directly beneath it — this is the visual, not the record. */}
      <div
        aria-hidden
        className="grid gap-[3px] rounded-card border border-line bg-ink-high p-3"
        style={{ gridTemplateColumns: "repeat(auto-fill, minmax(11px, 1fr))" }}
      >
        {Array.from({ length: TOTAL_CELLS }, (_, i) => (
          <span
            key={i}
            data-cell
            className="aspect-square rounded-[2px] bg-ember"
            style={{ willChange: "transform, opacity" }}
          />
        ))}
      </div>

      <dl className="mt-4 grid grid-cols-3 gap-3 font-mono text-xs">
        <div className="rounded-chip border border-line bg-ink px-3 py-2.5">
          <dt className="label mb-1.5">React + TS + Tailwind</dt>
          <dd className="tabular-nums text-chalk">{commas(REACT_TOKENS)} tokens · 187 lines</dd>
        </div>
        <div className="rounded-chip border border-ember/30 bg-ember-dim px-3 py-2.5">
          <dt className="label mb-1.5 text-ember">GUML</dt>
          <dd className="tabular-nums text-chalk">{commas(GUML_TOKENS)} tokens · 24 lines</dd>
        </div>
        <div className="rounded-chip border border-mint/25 bg-mint/5 px-3 py-2.5">
          <dt className="label mb-1.5 text-mint/70">saved</dt>
          <dd
            className={cn(
              "tabular-nums text-mint transition-opacity duration-500",
              ran ? "opacity-100" : "opacity-40",
            )}
          >
            88% · 8.1× fewer
          </dd>
        </div>
      </dl>

      <p className="mt-3 font-mono text-[0.7rem] text-fog-dim">
        1 cell = {TOKENS_PER_CELL} tokens · measured with cl100k_base on hand-authored fixtures
      </p>
    </div>
  );
}
