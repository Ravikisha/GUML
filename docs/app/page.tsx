import { ArrowRight, Check, TriangleAlert } from "lucide-react";
import Link from "next/link";
import { CodeCompare } from "@/components/code-compare";
import { Compressor } from "@/components/compressor";
import { DotGrid, Marquee, NumberTicker, Reveal, Spotlight } from "@/components/motion-bits";
import { Badge, ButtonLink, Meter, Panel, Section } from "@/components/ui";
import { FIXTURES } from "@/lib/fixtures.generated";
import { commas, reduction } from "@/lib/utils";

const tasks = FIXTURES.find((f) => f.id === "tasks")!;

/** What the compiler supplies, so a model never spends tokens on it. */
const CONVENTIONS: Array<[string, string]> = [
  ["fetch + cancellation", "declared once as a resource instead of written per screen"],
  ["loading, empty, error", "generated for every resource; `empty` takes your message"],
  ["optimistic + rollback", "one word — `optimistic:prepend` — snapshots and reverts on failure"],
  ["accessible names", "a control without a label is a compile error, not a lint warning"],
  ["the design system", "modifiers like `primary` map to tokens the compiler owns"],
  ["derived values", "`{tasks.open.count}` is a binding, so there is no stale memo to get wrong"],
];

const DIAGNOSTICS = [
  "GUML0001 tabs are not allowed for indentation",
  "GUML0003 unterminated `{` group",
  "GUML0011 unexpected indentation",
  "GUML0020 expected a tag name",
  "GUML0030 unknown tag `buton` — did you mean `btn`?",
  "GUML0033 unknown state",
  "GUML0050 icon control without a label",
  "GUML0051 input without a label",
];

