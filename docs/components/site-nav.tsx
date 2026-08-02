"use client";

import { Menu, X } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState } from "react";
import { NAV } from "@/lib/nav";
import { cn } from "@/lib/utils";
import { CommandMenu } from "./command-menu";
import { Logo } from "./logo";
import { ThemeToggle } from "./theme-toggle";

const TOP = [
  { title: "Docs", href: "/docs" },
  { title: "Examples", href: "/examples" },
  { title: "Playground", href: "/playground" },
  { title: "Chat", href: "/chat" },
  { title: "Research", href: "/research" },
];

export function SiteNav() {
  const pathname = usePathname();
  // Store which route the menu was opened on rather than syncing an effect to
  // `pathname`: navigating changes the route, so the menu closes itself during
  // render with no cascading state update.
  const [openedAt, setOpenedAt] = useState<string | null>(null);
  const open = openedAt === pathname;

  return (
    // A floating pill rather than a full-width bar with a bottom rule. The page separates its
    // sections with whitespace alone, so a hairline pinned across the top would be the one border
    // fighting that — and the 84px radius is the shape this direction is recognised by.
    <header className="sticky top-0 z-50 px-4 pt-3 md:px-6 md:pt-4">
      <div className="mx-auto flex h-14 max-w-(--container-page) items-center gap-6 rounded-nav border border-line bg-ink/80 px-4 shadow-lg backdrop-blur-xl md:px-6">
        <Link href="/" className="group flex items-center gap-2.5">
          {/* The mark sits with the wordmark rather than replacing it: at 22px the six bars are legible
              but the droplets are not, so the name still has to be there. */}
          <Logo className="size-6 shrink-0 text-chalk" />
          <span className="display-narrow text-xl leading-none text-chalk">GUML</span>
          <span className="hidden font-mono text-[0.65rem] text-fog-dim transition-colors group-hover:text-ember sm:inline">
            v0.1
          </span>
        </Link>

        <nav className="hidden items-center gap-1 md:flex">
          {TOP.map((item) => {
            const active = pathname.startsWith(item.href);
            return (
              <Link
                key={item.href}
                href={item.href}
                // Orange marks where you are. That is the accent doing a job rather than decorating
                // one, which is the whole licence for having a single hue.
                className={cn(
                  "tracked rounded-full px-3 py-1.5 text-body-sm transition-colors",
                  active ? "text-ember" : "text-fog hover:text-chalk",
                )}
              >
                {item.title}
              </Link>
            );
          })}
        </nav>

        <div className="ml-auto flex items-center gap-3">
          <CommandMenu />
          <ThemeToggle />
          <Link
            href="/docs/quickstart"
            className="tracked hidden rounded-button bg-ember px-4 py-2 text-body-sm font-medium text-white transition-opacity hover:opacity-90 md:inline-flex"
          >
            Get started
          </Link>
          <button
            type="button"
            onClick={() => setOpenedAt(open ? null : pathname)}
            aria-label={open ? "Close menu" : "Open menu"}
            aria-expanded={open}
            className="inline-flex size-9 items-center justify-center rounded-full border border-line-strong text-fog md:hidden"
          >
            {open ? <X className="size-4" /> : <Menu className="size-4" />}
          </button>
        </div>
      </div>

      {open && (
        <div className="mx-auto mt-2 max-w-(--container-page) rounded-panel border border-line bg-ink px-6 pb-8 pt-5 shadow-lg md:hidden">
          {NAV.map((group) => (
            <div key={group.title} className="mb-6">
              <p className="label mb-2">{group.title}</p>
              <ul className="space-y-1">
                {group.items.map((item) => (
                  <li key={item.href}>
                    <Link
                      href={item.href}
                      className={cn(
                        "block rounded-chip px-2 py-1.5 text-sm",
                        pathname === item.href ? "bg-chalk/8 text-chalk" : "text-fog",
                      )}
                    >
                      {item.title}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      )}
    </header>
  );
}
