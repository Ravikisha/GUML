import type { Metadata } from "next";
import { Playground, type Sample } from "@/components/playground";
import { Meter, Section } from "@/components/ui";
import { A, C } from "@/components/prose";
import { FIXTURES } from "@/lib/fixtures.generated";

export const metadata: Metadata = {
  title: "Playground",
  description:
    "Write GUML and watch the real compiler answer — diagnostics, emitted React, and a live preview, all running in your browser via WebAssembly.",
};

const SAMPLES: Sample[] = [
  {
    id: "counter",
    label: "counter",
    source: FIXTURES.find((f) => f.id === "counter")!.guml,
  },
  {
    id: "tasks",
    label: "task crud",
    source: FIXTURES.find((f) => f.id === "tasks")!.guml,
  },
  {
    id: "landing",
    label: "landing",
    source: FIXTURES.find((f) => f.id === "landing")!.guml,
  },
  {
    id: "broken",
    label: "broken (on purpose)",
    source: `page Broken
state count=0

crad sm center
  h Clicks
  metric {kount}
  btn quiet >count++
`,
  },
  {
    id: "blank",
    label: "blank",
    source: `page Scratch
state count=0

card sm center
  h Title
  p Write something here.
  btn Go primary >count++
`,
  },
];

export default function Page() {
  return (
    <>
      <div className="border-b border-line px-6 pt-14 pb-10 md:px-10">
        <div className="mx-auto max-w-(--container-page)">
          <Meter label="compiler" value="rust → wasm · 787 KB" tone="iris" />
          <h1 className="display-narrow mt-6 text-heading text-chalk">
            Playground
          </h1>
          <p className="mt-6 max-w-2xl text-lg leading-relaxed text-fog">
            The same Rust compiler the CLI runs, built to WebAssembly and loaded in this page.
            Diagnostics are the real ones, the emitted React is byte-for-byte what{" "}
            <C>guml build</C> writes, and the preview is rendered from the compiler&rsquo;s own UI
            tree — so it cannot drift from the code.
          </p>
          <p className="mt-4 max-w-2xl text-sm text-fog-dim">
            Try the <strong className="text-fog">broken</strong> sample: two mistakes, both caught in
            one pass, one of them fixable without asking a model. Then read{" "}
            <A href="/docs/compiler/diagnostics">why diagnostics are shaped that way</A>.
          </p>
        </div>
      </div>

      <Section className="border-t-0">
        <Playground samples={SAMPLES} />
      </Section>

      <Section meter={{ label: "what runs here", value: "and what does not", tone: "ember" }}>
        <div className="grid gap-8 md:grid-cols-2">
          <div>
            <h2 className="display-narrow text-2xl font-medium text-chalk">
              Honest about the runtime
            </h2>
            <p className="mt-4 leading-relaxed text-fog">
              The preview uses a small React runtime that walks the compiler&rsquo;s UI tree. It
              covers containers, text, controls, fields, state, actions and bindings, plus{" "}
              <C>list</C> over seeded rows. Bindings are evaluated by a tiny expression parser —
              never <C>eval</C> — which is what makes rendering a document from an untrusted agent
              defensible.
            </p>
            <p className="mt-4 leading-relaxed text-fog">
              Tags the compiler cannot lower yet render as a labelled gap rather than as
              approximate markup, matching how the CLI reports them.
            </p>
          </div>
          <ul className="space-y-3 font-mono text-sm text-fog">
            {[
              ["state, actions, bindings", true],
              ["containers, text, controls, fields", true],
              ["list over seeded or fetched rows", true],
              ["optimistic mutations with rollback", true],
              ["form, tabs, faq, tier", false],
              ["route, auth, js/raw escape hatches", false],
            ].map(([label, yes]) => (
              <li key={String(label)} className="flex items-center gap-3">
                <span
                  className={
                    yes
                      ? "size-1.5 shrink-0 rounded-full bg-mint"
                      : "size-1.5 shrink-0 rounded-full bg-ember"
                  }
                />
                <span className={yes ? "text-fog" : "text-fog-dim"}>{label}</span>
              </li>
            ))}
          </ul>
        </div>
      </Section>
    </>
  );
}
