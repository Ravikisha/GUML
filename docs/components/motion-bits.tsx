"use client";

import {
  animate,
  motion,
  useInView,
  useMotionValue,
  useReducedMotion,
  useTransform,
} from "motion/react";
import { useEffect, useRef, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/* --------------------------------------------------------------------------
   Motion pieces, in the spirit of Magic UI / Aceternity but re-cut to this
   site's tokens: one accent duo, no gradient stacks, and every effect either
   carries data or gets removed.
   -------------------------------------------------------------------------- */

/** A counter that only animates once, when scrolled into view. */
export function NumberTicker({
  value,
  from = 0,
  duration = 1.4,
  className,
}: {
  value: number;
  from?: number;
  duration?: number;
  className?: string;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  const inView = useInView(ref, { once: true, margin: "-15% 0px" });
  const reduce = useReducedMotion();
  const mv = useMotionValue(reduce ? value : from);
  const text = useTransform(mv, (v) => Math.round(v).toLocaleString("en-US"));

  useEffect(() => {
    if (!inView || reduce) return;
    const controls = animate(mv, value, { duration, ease: [0.16, 1, 0.3, 1] });
    return () => controls.stop();
  }, [inView, mv, value, duration, reduce]);

  return (
    <motion.span ref={ref} className={cn("tabular-nums", className)}>
      {text}
    </motion.span>
  );
}

/** Scroll reveal. Deliberately small: 14px and one beat, not a stage entrance. */
export function Reveal({
  children,
  delay = 0,
  className,
}: {
  children: ReactNode;
  delay?: number;
  className?: string;
}) {
  const reduce = useReducedMotion();
  if (reduce) return <div className={className}>{children}</div>;
  return (
    <motion.div
      className={className}
      initial={{ opacity: 0, y: 14 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-10% 0px" }}
      transition={{ duration: 0.55, delay, ease: [0.16, 1, 0.3, 1] }}
    >
      {children}
    </motion.div>
  );
}

/** Infinite horizontal strip. Used once, for the diagnostic-code ribbon. */
export function Marquee({
  children,
  className,
  reverse,
}: {
  children: ReactNode;
  className?: string;
  reverse?: boolean;
}) {
  return (
    <div
      className={cn(
        "group relative flex overflow-hidden [mask-image:linear-gradient(to_right,transparent,black_8%,black_92%,transparent)]",
        className,
      )}
    >
      <div
        className={cn(
          "flex w-max shrink-0 animate-marquee items-center gap-3 group-hover:[animation-play-state:paused]",
          reverse && "[animation-direction:reverse]",
        )}
      >
        {children}
        {children}
      </div>
    </div>
  );
}

/** Cursor-following light, kept to a single low-opacity iris wash. */
export function Spotlight({ className }: { className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const x = useMotionValue(50);
  const y = useMotionValue(0);
  const reduce = useReducedMotion();

  useEffect(() => {
    if (reduce) return;
    const el = ref.current?.parentElement;
    if (!el) return;
    const onMove = (e: PointerEvent) => {
      const r = el.getBoundingClientRect();
      x.set(((e.clientX - r.left) / r.width) * 100);
      y.set(((e.clientY - r.top) / r.height) * 100);
    };
    el.addEventListener("pointermove", onMove);
    return () => el.removeEventListener("pointermove", onMove);
  }, [x, y, reduce]);

  const background = useTransform(
    [x, y],
    ([px, py]) =>
      `radial-gradient(420px circle at ${px}% ${py}%, rgb(108 76 255 / 0.16), transparent 65%)`,
  );

  return (
    <motion.div
      ref={ref}
      aria-hidden
      style={{ background }}
      className={cn("pointer-events-none absolute inset-0 z-0", className)}
    />
  );
}

/** Dot grid backdrop — the indentation grid GUML actually parses on. */
export function DotGrid({ className }: { className?: string }) {
  return (
    <div
      aria-hidden
      className={cn(
        "pointer-events-none absolute inset-0 z-0 [mask-image:radial-gradient(60%_50%_at_50%_0%,black,transparent)]",
        className,
      )}
      style={{
        backgroundImage:
          "radial-gradient(circle at 1px 1px, rgb(255 255 255 / 0.14) 1px, transparent 0)",
        backgroundSize: "18px 18px",
      }}
    />
  );
}
