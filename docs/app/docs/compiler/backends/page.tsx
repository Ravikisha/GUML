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
      meter={{ label: "shipping", value: "3 of 7" }}
      title="Backends"
      lede="A backend turns the AST into source text. Three exist — React, a JSON UI tree, and static HTML — and they share one design-system table, which is what makes “GUML is an IR” a claim about the language rather than about one emitter."
      toc={[
        { id: "coverage", title: "What v0.1 covers" },
        { id: "example", title: "Source and output" },
        { id: "html", title: "Static HTML" },
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
          ["Controls", "lowered", "btn link check input toggle select, with explicit type= on buttons"],
          ["state", "lowered", "useState, a derived setter name, and a union type from an enumerated domain"],
          ["Actions", "lowered", "x++ x-- x=expr, ;-sequencing, and resource mutations"],
          [
            "Bindings",
            "lowered",
            "parsed to an expression tree and type-checked; syntax outside the grammar is GUML0023, never forwarded",
          ],
          ["Modifiers", "lowered", "the design-system table"],
          ["Anchors, routes", "lowered", "id= and href="],
          ["aria=", "lowered", "becomes aria-label; a missing accessible name is an error, not a lint"],
          [
            "data resources",
            "lowered",
            "fetch on mount with AbortController cancellation, loading / empty / error slots, optimistic apply and rollback",
          ],
          ["list / table", "lowered", "keyed map, where= filtering via useMemo, aggregates like tasks.open.count"],
          ["form / tabs / faq / tier", "lowered", "submit wiring with a pending flag, segmented control from a domain, faq as <details>"],
          ["js / raw blocks", "lowered", "emitted verbatim; reported as GUML0090 so the escape-hatch rate is countable"],
          ["Source maps", "lowered", "every declaration and element, nested ones included"],
          ["Retry, backoff, response cache", "not yet", "the resource layer stops at cancellation"],
          ["def user components", "not yet", "no user-defined tags"],
        ]}
      />

      <H2 id="example">Source and output</H2>
      <P>Same fixture, both sides — the second tab is what the compiler actually wrote.</P>
      <CodeCompare
        baseline="react"
        preview="guml"
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
        A construct a backend cannot lower correctly produces a warning and a visible marker in the
        output — never approximate code. The message names the backend, because the same construct can
        be a temporary gap in one and an architectural impossibility in another.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml build fixtures/a.guml --backend html

warning[GUML0030]: \`html\` backend: \`btn Decrement\` has an action, and the \`html\`
                   backend emits no JavaScript — rendered disabled
  --> fixtures/a.guml:9:5
   = help: the emitted markup marks the gap with \`data-guml-inert\`; a construct that
           needs a runtime cannot work in a no-JavaScript backend`}
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

      <H2 id="html">The static HTML backend</H2>
      <P>
        <C>--backend html</C> emits one <C>.html</C> file with no JavaScript. For a content-heavy page
        that is not a degraded output, it is the right one: a landing page needs no runtime, and the
        landing fixture lowers completely — including <C>faq</C>, because{" "}
        <C>&lt;details&gt;</C>/<C>&lt;summary&gt;</C> is interactive without script.
      </P>
      <P>
        What makes it evidence for the IR claim is that it shares <C>classes()</C> with the React
        backend. The same GUML produces the same class strings from both, so presentation belongs to
        the compiler; a second table would have made the agreement a coincidence, and a test holds
        them to it.
      </P>
      <P>
        Everything that needs a runtime is <em>reported and marked</em>, never dropped quietly:
      </P>
      <Table
        head={["construct", "what the html backend does"]}
        rows={[
          [
            <C key="a">state</C>,
            "renders the initial value; warns once that nothing will update it",
          ],
          [
            <C key="b">data</C>,
            <>
              nothing is fetched, so repeaters render their own <C key="c">empty</C> message — the
              state a first-time visitor actually sees
            </>,
          ],
          [
            "an action",
            <>
              the control renders <C key="d">disabled</C> with{" "}
              <C key="e">data-guml-inert</C>, and the dropped action is named in a warning
            </>,
          ],
          [<C key="f">tabs</C>, "not emitted: it switches state, and there is no state"],
          [
            <>a binding like <C key="g">{"{count}"}</C></>,
            "the declared initial value where one is knowable, an em dash and a warning where it is not",
          ],
          [<C key="h">js</C>, "dropped, with a warning — this backend emits no JavaScript"],
        ]}
      />
      <Note tone="warn" title="“Not yet” and “not ever” are different messages">
        <p>
          In the React backend an unlowered construct is a gap to be filled. Here it is architectural:
          there will never be an <C>onClick</C> in a file with no script. The diagnostic names which
          backend is speaking, so it does not tell a reader to wait for something that is not coming.
        </p>
      </Note>
      <P>
        Prose is also escaped here for the first time in the pipeline. GUML prose is never quoted —
        that is why it costs almost nothing in tokens — so this backend is the first thing that has to
        turn <C>&amp;</C> and <C>&lt;</C> into entities.
      </P>

      <H2 id="planned">Planned backends</H2>
      <Table
        head={["target", "why it exists", "phase"]}
        rows={[
          ["React + TS + Tailwind", "ecosystem gravity; easiest hand-off to a human; benchmark baseline", "shipping"],
          [
            "JSON UI tree",
            "what the browser runtime and the playground render; base for the A2UI emitter",
            "shipping",
          ],
          [
            "Static HTML",
            "no JavaScript at all: the right output for a content page, and the proof that presentation belongs to the compiler rather than to a backend",
            "shipping",
          ],
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
