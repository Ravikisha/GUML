import Link from "next/link";
import { NAV } from "@/lib/nav";

export function SiteFooter() {
  return (
    <footer className="border-t border-line px-6 py-14 md:px-10">
      <div className="mx-auto grid max-w-7xl gap-10 md:grid-cols-[1.4fr_repeat(4,1fr)]">
        <div>
          <p className="display-narrow text-2xl font-extrabold text-chalk">GUML</p>
          <p className="mt-3 max-w-xs text-sm leading-relaxed text-fog">
            An intermediate representation and compiler for LLM-generated interfaces. Apache-2.0.
          </p>
          <p className="mt-6 font-mono text-[0.7rem] leading-relaxed text-fog-dim">
            Pre-Phase-0. The compiler front end works and is tested; the research question it
            exists to answer is still open.
          </p>
        </div>

        {NAV.map((group) => (
          <div key={group.title}>
            <p className="label mb-3">{group.title}</p>
            <ul className="space-y-2">
              {group.items.map((item) => (
                <li key={item.href}>
                  <Link href={item.href} className="text-sm text-fog transition-colors hover:text-chalk">
                    {item.title}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>

      <div className="mx-auto mt-12 flex max-w-7xl flex-wrap items-center justify-between gap-3 border-t border-line pt-6">
        <p className="font-mono text-[0.7rem] text-fog-dim">
          Token figures measured with cl100k_base on hand-authored fixtures. Both sides written by
          the same author.
        </p>
        <p className="font-mono text-[0.7rem] text-fog-dim">
          Built with the compiler&rsquo;s own lexer rules
        </p>
      </div>
    </footer>
  );
}