export default function HomePage() {
  return (
    <>
      {/* ---------------------------------------------------------------- hero */}
      <div className="relative overflow-hidden">
        <DotGrid />
        <Spotlight />
        <div className="relative mx-auto grid max-w-7xl items-center gap-14 px-6 pt-16 pb-24 md:px-10 lg:grid-cols-[1.05fr_1fr] lg:pt-24">
          <div>
            <Meter label="task crud · react → guml" value="1,434 → 175 tokens" tone="mint" />

            <h1
              data-compress-headline
              className="mt-7 text-hero leading-[0.86] font-extrabold tracking-[-0.03em] text-chalk"
              style={{
                fontFamily: "var(--font-display)",
                // The Compressor tweens --wdth from 100 to 76 on load, so the
                // headline narrows while the token counter falls.
                fontVariationSettings: '"wdth" var(--wdth, 100), "opsz" 48',
              }}
            >
              Write less.
              <br />
              Ship the same app.
            </h1>

            <p className="mt-7 max-w-xl text-lg leading-relaxed text-fog">
              GUML is what a model emits instead of React. Twenty-four lines of markup compile to a
              working task app — fetch, optimistic updates, rollback, loading and empty states,
              accessible labels — none of which the model has to write, and none of which it can get
              wrong.
            </p>

            <div className="mt-9 flex flex-wrap items-center gap-3">
              <ButtonLink href="/docs/quickstart" size="lg">
                Start in 60 seconds
                <ArrowRight className="size-4" />
              </ButtonLink>
              <ButtonLink href="/docs/research/measurements" variant="outline" size="lg">
                Read the measurements
              </ButtonLink>
            </div>

            <p className="mt-6 font-mono text-xs text-fog-dim">
              Rust compiler · 49 tests · React backend shipping, Svelte and Web Components planned
            </p>
          </div>

          <Panel className="p-5 md:p-6">
            <Compressor />
          </Panel>
        </div>
      </div>

      {/* ------------------------------------------------------- the comparison */}
      <Section
        meter={{ label: "same app, three representations", value: "one is 8.2× smaller", tone: "iris" }}
      >
        <div className="grid gap-10 lg:grid-cols-[0.85fr_1.15fr] lg:items-start">
          <div className="lg:sticky lg:top-24">
            <h2 className="display-narrow text-display font-bold leading-[0.95] tracking-tight">
              The model writes the intent. The compiler writes the rest.
            </h2>
            <p className="mt-5 text-fog">
              Every tab builds the same task list. Only the first is small enough for a model to
              produce in a couple of seconds — and only the first makes an unknown component a
              compile error instead of a runtime surprise.
            </p>
            <p className="mt-4 text-sm text-fog-dim">
              The JSON tab is an A2UI-shaped UI spec: the same idea, 44% more tokens, because quotes,
              braces and repeated keys are not free.
            </p>
            <Link
              href="/examples"
              className="mt-6 inline-flex items-center gap-1.5 font-mono text-sm text-iris hover:text-chalk"
            >
              All examples <ArrowRight className="size-3.5" />
            </Link>
          </div>

          <CodeCompare
            baseline="react"
            panes={[
              {
                id: "guml",
                label: "tasks.guml",
                lang: "guml",
                code: tasks.guml,
                tokens: tasks.tokens.guml,
                note: "what the model emits",
              },
              {
                id: "react",
                label: "TaskList.tsx",
                lang: "tsx",
                code: tasks.react,
                tokens: tasks.tokens.react,
                note: "what it stands in for",
              },
              {
                id: "json",
                label: "spec.json",
                lang: "json",
                code: tasks.json ?? "",
                tokens: tasks.tokens.json,
                note: "A2UI-shaped JSON IR — 315 tokens minified",
              },
            ]}
          />
        </div>
      </Section>

      {/* ---------------------------------------------------------- conventions */}
      <Section meter={{ label: "written by the compiler", value: "0 tokens", tone: "mint" }}>
        <h2 className="display-narrow max-w-3xl text-display font-bold leading-[0.95] tracking-tight">
          Convention is compression.
        </h2>
        <p className="mt-5 max-w-2xl text-fog">
          Anything a model would otherwise have to remember, the compiler does instead. That removes
          the tokens and the failure mode in the same move.
        </p>

        <ul className="mt-12 grid gap-px overflow-hidden rounded-panel border border-white/8 bg-white/8 sm:grid-cols-2 lg:grid-cols-3">
          {CONVENTIONS.map(([title, detail], i) => (
            <li key={title} className="bg-ink p-6">
              <Reveal delay={i * 0.04}>
                <Check className="mb-4 size-4 text-mint" />
                <p className="font-mono text-sm text-chalk">{title}</p>
                <p className="mt-2 text-sm leading-relaxed text-fog">{detail}</p>
              </Reveal>
            </li>
          ))}
        </ul>
      </Section>

      {/* -------------------------------------------------------------- numbers */}
      <Section meter={{ label: "measured · cl100k_base", value: "3 hand-authored fixtures" }}>
        <div className="grid gap-10 lg:grid-cols-[0.8fr_1.2fr] lg:items-end">
          <h2 className="display-narrow text-display font-bold leading-[0.95] tracking-tight">
            Numbers, with their caveats attached.
          </h2>
          <p className="text-fog">
            Compression is bounded by prose: a landing page floors out because the copy <em>is</em>{" "}
            the payload. Structure-heavy screens approach 8×; content-heavy pages settle near 4×.
            One blended average would hide that, so this site never reports one.
          </p>
        </div>

        <div className="mt-12 overflow-x-auto rounded-panel border border-white/8">
          <table className="w-full min-w-[34rem] text-left">
            <thead>
              <tr className="border-b border-white/8 bg-white/[0.02]">
                {["fixture", "React", "GUML", "reduction"].map((h) => (
                  <th key={h} className="label px-5 py-3 font-normal">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="font-mono text-sm">
              {FIXTURES.map((f) => (
                <tr key={f.id} className="border-b border-white/8 last:border-0">
                  <td className="px-5 py-4">
                    <span className="text-chalk">{f.title}</span>
                    <Badge className="ml-3" tone={f.category === "structure" ? "iris" : "neutral"}>
                      {f.category}
                    </Badge>
                  </td>
                  <td className="px-5 py-4 tabular-nums text-ember">{commas(f.tokens.react)}</td>
                  <td className="px-5 py-4 tabular-nums text-iris">{commas(f.tokens.guml)}</td>
                  <td className="px-5 py-4 tabular-nums text-mint">
                    <NumberTicker value={reduction(f.tokens.react, f.tokens.guml)} />%
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <p className="mt-4 max-w-3xl font-mono text-xs leading-relaxed text-fog-dim">
          Both sides were authored by the same person, and these are authored artifacts rather than
          model generations. Whether a model can actually produce correct GUML is{" "}
          <Link href="/docs/research/phase0" className="text-iris underline decoration-iris/40">
            still an open question
          </Link>
          .
        </p>
      </Section>

      {/* --------------------------------------------------------------- honest */}
      <Section meter={{ label: "risk register", value: "unresolved", tone: "ember" }}>
        <div className="grid gap-10 lg:grid-cols-2 lg:items-start">
          <div>
            <h2 className="display-narrow text-display font-bold leading-[0.95] tracking-tight">
              Why this might not work.
            </h2>
            <p className="mt-5 text-fog">
              GUML has zero training data by construction, and the literature on low-resource
              languages consistently finds that models are worse at languages they have never seen.
              One paper points the other way: a constrained DSL beating Python by 40 points on
              multi-step tasks it had never encountered. Both findings are well supported.
            </p>
            <p className="mt-4 text-fog">
              Reconciling them is the real research question. A two-week experiment decides whether
              the rest of this project is worth building.
            </p>
            <ButtonLink href="/docs/research/phase0" variant="outline" className="mt-7">
              The Phase 0 gate
              <ArrowRight className="size-4" />
            </ButtonLink>
          </div>

          <Panel className="border-ember/25 bg-ember/[0.04] p-6">
            <TriangleAlert className="mb-4 size-4 text-ember" />
            <ul className="space-y-4 text-sm leading-relaxed text-fog">
              <li>
                <span className="font-mono text-chalk">Out-of-distribution penalty.</span> A model
                fluent in React may simply be worse at GUML, whatever the token count says.
              </li>
              <li>
                <span className="font-mono text-chalk">Capability threshold.</span> If the win shows
                up only on small models, this is a cost optimisation, not a reliability one.
              </li>
              <li>
                <span className="font-mono text-chalk">Diff-based editing.</span> Agents patch files
                rather than rewriting them, which already recovers much of the saving.
              </li>
              <li>
                <span className="font-mono text-chalk">The expressiveness cliff.</span> Every escape
                hatch spends the compression back. The rate gets tracked and published.
              </li>
            </ul>
          </Panel>
        </div>
      </Section>

      {/* ---------------------------------------------------- diagnostics ribbon */}
      <div className="border-t border-white/8 py-6">
        <Marquee>
          {DIAGNOSTICS.map((d) => (
            <span
              key={d}
              className="rounded-full border border-white/8 px-4 py-1.5 font-mono text-[0.72rem] whitespace-nowrap text-fog-dim"
            >
              {d}
            </span>
          ))}
        </Marquee>
        <p className="mx-auto mt-5 max-w-2xl px-6 text-center font-mono text-[0.7rem] leading-relaxed text-fog-dim md:px-10">
          Every diagnostic carries a span, a fix, and — where the fix is unambiguous — a replacement
          string the repair loop applies without another model call.
        </p>
      </div>

      {/* ------------------------------------------------------------------ cta */}
      <Section className="text-center">
        <h2 className="display-narrow mx-auto max-w-2xl text-display font-bold leading-[0.95] tracking-tight">
          Eleven lines is a working counter.
        </h2>
        <p className="mx-auto mt-5 max-w-lg text-fog">
          Clone it, run <span className="font-mono text-chalk">cargo test</span>, compile a fixture,
          read the emitted React. The whole loop takes a minute.
        </p>
        <div className="mt-9 flex flex-wrap justify-center gap-3">
          <ButtonLink href="/docs/quickstart" size="lg">
            Quickstart
            <ArrowRight className="size-4" />
          </ButtonLink>
          <ButtonLink href="/docs/language/syntax" variant="outline" size="lg">
            Language reference
          </ButtonLink>
        </div>
      </Section>
    </>
  );
}
