"use client";

import { Menu, X } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState } from "react";
import { NAV } from "@/lib/nav";
import { cn } from "@/lib/utils";
import { CommandMenu } from "./command-menu";
import { ThemeToggle } from "./theme-toggle";

const TOP = [
  { title: "Docs", href: "/docs" },
  { title: "Examples", href: "/examples" },
  { title: "Playground", href: "/playground" },
  { title: "Chat", href: "/chat" },
  { title: "Research", href: "/docs/research/measurements" },
];

export function SiteNav() {
  const pathname = usePathname();
  // Store which route the menu was opened on rather than syncing an effect to
  // `pathname`: navigating changes the route, so the menu closes itself during
  // render with no cascading state update.
  const [openedAt, setOpenedAt] = useState<string | null>(null);
  const open = openedAt === pathname;

  return (
    <header className="sticky top-0 z-50 border-b border-line bg-ink/80 backdrop-blur-xl">
      <div className="mx-auto flex h-16 max-w-7xl items-center gap-6 px-6 md:px-10">
        <Link href="/" className="group flex items-baseline gap-2">
          <span className="display-narrow text-xl font-extrabold tracking-tight text-chalk">
            GUML
          </span>
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
                className={cn(
                  "rounded-full px-3 py-1.5 text-sm transition-colors",
                  active ? "text-chalk" : "text-fog hover:text-chalk",
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
            className="hidden rounded-full bg-chalk px-4 py-1.5 text-sm font-medium text-ink transition-colors hover:opacity-90 md:inline-flex"
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
        <div className="border-t border-line bg-ink px-6 pb-8 pt-4 md:hidden">
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
