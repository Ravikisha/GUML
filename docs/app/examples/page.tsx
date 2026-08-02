import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { CodeCompare } from "@/components/code-compare";
import { LivePreview } from "@/components/live-preview";
import { NumberTicker, Reveal } from "@/components/motion-bits";
import { Badge, ButtonLink, Meter, Section } from "@/components/ui";
import { FIXTURES } from "@/lib/fixtures.generated";
import { commas, reduction } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Examples",
  description:
    "Three fixtures, every representation side by side, with the token counts that back the project's claims.",
};

export default function Page() {
  const totalReact = FIXTURES.reduce((n, f) => n + f.tokens.react, 0);
  const totalGuml = FIXTURES.reduce((n, f) => n + f.tokens.guml, 0);

  return (
    <>
      <div className="px-6 pt-16 pb-8 md:px-10">
        <div className="mx-auto max-w-(--container-page)">
          <Meter
            label="fixtures"
            value={`${commas(totalReact)} → ${commas(totalGuml)} tokens`}
            tone="mint"
          />
          <h1 className="display-narrow mt-6 text-heading text-chalk">
            Examples
          </h1>
          <p className="mt-6 max-w-2xl text-lg leading-relaxed text-fog">
            These three files are not illustrations — they are the artifacts behind every number on
            this site. They live in <code className="font-mono text-chalk">fixtures/</code> in the
            repository, they are checked by CI, and the code below is generated from them rather than
            retyped.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <ButtonLink href="/research/measurements" variant="outline">
              How they were measured
              <ArrowRight className="size-4" />
            </ButtonLink>
            <ButtonLink href="/docs/language/syntax" variant="quiet">
              Language reference
            </ButtonLink>
          </div>
        </div>
      </div>

      {FIXTURES.map((f) => {
        const cut = reduction(f.tokens.react, f.tokens.guml);
        return (
          <Section
            key={f.id}
            id={f.id}
            meter={{
              label: `${f.category}-heavy`,
              value: `${cut}% fewer tokens`,
              tone: f.category === "structure" ? "mint" : "neutral",
            }}
          >
            <Reveal>
              <div className="grid gap-8 lg:grid-cols-[0.8fr_1.2fr] lg:items-start">
                <div className="lg:sticky lg:top-24">
                  {/* No `01 / 02 / 03` marker. These three fixtures are a set, not a sequence —
                      nothing about the counter comes before the task list — so a numeral would be
                      ordering information the content does not have. The quantity that *is* true of
                      each one is its token count, and that already has a place below. */}
                  <h2 className="display-narrow text-2xl font-medium text-chalk">{f.title}</h2>
                  <p className="mt-4 leading-relaxed text-fog">{f.blurb}</p>

                  <dl className="mt-7 space-y-3 font-mono text-sm">
                    <div className="flex items-baseline justify-between border-b border-line pb-2">
                      <dt className="text-fog-dim">React</dt>
                      <dd className="tabular-nums text-ember">{commas(f.tokens.react)}</dd>
                    </div>
                    <div className="flex items-baseline justify-between border-b border-line pb-2">
                      <dt className="text-fog-dim">GUML</dt>
                      <dd className="tabular-nums text-iris">{commas(f.tokens.guml)}</dd>
                    </div>
                    {f.tokens.json ? (
                      <div className="flex items-baseline justify-between border-b border-line pb-2">
                        <dt className="text-fog-dim">JSON IR</dt>
                        <dd className="tabular-nums text-fog">{commas(f.tokens.json)}</dd>
                      </div>
                    ) : null}
                    <div className="flex items-baseline justify-between">
                      <dt className="text-fog-dim">saved</dt>
                      <dd className="tabular-nums text-mint">
                        <NumberTicker value={cut} />%
                      </dd>
                    </div>
                  </dl>

                  {f.category === "content" ? (
                    <p className="mt-6 rounded-card border border-line p-4 text-sm leading-relaxed text-fog-dim">
                      232 of this page&rsquo;s 376 GUML tokens are the copy itself. Structural overhead
                      is 144 tokens against React&rsquo;s ~1,416 — the compression floor is the prose,
                      not the language.
                    </p>
                  ) : null}

                  {f.emitted ? (
                    <Badge tone="mint" className="mt-6">
                      compiles today
                    </Badge>
                  ) : (
                    <Badge tone="ember" className="mt-6">
                      parses · backend gap
                    </Badge>
                  )}
                </div>

                <CodeCompare
                  baseline="react"
                  maxHeight={520}
                  panes={[
                    {
                      id: "guml",
                      label: `${f.id}.guml`,
                      lang: "guml",
                      code: f.guml,
                      tokens: f.tokens.guml,
                      note: "the source",
                    },
                    {
                      id: "react",
                      label: "hand-written React",
                      lang: "tsx",
                      code: f.react,
                      tokens: f.tokens.react,
                      note: "the reference implementation it stands in for",
                    },
                    ...(f.emitted
                      ? [
                          {
                            id: "emitted",
                            label: "emitted",
                            lang: "tsx" as const,
                            code: f.emitted,
                            note: "what the compiler actually produced",
                          },
                        ]
                      : []),
                    ...(f.json
                      ? [
                          {
                            id: "json",
                            label: "JSON IR",
                            lang: "json" as const,
                            code: f.json,
                            tokens: f.tokens.json,
                            note: "A2UI-shaped spec — 324 tokens minified",
                          },
                        ]
                      : []),
                  ]}
                />
              </div>

              <div className="mt-6">
                <LivePreview
                  source={f.guml}
                  label={`live · ${f.id}.guml compiled in your browser`}
                />
              </div>
            </Reveal>
          </Section>
        );
      })}

      <Section className="text-center">
        <h2 className="display-narrow mx-auto max-w-xl text-heading font-medium">
          Run them yourself.
        </h2>
        <p className="mx-auto mt-5 max-w-lg text-fog">
          Every fixture is checked by CI on Linux and Windows, so a fixture that stops compiling is a
          broken claim rather than just a broken test.
        </p>
        <div className="mx-auto mt-8 max-w-md rounded-card border border-line bg-code code-surface p-4 text-left font-mono text-sm text-fog">
          <span className="text-fog-dim">$ </span>cargo run -q -p guml-cli -- build fixtures/a.guml
        </div>
        <div className="mt-8">
          <Link
            href="/docs/quickstart"
            className="inline-flex items-center gap-1.5 font-mono text-sm text-iris hover:text-chalk"
          >
            Quickstart <ArrowRight className="size-3.5" />
          </Link>
        </div>
      </Section>
    </>
  );
}
