"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { NAV } from "@/lib/nav";
import { cn } from "@/lib/utils";

export function DocsSidebar() {
  const pathname = usePathname();

  return (
    <nav aria-label="Documentation" className="space-y-8">
      {NAV.map((group) => (
        <div key={group.title}>
          <p className="label mb-3">{group.title}</p>
          <ul className="space-y-0.5 border-l border-white/8">
            {group.items.map((item) => {
              const active = pathname === item.href;
              return (
                <li key={item.href}>
                  <Link
                    href={item.href}
                    aria-current={active ? "page" : undefined}
                    className={cn(
                      "-ml-px flex items-center gap-2 border-l py-1.5 pl-4 text-sm transition-colors",
                      active
                        ? "border-ember font-medium text-chalk"
                        : "border-transparent text-fog hover:border-white/25 hover:text-chalk",
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
      ))}
    </nav>
  );
}
