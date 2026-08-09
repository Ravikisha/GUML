import Link from "next/link";
import { NAV } from "@/lib/nav";

export function SiteFooter() {
  return (
    <footer className="border-t border-line px-6 py-14 md:px-10">
      <div className="mx-auto grid max-w-(--container-page) gap-10 md:grid-cols-[1.4fr_repeat(4,1fr)]">
        <div>
          <p className="display-narrow text-2xl font-medium text-chalk">GUML</p>
          <p className="mt-3 max-w-xs text-sm leading-relaxed text-fog">
            An intermediate representation and compiler for LLM-generated interfaces. MIT.
          </p>
          <p className="mt-6 font-mono text-[0.7rem] leading-relaxed text-fog-dim">
            The compiler works and is tested. Whether a constrained IR measurably improves what a
            model produces is a separate and open question, kept in{" "}
            <Link href="/research" className="underline decoration-fog-dim/50 hover:text-fog">
              research
            </Link>
            .
          </p>
          <p className="mt-4 flex gap-4 text-xs text-fog-dim">
            <Link href="/privacy" className="transition-colors hover:text-fog">
              Privacy
            </Link>
            <a
              href="https://github.com/guml-lang/guml"
              className="transition-colors hover:text-fog"
              target="_blank"
              rel="noreferrer"
            >
              GitHub
            </a>
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

      <div className="mx-auto mt-12 flex max-w-(--container-page) flex-wrap items-center justify-between gap-3 border-t border-line pt-6">
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
