import { Info, Lightbulb, TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";
import { npmUrl, pkg } from "@/lib/packages";
import { cn } from "@/lib/utils";

/* --------------------------------------------------------------------------
   Documentation content primitives.

   Explicit components rather than a markdown pipeline: heading ids are hand
   written so the table of contents can never drift from the page, and every
   block gets the site's type scale instead of a generic reset.

   Prose carries a measure. Between `lg` and `xl` the article column is ~848px
   wide with no table-of-contents rail beside it, which put body text at ~105
   characters a line — long enough that the eye loses the return sweep. `MEASURE`
   caps running text at 68ch and is deliberately *not* applied to tables, code
   listings or previews: those want the full column, and a code block wrapped to
   a reading measure is a code block that scrolls for no reason.
   -------------------------------------------------------------------------- */

const MEASURE = "max-w-[68ch]";

export function H2({ id, children }: { id: string; children: ReactNode }) {
  return (
    <h2
      id={id}
      className="display-narrow mt-16 max-w-3xl scroll-mt-28 text-2xl font-medium text-chalk first:mt-0 md:text-3xl"
    >
      <a href={`#${id}`} className="group no-underline">
        {children}
        <span className="ml-2 align-middle font-mono text-base text-fog-dim opacity-0 transition-opacity group-hover:opacity-100">
          #
        </span>
      </a>
    </h2>
  );
}

/* A mono subhead under a proportional H2 — the size gap between them was one step, so a run of
   H3s read as bold body copy. The rule above is what separates two sibling topics; it does the
   job the missing size step was failing to do, without adding a weight above 500. */
export function H3({ id, children }: { id?: string; children: ReactNode }) {
  return (
    <h3
      id={id}
      className="mt-12 max-w-3xl scroll-mt-28 border-t border-line pt-6 font-mono text-[1.05rem] font-medium tracking-tight text-chalk"
    >
      {children}
    </h3>
  );
}

export function P({ children, className }: { children: ReactNode; className?: string }) {
  return <p className={cn("mt-5 leading-[1.75] text-fog", MEASURE, className)}>{children}</p>;
}

export function Lede({ children }: { children: ReactNode }) {
  return <p className="mt-6 max-w-[62ch] text-lg leading-relaxed text-fog">{children}</p>;
}

export function UL({ children }: { children: ReactNode }) {
  return <ul className={cn("mt-5 space-y-2.5 text-fog", MEASURE)}>{children}</ul>;
}

export function OL({ children }: { children: ReactNode }) {
  return <ol className={cn("mt-5 space-y-3 text-fog", MEASURE)}>{children}</ol>;
}

export function LI({ children }: { children: ReactNode }) {
  return (
    <li className="relative pl-5 leading-[1.7] before:absolute before:top-[0.7em] before:left-0 before:h-1 before:w-1 before:rounded-full before:bg-fog-dim">
      {children}
    </li>
  );
}

/** Inline code. Deliberately not a full block: no scroll, no copy button. */
export function C({ children }: { children: ReactNode }) {
  return (
    <code className="rounded-[4px] border border-line bg-chalk/[0.04] px-1.5 py-0.5 font-mono text-[0.85em] text-chalk">
      {children}
    </code>
  );
}

/**
 * A published package, rendered as inline code that links to its npm page.
 *
 * The name is looked up in `lib/packages.ts` rather than passed through, so a typo is a build error
 * instead of a link to a registry 404 — which reads as "this package does not exist" rather than "this
 * page has a typo", and is exactly the wrong impression on an install page.
 */
export function Pkg({ name }: { name: string }) {
  const { name: verified } = pkg(name);
  return (
    <a
      href={npmUrl(verified)}
      target="_blank"
      rel="noreferrer"
      className="rounded-[4px] border border-line bg-chalk/[0.04] px-1.5 py-0.5 font-mono text-[0.85em] text-iris underline decoration-iris/30 underline-offset-2 transition-colors hover:decoration-iris"
    >
      {verified}
    </a>
  );
}

export function A({ href, children }: { href: string; children: ReactNode }) {
  const external = href.startsWith("http");
  return (
    <a
      href={href}
      {...(external ? { target: "_blank", rel: "noreferrer" } : {})}
      className="text-iris underline decoration-iris/40 underline-offset-2 transition-colors hover:decoration-iris"
    >
      {children}
    </a>
  );
}

const NOTE_TONE = {
  info: { cls: "border-iris/25 bg-iris/[0.05]", icon: Info, iconCls: "text-iris" },
  warn: { cls: "border-ember/25 bg-ember/[0.05]", icon: TriangleAlert, iconCls: "text-ember" },
  tip: { cls: "border-mint/25 bg-mint/[0.04]", icon: Lightbulb, iconCls: "text-mint" },
} as const;

export function Note({
  tone = "info",
  title,
  children,
}: {
  tone?: keyof typeof NOTE_TONE;
  title?: string;
  children: ReactNode;
}) {
  const { cls, icon: Icon, iconCls } = NOTE_TONE[tone];
  return (
    // Wider than `MEASURE` by the padding it adds, so the text inside a note lands on the same
    // measure as the paragraph above it rather than on a shorter one.
    <aside className={cn("mt-7 max-w-[72ch] rounded-card border p-5", cls)}>
      <div className="flex items-center gap-2">
        <Icon className={cn("size-4 shrink-0", iconCls)} />
        {title ? <p className="font-mono text-sm text-chalk">{title}</p> : null}
      </div>
      <div className="mt-2 text-sm leading-relaxed text-fog [&>p:first-child]:mt-0">{children}</div>
    </aside>
  );
}

export function Table({
  head,
  rows,
  className,
}: {
  head: string[];
  rows: ReactNode[][];
  className?: string;
}) {
  return (
    <div className={cn("mt-7 overflow-x-auto rounded-card border border-line", className)}>
      <table className="w-full min-w-[30rem] text-left text-sm">
        <thead>
          <tr className="border-b border-line bg-chalk/[0.02]">
            {head.map((h) => (
              <th key={h} className="label px-4 py-2.5 font-normal">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={i} className="border-b border-line align-top last:border-0">
              {row.map((cell, j) => (
                <td key={j} className="px-4 py-3 leading-relaxed text-fog first:text-chalk">
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Numbered steps. Used only where order genuinely matters. */
export function Steps({ children }: { children: ReactNode }) {
  return <ol className="mt-8 max-w-[72ch] space-y-8 border-l border-line pl-7">{children}</ol>;
}

export function Step({ n, title, children }: { n: number; title: string; children: ReactNode }) {
  return (
    <li className="relative">
      <span className="absolute -left-[2.32rem] flex size-6 items-center justify-center rounded-full border border-line-strong bg-ink font-mono text-[0.7rem] text-fog">
        {n}
      </span>
      <p className="font-mono text-sm text-chalk">{title}</p>
      <div className="[&>*:first-child]:mt-3">{children}</div>
    </li>
  );
}
