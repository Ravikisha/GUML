import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { CodeCompare } from "@/components/code-compare";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";
import { FIXTURES } from "@/lib/fixtures.generated";

export const metadata: Metadata = {
  title: "Backends",
  description: "What the React backend covers today, what it refuses to guess, and what is planned.",
};

const counter = FIXTURES.find((f) => f.id === "counter")!;

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/backends"
      meter={{ label: "shipping", value: "1 of 6", tone: "ember" }}
      title="Backends"
      lede="A backend turns the AST into source text. React is the one that works; the others are planned, and the gaps in the working one are reported rather than guessed."
      toc={[
        { id: "coverage", title: "What v0.1 covers" },
        { id: "example", title: "Source and output" },
        { id: "gaps", title: "What it refuses to guess" },
        { id: "design-system", title: "The design-system table" },
        { id: "planned", title: "Planned backends" },
        { id: "writing", title: "Writing one" },
      ]}
    >
      <H2 id="coverage">What v0.1 covers</H2>
      <Table
        head={["construct", "status", "notes"]}
        rows={[
          ["Containers", "lowered", "card row col section nav hero footer"],
          ["Text", "lowered", "h h1 h2 p text metric head empty; prose verbatim"],
          ["Controls", "lowered", "btn link check input, with explicit type= on buttons"],
          ["state", "lowered", "useState plus a derived setter name"],
          ["Actions", "lowered", "x++ x-- x=expr and ;-sequencing"],
          ["Bindings", "pass-through", "forwarded to JSX; no field checking yet"],
          ["Modifiers", "lowered", "the design-system table"],
          ["Anchors, routes", "lowered", "id= and href="],
          ["aria=", "lowered", "becomes aria-label"],
          ["data resources", "reported", "warns; needs the desugar pass"],
          ["list / table", "reported", "warns; needs the desugar pass"],
          ["form / tabs / faq / tier", "reported", "warns; needs the desugar pass"],
        ]}
      />

      <H2 id="example">Source and output</H2>
      <P>Same fixture, both sides — the second tab is what the compiler actually wrote.</P>
      <CodeCompare
        baseline="react"
        panes={[
          {
            id: "guml",
            label: "counter.guml",
            lang: "guml",
            code: counter.guml,
            tokens: counter.tokens.guml,
            note: "the source",
          },
          ...(counter.emitted
            ? [
                {
                  id: "react",
                  label: "Counter.tsx",
                  lang: "tsx" as const,
                  code: counter.emitted,
                  tokens: counter.tokens.react,
                  note: "emitted by the React backend",
                },
              ]
            : []),
        ]}
      />

      <H2 id="gaps">What it refuses to guess</H2>
      <P>
        A construct the backend cannot lower correctly produces a warning and a TODO in the output —
        never approximate code.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo run -q -p guml-cli -- build fixtures/b.guml

warning[GUML0030]: v0.1 React backend does not yet lower resource \`tasks\`
  --> fixtures/b.guml:4:1
   = help: tracked in ROADMAP.md Phase 3; the emitted file marks the gap with a TODO`}
      />
      <Note tone="info" title="Why warn instead of approximating">
        <p>
          The project&rsquo;s central claim is about reliability. A compiler that quietly emits
          plausible-but-wrong fetch logic would destroy that claim far more effectively than one that
          admits a gap. An honest partial compiler is useful; a quietly wrong one is worse than none.
        </p>
      </Note>

      <H2 id="design-system">The design-system table</H2>
      <P>
        One function in the React backend maps modifiers to classes. Every string in it is a token the
        model no longer emits, and a presentational decision it cannot get wrong.
      </P>
      <CodeBlock
        lang="tsx"
        filename="crates/guml-codegen/src/react.rs"
        code={`"btn" => {
    c.push("rounded-md px-4 py-2 text-sm font-medium transition-colors");
    if has("primary")      { c.push("bg-slate-900 text-white hover:bg-slate-800"); }
    else if has("outline") { c.push("border border-slate-300 text-slate-700 hover:bg-slate-50"); }
    else if has("quiet")   { c.push("text-slate-500 hover:text-slate-900"); }
    else if has("danger")  { c.push("bg-red-600 text-white hover:bg-red-700"); }
    else                   { c.push("border border-slate-300 text-slate-700 hover:bg-slate-50"); }
    c.push("disabled:opacity-40");
}`}
      />
      <P>
        Swapping that table re-themes every page compiled with it — which is how a design system stops
        being a request in a prompt and becomes a guarantee. See{" "}
        <A href="/docs/language/modifiers">modifiers</A>.
      </P>

      <H2 id="planned">Planned backends</H2>
      <Table
        head={["target", "why it exists", "phase"]}
        rows={[
          ["React + TS + Tailwind", "ecosystem gravity; easiest hand-off to a human; benchmark baseline", "shipping"],
          ["Static HTML/CSS/JS", "best Lighthouse numbers for the benchmark", "1.5"],
          ["Svelte", "the compile-away-the-framework and bundle-size story", "7"],
          ["Web Components", "portability and embedding", "7"],
          ["A2UI", "emit into Google's agent-UI format", "7"],
          ["MCP-UI", "emit into the MCP UI resource format", "7"],
        ]}
      />
      <P>
        The A2UI and MCP-UI emitters are strategic as much as technical: they turn the closest
        standards competitor into a distribution channel and a benchmark baseline. See{" "}
        <A href="/docs/research/prior-art">prior art</A>.
      </P>

      <H2 id="writing">Writing one</H2>
      <CodeBlock
        lang="tsx"
        filename="crates/guml-codegen/src/lib.rs"
        code={`pub trait Backend {
    fn name(&self) -> &'static str;
    fn emit(&self, program: &Program) -> Emitted;
}

pub struct Emitted {
    pub files: Vec<OutFile>,
    pub diagnostics: Diagnostics,
}`}
      />
      <P>That is the whole contract. Requirements for a new backend:</P>
      <UL>
        <LI>Emitted code must be idiomatic and hand-editable — developers rate it in the benchmark&rsquo;s human study.</LI>
        <LI>
          Call <C>unsupported()</C> for anything you cannot lower, and leave a TODO in the output.
        </LI>
        <LI>Snapshot tests, plus a Playwright test if the construct is interactive.</LI>
        <LI>Zero axe-core violations on the emitted result.</LI>
        <LI>No shared mutable state between backends.</LI>
      </UL>
    </DocPage>
  );
}
