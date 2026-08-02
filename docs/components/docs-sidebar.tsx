"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { NAV } from "@/lib/nav";
import { cn } from "@/lib/utils";

export function DocsSidebar() {
  const pathname = usePathname();

  return (
    <nav aria-label="Documentation" className="space-y-8">
      {NAV.map((group) => {
        // Which group you are reading in, so a sidebar of five identical grey labels tells you
        // where you are before you have to find the one orange line.
        const inGroup = group.items.some((item) => item.href === pathname);
        return (
        <div key={group.title}>
          <p className={cn("label mb-3 transition-colors", inGroup && "text-chalk")}>
            {group.title}
          </p>
          <ul className="space-y-0.5 border-l border-line">
            {group.items.map((item) => {
              const active = pathname === item.href;
              return (
                <li key={item.href}>
                  <Link
                    href={item.href}
                    aria-current={active ? "page" : undefined}
                    // Hover is an *orange* hairline, not a neutral one. The accent's job on this
                    // site is to mark position; a grey hover rule was the one place a pointer
                    // hovering a link and a link being current looked like different systems.
                    className={cn(
                      "-ml-px flex items-center gap-2 border-l py-1.5 pl-4 text-sm transition-colors",
                      active
                        ? "border-ember font-medium text-chalk"
                        : "border-transparent text-fog hover:border-ember/40 hover:text-chalk",
                    )}
                  >
                    {item.title}
                    {item.note ? (
                      <span className="rounded-full border border-ember/40 bg-ember/10 px-1.5 font-mono text-[0.6rem] text-ember">
                        {item.note}
                      </span>
                    ) : null}
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
        );
      })}
    </nav>
  );
}
