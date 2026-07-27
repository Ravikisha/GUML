import { ArrowLeft, ArrowRight } from "lucide-react";
import Link from "next/link";
import type { ReactNode } from "react";
import { neighbours } from "@/lib/nav";
import { Meter } from "./ui";

export type Toc = Array<{ id: string; title: string }>;

/**
 * Shell for a documentation page: title block, on-this-page rail, and a pager
 * derived from the same nav order as the sidebar, so the reading path through
 * the docs is defined in exactly one place.
 */
export function DocPage({
  title,
  lede,
  meter,
  toc,
  pathname,
  children,
}: {
  title: string;
  lede?: ReactNode;
  meter?: { label: string; value: string; tone?: "neutral" | "ember" | "iris" | "mint" };
  toc?: Toc;
  pathname: string;
  children: ReactNode;
}) {
  const { prev, next } = neighbours(pathname);

  return (
    <div className="flex gap-12">
      <article className="min-w-0 flex-1 pb-16">
        {meter ? <Meter label={meter.label} value={meter.value} tone={meter.tone} className="mb-6" /> : null}

        <h1 className="display-narrow text-4xl font-extrabold tracking-[-0.02em] text-chalk md:text-5xl">
          {title}
        </h1>
        {lede ? <div className="mt-6 max-w-2xl text-lg leading-relaxed text-fog">{lede}</div> : null}

        <div className="mt-12">{children}</div>

        {(prev || next) && (
          <nav className="mt-20 grid gap-3 border-t border-white/8 pt-8 sm:grid-cols-2">
            {prev ? (
              <Link
                href={prev.href}
                className="group rounded-card border border-white/8 p-4 transition-colors hover:border-white/20"
              >
                <span className="label flex items-center gap-1.5">
                  <ArrowLeft className="size-3" /> previous
                </span>
                <span className="mt-2 block text-sm text-fog transition-colors group-hover:text-chalk">
                  {prev.title}
                </span>
              </Link>
            ) : (
              <span />
            )}
            {next ? (
              <Link
                href={next.href}
                className="group rounded-card border border-white/8 p-4 text-right transition-colors hover:border-white/20"
              >
                <span className="label flex items-center justify-end gap-1.5">
                  next <ArrowRight className="size-3" />
                </span>
                <span className="mt-2 block text-sm text-fog transition-colors group-hover:text-chalk">
                  {next.title}
                </span>
              </Link>
            ) : null}
          </nav>
        )}
      </article>

      {toc && toc.length > 0 ? (
        <aside className="sticky top-24 hidden h-fit w-52 shrink-0 xl:block">
          <p className="label mb-3">on this page</p>
          <ul className="space-y-2 border-l border-white/8">
            {toc.map((item) => (
              <li key={item.id}>
                <a
                  href={`#${item.id}`}
                  className="-ml-px block border-l border-transparent pl-4 text-sm text-fog transition-colors hover:border-white/25 hover:text-chalk"
                >
                  {item.title}
                </a>
              </li>
            ))}
          </ul>
        </aside>
      ) : null}
    </div>
  );
}
