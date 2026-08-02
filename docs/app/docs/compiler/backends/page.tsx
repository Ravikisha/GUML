import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { CodeCompare } from "@/components/code-compare";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";
import { FIXTURES } from "@/lib/fixtures.generated";

export const metadata: Metadata = {
  title: "Backends",
  description: "What the React backend covers, what it refuses to guess, and how the seven targets stay in agreement.",
};

const counter = FIXTURES.find((f) => f.id === "counter")!;

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/backends"
      meter={{ label: "shipping", value: "4 of 7" }}
      title="Backends"
      lede="A backend turns the AST into source text. Seven exist — React, Svelte 5, static HTML, Web Components, a JSON UI tree, and the A2UI and MCP-UI agent formats — and they share one element table, one design-system table, one expression lowering and one liveness answer. That sharing is what makes “GUML is an IR” a claim about the language rather than about one emitter, and all four of those tables have drifted at least once."
      toc={[
        { id: "coverage", title: "What v0.1 covers" },
        { id: "retry", title: "Retry and the deferred cache" },
        { id: "example", title: "Source and output" },
        { id: "html", title: "Static HTML" },
        { id: "gaps", title: "What it refuses to guess" },
        { id: "design-system", title: "The design-system table" },
        { id: "targets", title: "Every backend" },
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
            "fetch on mount with AbortController cancellation, loading / empty / error slots, optimistic apply and rollback, and a callable .list refetch",
          ],
          [
            "list / table",
            "lowered",
            "keyed map, where= filtering on the row's own boolean field, aggregates like tasks.open.count",
          ],
          [
            "on mount / on {expr}",
            "lowered",
            "useEffect with the trigger as its dependency; onMount and $effect + untrack in Svelte",
          ],
          ["form / tabs / faq / tier", "lowered", "submit wiring with a pending flag, segmented control from a domain, faq as <details>"],
          ["js / raw blocks", "lowered", "emitted verbatim; reported as GUML0090 so the escape-hatch rate is countable"],
          ["Source maps", "lowered", "every declaration and element, nested ones included"],
          [
            "Retry with backoff",
            "lowered",
            "idempotent methods only, on 5xx and transport failures; emitted once per file, never imported",
          ],
          ["def user components", "lowered", "expanded at compile time, so every backend gets them free"],
          [
            "Response cache",
            "lowered",
            "in-flight deduplication, stale-while-revalidate, invalidation on mutation, and serving stale on a network failure",
          ],
          [
            "Error boundary",
            "lowered",
            "only for a document using a js or raw block — generated render code has nothing in it to throw",
          ],
        ]}
      />

      <H2 id="retry">Retry, and the cache that is not built yet</H2>
      <P>
        Every fetch and every mutation goes through a generated <C>retrying</C> helper — emitted once
        per file, never imported, because a compiled page has no GUML runtime dependency. Its policy is
        deliberately narrow, and each part of it is a mistake the hand-written version usually makes:
      </P>
      <Table
        head={["rule", "why"]}
        rows={[
          [
            "idempotent methods only",
            "a repeated POST with no idempotency key creates two rows, so the decision belongs to the method rather than to a flag a caller can set on the wrong request",
          ],
          [
            "5xx and transport failures only",
            "a 404 or a 422 answers the same way next time; retrying it only delays the error the author needs to see",
          ],
          ["an abort is not retried", "that is the leak the AbortController existed to prevent"],
          ["exponential, from 300 ms, three attempts", "a fixed delay converts one slow backend into a thundering herd"],
        ]}
      />
      <Note tone="warn" title="A response cache is deferred on purpose">
        <p>
          Retry needed no new syntax, so it is on by default and invisible. A cache does need syntax,
          because its <em>lifetime</em> is not something the compiler can infer — a price list and an
          unread-message count want opposite answers, and a module-level cache shared across component
          instances changes what a second mount sees. Guessing a default here would be a decision
          disguised as a convenience, so the language waits for the decision instead.
        </p>
      </Note>

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

      <H2 id="targets">Every backend, and why it exists</H2>
      <Table
        head={["target", "why it exists", "state"]}
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
          ["Svelte 5", "the compile-away-the-framework and bundle-size story; runes, not stores", "shipping"],
          [
            "Web Components",
            "portability: a file a browser runs as-is, embeddable where a framework is not on the table",
            "shipping",
          ],
          [
            "A2UI",
            "the agent-UI format. Targets the documented shape, and says so in the payload rather than claiming conformance it has not validated",
            "shipping",
          ],
          [
            "MCP-UI",
            "composes the html and wc backends into the protocol's two rendering modes; invents no format",
            "shipping",
          ],
        ]}
      />
      <P>
        The A2UI and MCP-UI emitters are strategic as much as technical: they turn the closest
        standards competitor into a distribution channel and a benchmark baseline. See{" "}
        <A href="/research/prior-art">prior art</A>.
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
