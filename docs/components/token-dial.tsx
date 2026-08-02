"use client";

import gsap from "gsap";
import { useCallback, useEffect, useRef, useState } from "react";
import { cn, commas } from "@/lib/utils";

/**
 * The Token Dial — this page's signature.
 *
 * One ring, read as a measuring instrument rather than a pie chart. The full circumference is a
 * *fixed* scale: the largest React baseline in the measured set. Every fixture is then drawn against
 * that same circle, which is the only way switching fixtures stays comparable — a ring that
 * re-normalised per fixture would show three identical shapes and claim to be a measurement.
 *
 * Two arcs share a start point at twelve o'clock:
 *
 *   ghost  — what the React baseline costs. Warm neutral, because the baseline is being measured,
 *            not sold.
 *   signal — what GUML costs. Signal Orange, the page's single accent marking its single idea.
 *
 * The burn is the orange arc travelling from the ghost's length down to its own, with the counter
 * falling alongside it. That is the whole animation on the page; everything else stays still.
 *
 * Ticks are every 100 tokens of the same scale, so the ring carries units. Nothing here is chosen
 * for how it looked at this size — the geometry comes out of the figures in the research report.
 */

export type DialFixture = {
  id: string;
  title: string;
  /** Tokens of GUML. */
  guml: number;
  /** Tokens of the React baseline it stands in for. */
  react: number;
};

/* Geometry. A viewBox so the dial scales with its container and the stroke never thins out.
   Thinner than it started: at 22px the ghost arc — 87% of the circle on the task fixture — made the
   warm neutral the single largest object in a hero that is supposed to have one hue. The band lost
   4px and most of its opacity, and the ring stopped competing with the thing it is measuring. */
const SIZE = 420;
const C = SIZE / 2;
const R = 172;
const STROKE = 18;
const CIRC = 2 * Math.PI * R;

/** One tick per 100 tokens. Stated in the caption, because a tick with no unit is decoration. */
const TOKENS_PER_TICK = 100;

const BURN_SECONDS = 1.5;

/** Rounded to three decimals — well under a device pixel at this scale, and *deterministic*.
    `Math.cos` is only required to be implementation-approximate, so Node and V8-in-Chrome disagreed
    in the seventeenth digit: the server wrote `341.9327996332046` and the client computed
    `341.93279963320464`, which React reported as a hydration mismatch on every tick mark. */
const fixed = (n: number) => Math.round(n * 1000) / 1000;

function polar(value: number, scale: number, radius: number) {
  // −90° so the scale starts at twelve o'clock, which is where a dial is read from.
  const a = ((value / scale) * 360 - 90) * (Math.PI / 180);
  return { x: fixed(C + radius * Math.cos(a)), y: fixed(C + radius * Math.sin(a)) };
}

