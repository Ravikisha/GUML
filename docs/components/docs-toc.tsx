"use client";

import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

export type Toc = Array<{ id: string; title: string }>;

/**
 * On-this-page rail with a live marker.
 *
 * Without one, the rail told you what a long page contains but never where you are in it — which is
 * the question a reader scrolling a syntax reference is actually asking. The marker is Signal Orange
 * because that is what the accent does across this site: it marks position, in the top nav, in the
 * sidebar, and here.
 *
 * The active id is decided from measured positions rather than from `IntersectionObserver`
 * entries alone: an observer fires on *crossings*, so a page loaded mid-scroll, or a heading whose
 * section is taller than the viewport, leaves nothing intersecting and no entry to read. Taking the
 * last heading above the scroll line always has an answer.
 */
export function DocsToc({ items }: { items: Toc }) {
  const [active, setActive] = useState(items[0]?.id ?? "");

  useEffect(() => {
    if (items.length === 0) return;

    // Matches `scroll-padding-top` in globals.css, so clicking a link and scrolling to the same
    // place agree on which heading is current.
    const LINE = 112;

    let frame = 0;
    const read = () => {
      frame = 0;
      let current = items[0].id;
      for (const item of items) {
        const el = document.getElementById(item.id);
        if (el && el.getBoundingClientRect().top <= LINE) current = item.id;
      }
      // Bottom of the document: the last heading wins even if its section is short enough to sit
      // below the line, otherwise the final entry can never become active on a tall page.
      if (window.innerHeight + window.scrollY >= document.body.scrollHeight - 2) {
        current = items[items.length - 1].id;
      }
      setActive(current);
    };

    const onScroll = () => {
      if (frame) return;
      frame = requestAnimationFrame(read);
    };

    read();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);
    return () => {
      if (frame) cancelAnimationFrame(frame);
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
    };
  }, [items]);

  return (
    <aside className="sticky top-24 hidden h-fit w-52 shrink-0 xl:block">
      <p className="label mb-3">on this page</p>
      <ul className="space-y-2 border-l border-line">
        {items.map((item) => {
          const on = item.id === active;
          return (
            <li key={item.id}>
              <a
                href={`#${item.id}`}
                aria-current={on ? "location" : undefined}
                className={cn(
                  "-ml-px block border-l pl-4 text-sm transition-colors",
                  on
                    ? "border-ember text-chalk"
                    : "border-transparent text-fog hover:border-ember/40 hover:text-chalk",
                )}
              >
                {item.title}
              </a>
            </li>
          );
        })}
      </ul>
    </aside>
  );
}
