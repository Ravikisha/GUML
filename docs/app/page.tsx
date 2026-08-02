import { ArrowRight, Check, TriangleAlert } from "lucide-react";
import Link from "next/link";
import { CodeCompare } from "@/components/code-compare";
import { Marquee, NumberTicker, Reveal } from "@/components/motion-bits";
import { TokenDial } from "@/components/token-dial";
import { Badge, ButtonLink, Panel, Section } from "@/components/ui";
import { FIXTURES } from "@/lib/fixtures.generated";
import { commas, reduction } from "@/lib/utils";

const tasks = FIXTURES.find((f) => f.id === "tasks")!;

/** Only the four numbers the dial reads, so the fixtures' source strings stay out of the client
    bundle — `FIXTURES` carries every fixture's React, GUML and JSON listing. */
const DIAL = FIXTURES.map((f) => ({
  id: f.id,
  title: f.title,
  guml: f.tokens.guml,
  react: f.tokens.react,
})).sort((a, b) => a.react - b.react);

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
      {/* ----------------------------------------------------------------- hero
          No dot grid, no spotlight, no card. The page is paper, and it separates its parts with
          whitespace — a decorative field behind the headline, or a border around the dial, would be
          the first thing contradicting that. What fills the space instead is the measurement itself:
          one ring, burning from a React baseline down to what GUML costs. */}
      <div className="relative mx-auto max-w-(--container-page) px-6 pt-16 pb-24 md:px-10 lg:pt-24 lg:pb-32">
        <p className="label">measured · cl100k_base · hand-authored fixtures</p>

        {/* Full-bleed, because display type at this size needs the whole measure. Inside a column it
            wrapped mid-word, which is the difference between editorial scale and a headline that has
            simply outgrown its container. Weight 500, 0.9 leading: large, not heavy. */}
        <h1 className="display-wide mt-8 text-hero text-chalk">
          Write less.
          <br />
          Ship the same app.
        </h1>

        {/* The dial takes the wider column: it is the argument, and the prose is its caption. On a
            phone the order flips — the ring comes first, because a 480px circle above the fold says
            what two paragraphs take longer to. */}
        <div className="mt-14 grid items-center gap-14 lg:mt-20 lg:grid-cols-[0.9fr_1.1fr] lg:gap-16">
          <div className="order-2 lg:order-1">
            <p className="max-w-xl text-subheading leading-snug text-fog">
              GUML is what a model emits instead of React. Twenty-five lines of markup compile to a
              working task app — fetch, optimistic updates, rollback, loading and empty states,
              accessible labels — none of which the model writes, and none of which it can get wrong.
            </p>

            <div className="mt-10 flex flex-wrap items-center gap-6">
              <ButtonLink href="/docs/quickstart" size="lg">
                Start in 60 seconds
                <ArrowRight className="size-4" />
              </ButtonLink>
              {/* A ghost text link, not a second filled button. One principal action per screen. */}
              <Link
                href="/research/measurements"
                className="tracked group inline-flex items-center gap-1.5 rounded-chip py-2 text-body-sm text-ember transition-colors hover:text-chalk"
              >
                Read the measurements
                <ArrowRight className="size-3.5 transition-transform group-hover:translate-x-0.5" />
              </Link>
            </div>

            <p className="mt-10 font-mono text-xs text-fog-dim">
              Rust compiler · 396 tests · React, Svelte, static-HTML and JSON backends
            </p>
          </div>

          <TokenDial fixtures={DIAL} initial="tasks" className="order-1 lg:order-2" />
        </div>
      </div>

      {/* ------------------------------------------------------- the comparison */}
      <Section
        meter={{ label: "same app, three representations", value: "one is 8.1× smaller", tone: "iris" }}
      >
        <div className="grid gap-10 lg:grid-cols-[0.85fr_1.15fr] lg:items-start">
          <div className="lg:sticky lg:top-24">
            <h2 className="display-narrow text-heading font-medium">
              The model writes the intent. The compiler writes the rest.
            </h2>
            <p className="mt-5 text-fog">
              Every tab builds the same task list. Only the first is small enough for a model to
              produce in a couple of seconds — and only the first makes an unknown component a
              compile error instead of a runtime surprise.
            </p>
            <p className="mt-4 text-sm text-fog-dim">
              The JSON tab is an A2UI-shaped UI spec: the same idea, 45% more tokens, because quotes,
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
                note: "A2UI-shaped JSON IR — 324 tokens minified",
              },
            ]}
            preview="guml"
          />
        </div>
      </Section>

      {/* ---------------------------------------------------------- conventions */}
      <Section meter={{ label: "written by the compiler", value: "0 tokens", tone: "mint" }}>
        <h2 className="display-narrow max-w-3xl text-heading font-medium">
          Convention is compression.
        </h2>
        <p className="mt-5 max-w-2xl text-fog">
          Anything a model would otherwise have to remember, the compiler does instead. That removes
          the tokens and the failure mode in the same move.
        </p>

        <ul className="mt-12 grid gap-px overflow-hidden rounded-panel border border-line bg-chalk/8 sm:grid-cols-2 lg:grid-cols-3">
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
          <h2 className="display-narrow text-heading font-medium">
            Numbers, with their caveats attached.
          </h2>
          <p className="text-fog">
            Compression is bounded by prose: a landing page floors out because the copy <em>is</em>{" "}
            the payload. Structure-heavy screens approach 8×; content-heavy pages settle near 4×.
            One blended average would hide that, so this site never reports one.
          </p>
        </div>

        <div className="mt-12 overflow-x-auto rounded-panel border border-line">
          <table className="w-full min-w-[34rem] text-left">
            <thead>
              <tr className="border-b border-line bg-chalk/[0.02]">
                {["fixture", "React", "GUML", "reduction"].map((h) => (
                  <th key={h} className="label px-5 py-3 font-normal">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="font-mono text-sm">
              {FIXTURES.map((f) => (
                <tr key={f.id} className="border-b border-line last:border-0">
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
          <Link href="/research" className="text-iris underline decoration-iris/40">
            still an open question
          </Link>
          .
        </p>
      </Section>

      {/* --------------------------------------------------------------- honest */}
      <Section meter={{ label: "risk register", value: "unresolved", tone: "ember" }}>
        <div className="grid gap-10 lg:grid-cols-2 lg:items-start">
          <div>
            <h2 className="display-narrow text-heading font-medium">
              Why this might not work.
            </h2>
            <p className="mt-5 text-fog">
              GUML has zero training data by construction, and the literature on low-resource
              languages consistently finds that models are worse at languages they have never seen.
              One paper points the other way: a constrained DSL beating Python by 40 points on
              multi-step tasks it had never encountered. Both findings are well supported.
            </p>
            <p className="mt-4 text-fog">
              Reconciling them is the open question, and it needs a controlled comparison with human
              grading that has not been run. The compiler does not depend on the answer — it either
              lowers a construct correctly or it does not, and tests settle that. The claim about model
              behaviour is the part that stays labelled.
            </p>
            <ButtonLink href="/research" variant="outline" className="mt-7">
              What is and is not measured
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
      <div className="border-t border-line py-6">
        <Marquee>
          {DIAGNOSTICS.map((d) => (
            <span
              key={d}
              className="rounded-full border border-line px-4 py-1.5 font-mono text-[0.72rem] whitespace-nowrap text-fog-dim"
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
        <h2 className="display-narrow mx-auto max-w-2xl text-heading font-medium">
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
