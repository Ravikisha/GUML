import { ArrowLeft, ArrowRight } from "lucide-react";
import Link from "next/link";
import type { ReactNode } from "react";
import { neighbours } from "@/lib/nav";
import { DocsToc, type Toc } from "./docs-toc";
import { Meter } from "./ui";

export type { Toc };

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

        {/* Larger than the old 44px. A documentation page opens on a title and a lede, and at the
            landing page's scale that title is the only thing carrying this direction into the docs —
            at 44px it read as a section heading rather than as the top of a page. */}
        <h1 className="display-wide max-w-3xl text-heading-lg text-chalk">
          {title}
        </h1>
        {lede ? <div className="mt-6 max-w-[62ch] text-lg leading-relaxed text-fog">{lede}</div> : null}

        <div className="mt-12">{children}</div>

        {(prev || next) && (
          <nav className="mt-20 grid gap-3 border-t border-line pt-8 sm:grid-cols-2">
            {prev ? (
              <Link
                href={prev.href}
                className="group rounded-card border border-line p-4 transition-[border-color,box-shadow] hover:border-ember/40 hover:shadow-md"
              >
                <span className="label flex items-center gap-1.5 transition-colors group-hover:text-ember">
                  <ArrowLeft className="size-3 transition-transform group-hover:-translate-x-0.5" />{" "}
                  previous
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
                className="group rounded-card border border-line p-4 text-right transition-[border-color,box-shadow] hover:border-ember/40 hover:shadow-md"
              >
                <span className="label flex items-center justify-end gap-1.5 transition-colors group-hover:text-ember">
                  next{" "}
                  <ArrowRight className="size-3 transition-transform group-hover:translate-x-0.5" />
                </span>
                <span className="mt-2 block text-sm text-fog transition-colors group-hover:text-chalk">
                  {next.title}
                </span>
              </Link>
            ) : null}
          </nav>
        )}
      </article>

      {toc && toc.length > 0 ? <DocsToc items={toc} /> : null}
    </div>
  );
}
