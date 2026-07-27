import Link from "next/link";
import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/utils";

/* --------------------------------------------------------------------------
   Primitives, shadcn-style: owned in-repo, styled from the site's own tokens
   rather than a default theme.
   -------------------------------------------------------------------------- */

const BUTTON_BASE =
  "inline-flex items-center justify-center gap-2 rounded-full font-medium " +
  "transition-[transform,background-color,border-color,color] duration-150 " +
  "active:scale-[0.98] disabled:pointer-events-none disabled:opacity-40 whitespace-nowrap";

const BUTTON_VARIANT = {
  primary: "bg-chalk text-ink hover:bg-white",
  ember: "bg-ember text-ink hover:brightness-110",
  outline: "border border-white/20 text-chalk hover:border-white/40 hover:bg-white/5",
  quiet: "text-fog hover:text-chalk",
} as const;

const BUTTON_SIZE = {
  sm: "h-9 px-4 text-sm",
  md: "h-11 px-5 text-[0.95rem]",
  lg: "h-12 px-6 text-base",
} as const;

type ButtonProps = {
  variant?: keyof typeof BUTTON_VARIANT;
  size?: keyof typeof BUTTON_SIZE;
  className?: string;
  children: ReactNode;
};

export function Button({
  variant = "primary",
  size = "md",
  className,
  children,
  ...rest
}: ButtonProps & ComponentProps<"button">) {
  return (
    <button
      className={cn(BUTTON_BASE, BUTTON_VARIANT[variant], BUTTON_SIZE[size], className)}
      {...rest}
    >
      {children}
    </button>
  );
}

export function ButtonLink({
  variant = "primary",
  size = "md",
  className,
  children,
  href,
  ...rest
}: ButtonProps & ComponentProps<typeof Link>) {
  return (
    <Link
      href={href}
      className={cn(BUTTON_BASE, BUTTON_VARIANT[variant], BUTTON_SIZE[size], className)}
      {...rest}
    >
      {children}
    </Link>
  );
}

export function Badge({
  children,
  tone = "neutral",
  className,
}: {
  children: ReactNode;
  tone?: "neutral" | "ember" | "iris" | "mint";
  className?: string;
}) {
  const tones = {
    neutral: "border-white/15 text-fog",
    ember: "border-ember/40 text-ember bg-ember/10",
    iris: "border-iris/50 text-iris bg-iris/10",
    mint: "border-mint/40 text-mint bg-mint/10",
  } as const;
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 font-mono text-[0.65rem] uppercase tracking-[0.12em]",
        tones[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}

export function Panel({
  children,
  className,
  as: As = "div",
}: {
  children: ReactNode;
  className?: string;
  as?: "div" | "section" | "aside";
}) {
  return (
    <As
      className={cn(
        "rounded-panel border border-white/8 bg-ink-raised/70 backdrop-blur-[2px]",
        className,
      )}
    >
      {children}
    </As>
  );
}

/**
 * The site's structural device. A meter carries a real value — a token count, a
 * phase number, a diagnostic code — never a decorative number.
 */
export function Meter({
  label,
  value,
  tone = "neutral",
  className,
}: {
  label: string;
  value: ReactNode;
  tone?: "neutral" | "ember" | "iris" | "mint";
  className?: string;
}) {
  const tones = {
    neutral: "text-chalk",
    ember: "text-ember",
    iris: "text-iris",
    mint: "text-mint",
  } as const;
  return (
    <div className={cn("flex items-baseline gap-2", className)}>
      <span className="label">{label}</span>
      <span className={cn("font-mono text-sm tabular-nums", tones[tone])}>{value}</span>
    </div>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return (
    <kbd className="rounded-md border border-white/15 bg-white/5 px-1.5 py-0.5 font-mono text-[0.7rem] text-fog">
      {children}
    </kbd>
  );
}

/** Section wrapper: consistent gutters and the one-per-section meter slot. */
export function Section({
  children,
  className,
  meter,
  id,
}: {
  children: ReactNode;
  className?: string;
  meter?: { label: string; value: ReactNode; tone?: "neutral" | "ember" | "iris" | "mint" };
  id?: string;
}) {
  return (
    <section id={id} className={cn("relative border-t border-white/8 px-6 py-20 md:px-10", className)}>
      <div className="mx-auto max-w-6xl">
        {meter ? (
          <div className="mb-10 flex items-center justify-between">
            <Meter label={meter.label} value={meter.value} tone={meter.tone} />
            <span className="h-px w-24 bg-white/10 md:w-48" aria-hidden />
          </div>
        ) : null}
        {children}
      </div>
    </section>
  );
}