export function TokenDial({
  fixtures,
  initial,
  className,
}: {
  fixtures: DialFixture[];
  /** Which fixture the dial opens on. Defaults to the first. */
  initial?: string;
  className?: string;
}) {
  const scale = Math.max(...fixtures.map((f) => f.react));
  const [activeId, setActiveId] = useState(
    () => fixtures.find((f) => f.id === initial)?.id ?? fixtures[0].id,
  );
  const active = fixtures.find((f) => f.id === activeId) ?? fixtures[0];

  const arc = useRef<SVGCircleElement>(null);
  const readout = useRef<HTMLSpanElement>(null);
  const tween = useRef<gsap.core.Tween | null>(null);

  const run = useCallback(
    (f: DialFixture, immediate: boolean) => {
      const ring = arc.current;
      const num = readout.current;
      if (!ring || !num) return;

      // Killed by handle rather than by target: each run allocates a fresh state object, so
      // `killTweensOf(state)` would leave the *previous* tween alive and still writing to the same
      // two nodes — two counters fighting over one element.
      tween.current?.kill();

      const to = f.guml / scale;
      const state = { p: immediate ? to : f.react / scale, v: immediate ? f.guml : f.react };
      const write = () => {
        ring.setAttribute("stroke-dasharray", `${CIRC * state.p} ${CIRC}`);
        num.textContent = commas(Math.round(state.v));
      };

      write();
      if (immediate) return;

      tween.current = gsap.to(state, {
        p: to,
        v: f.guml,
        duration: BURN_SECONDS,
        ease: "power2.inOut",
        onUpdate: write,
      });
    },
    [scale],
  );

  useEffect(() => {
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    run(active, reduce);
    return () => {
      tween.current?.kill();
    };
  }, [active, run]);

  const ratio = (active.react / active.guml).toFixed(1);
  const saved = Math.round((1 - active.guml / active.react) * 100);
  const ticks = Math.floor(scale / TOKENS_PER_TICK);

  return (
    <div className={cn("flex flex-col items-center", className)}>
      <div className="relative w-full max-w-[30rem]">
        <svg viewBox={`0 0 ${SIZE} ${SIZE}`} className="w-full" aria-hidden>
          {/* The scale itself: the full circle is `scale` tokens. */}
          <circle
            cx={C}
            cy={C}
            r={R}
            fill="none"
            strokeWidth={STROKE}
            className="stroke-chalk/[0.05]"
          />

          {/* Units. Every hundredth token, with a longer mark every five hundred. */}
          {/* Raised from /30 at 0.4: a minor tick landed at ~0.12 effective opacity on paper, so the
              ring read as four stray specks rather than as a scale with units. */}
          <g className="stroke-chalk/45">
            {Array.from({ length: ticks + 1 }, (_, i) => {
              const value = i * TOKENS_PER_TICK;
              const major = value % 500 === 0;
              const from = polar(value, scale, R + STROKE / 2 + 6);
              const to = polar(value, scale, R + STROKE / 2 + (major ? 16 : 10));
              return (
                <line
                  key={value}
                  x1={from.x}
                  y1={from.y}
                  x2={to.x}
                  y2={to.y}
                  strokeWidth={major ? 1.5 : 1}
                  strokeLinecap="round"
                  opacity={major ? 1 : 0.5}
                />
              );
            })}
          </g>

          {/* Ghost: the React baseline, left standing so the burn has something to be measured
              against. Rotated so both arcs share twelve o'clock as their origin. */}
          <circle
            cx={C}
            cy={C}
            r={R}
            fill="none"
            strokeWidth={STROKE}
            strokeLinecap="round"
            strokeDasharray={`${CIRC * (active.react / scale)} ${CIRC}`}
            transform={`rotate(-90 ${C} ${C})`}
            // A wash of the accent rather than a warm grey. Even at 14% the neutral read as *grey*,
            // and it is the largest object in the hero — which put a second temperature on a page
            // whose entire licence is having one. Same hue, two intensities: the ring is Linen
            // Blush, the arc is Signal Orange.
            className="stroke-ember/[0.15]"
          />

          {/* Signal: what GUML costs. Animated from the ghost's length down to its own. */}
          <circle
            ref={arc}
            cx={C}
            cy={C}
            r={R}
            fill="none"
            strokeWidth={STROKE}
            strokeLinecap="round"
            strokeDasharray={`${CIRC * (active.react / scale)} ${CIRC}`}
            transform={`rotate(-90 ${C} ${C})`}
            className="stroke-ember"
          />
        </svg>

        {/* The readout, in HTML rather than SVG text so it inherits the page's type scale instead
            of needing a second one. At `text-display` it fills the ring the way the direction asks
            display type to — the first pass set it a step smaller and left a hollow donut. */}
        <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center px-[16%] text-center">
          <p className="display-wide text-display text-chalk">
            <span ref={readout} className="tabular-nums">
              {commas(active.react)}
            </span>
          </p>
          <p className="mt-2 font-mono text-[0.7rem] text-fog">tokens · {active.id}.guml</p>
          <p className="mt-5 font-mono text-sm text-ember">{ratio}× smaller</p>
        </div>
      </div>

      {/* Which fixture the dial is reading. Three, because the site reports a range rather than a
          blended average — compression is bounded by prose, and the switcher is where that shows. */}
      <div
        role="tablist"
        aria-label="Measured fixture"
        className="mt-8 flex flex-wrap justify-center gap-2"
      >
        {fixtures.map((f) => {
          const on = f.id === active.id;
          return (
            <button
              key={f.id}
              type="button"
              role="tab"
              aria-selected={on}
              onClick={() => (on ? run(f, false) : setActiveId(f.id))}
              className={cn(
                "tracked rounded-full border px-3.5 py-1.5 font-mono text-[0.7rem] transition-colors",
                on
                  ? "border-ember/40 bg-ember-dim text-ember"
                  : "border-line text-fog hover:border-ember/40 hover:text-chalk",
              )}
            >
              {f.title}
            </button>
          );
        })}
      </div>

      {/* One caption, each figure stated once. The first pass had a swatch legend *and* a scale note
          under a readout that already said 178 — the number appeared three times in one component,
          which reads as a component unsure which of its numbers is the point. */}
      <p className="mt-6 max-w-md text-center font-mono text-[0.7rem] leading-relaxed text-fog-dim">
        Burned down from{" "}
        <span className="tabular-nums text-chalk">{commas(active.react)}</span> tokens of React —{" "}
        {saved}% fewer. 1 tick = {TOKENS_PER_TICK} tokens; the full ring is {commas(scale)}, the
        largest baseline in the set, so all three fixtures share one scale.
      </p>
    </div>
  );
}
