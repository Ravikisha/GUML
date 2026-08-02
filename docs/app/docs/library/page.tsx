import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { LivePreview } from "@/components/live-preview";
import { A, C, H2, LI, Note, P, Pkg, Table, UL } from "@/components/prose";
import { FIXTURES } from "@/lib/fixtures.generated";
import { PACKAGES } from "@/lib/packages";

export const metadata: Metadata = {
  title: "React library",
  description:
    "Ship GUML in a React or Next app: the Rust compiler as WebAssembly, plus a runtime that renders its UI tree.",
};

const counter = FIXTURES.find((f) => f.id === "counter")!;

export default function Page() {
  return (
    <DocPage
      pathname="/docs/library"
      meter={{ label: "package", value: "@guml/core · 787 KB wasm", tone: "iris" }}
      title="React library"
      lede="The compiler compiled: the same Rust that powers the CLI, built to WebAssembly, with a React runtime that renders its output. No build step in your app, no server."
      toc={[
        { id: "install", title: "Install" },
        { id: "smaller", title: "Smaller packages" },
        { id: "render", title: "Render GUML" },
        { id: "compile", title: "Compile without rendering" },
        { id: "repair", title: "Mechanical repair" },
        { id: "components", title: "Your own components" },
        { id: "data", title: "Data and mutations" },
        { id: "security", title: "Security" },
        { id: "scope", title: "Scope" },
      ]}
    >
      <H2 id="install">Install</H2>
      <CodeBlock lang="bash" filename="terminal" code={`pnpm add @guml/core`} />
      <P>
        Two entry points: <Pkg name="@guml/core" /> for the compiler API and <C>@guml/core/react</C> for the
        renderer. React is an optional peer dependency, so the compiler works in a plain script or a
        worker without it.
      </P>
      <Note tone="warn" title="Browser and bundlers, not Node">
        <p>
          The wasm is built for the web target, so it loads itself with <C>fetch</C>. That works in
          Next.js, Vite and anything else serving assets over HTTP. It does <em>not</em> work in plain
          Node, where the failure surfaces as an undici <C>fetch failed</C> on a <C>file://</C> URL
          with nothing in the message about WebAssembly.
        </p>
        <p className="mt-3">
          To compile GUML from Node or a shell, use the CLI — <C>cargo install guml-cli</C> — or{" "}
          <Pkg name="@guml/fmt" /> below, which does load in Node.
        </p>
      </Note>

      <H2 id="smaller">Smaller packages, if you need less</H2>
      <P>
        The compiler is 787 KB of WebAssembly. Two of the things people most often want from it do not
        need the compiler at all, so they ship separately rather than making everyone pay for the whole
        thing.
      </P>
      {/* Rows come from `lib/packages.ts` so the sizes here and the sizes on the registry are one
          number, checked by `pnpm check:packages`. A hand-written table of sizes is stale the first
          time anything is republished, and nothing about the page would show it. */}
      <Table
        head={["package", "for", "unpacked", "Node"]}
        rows={PACKAGES.filter((p) => !p.name.includes("widgets") && !p.name.includes("shadcn")).map(
          (p) => [<Pkg key={p.name} name={p.name} />, p.purpose, p.size, p.node ? "yes" : "no"],
        )}
      />
      <P>
        The split follows the compiler&rsquo;s own shape rather than being a packaging convenience.{" "}
        <C>guml-fmt</C> sits <em>below the parser</em> — it needs the lexer, the registry and the
        diagnostic codes, and nothing else — so building only that is 178 KB instead of 787. A
        pre-commit hook that formats GUML has no reason to download the code generator for seven
        backends.
      </P>
      <P>
        <Pkg name="@guml/highlight" /> goes further and has no wasm at all. It is a hand-written tokeniser held
        to the compiler&rsquo;s classifier by a parity test that compares the two span for span over
        every fixture — 936 spans across 10 documents. That is what makes a second implementation safe
        rather than a second source of truth, and it buys something the wasm cannot offer: highlighting
        that runs synchronously during server rendering, and in Node.
      </P>
      <Note tone="info" title="Why the whole compiler ships to the browser">
        <p>
          A re-implementation in TypeScript would drift from the Rust one, and the moment a preview
          disagrees with <C>guml build</C> the preview is worse than nothing. Compiling the real thing
          to wasm32 costs 787 KB and removes that class of bug entirely.
        </p>
      </Note>

      <H2 id="render">Render GUML</H2>
      <CodeBlock
        lang="tsx"
        filename="app/page.tsx"
        code={`"use client";
import { Guml } from "@guml/core/react";

const source = \`page Counter
state count=0

card sm center
  h Clicks
  metric {count}
  row center
    btn Decrement ghost disabled={!count} >count--
    btn Increment primary >count++
\`;

export default function Page() {
  return <Guml source={source} />;
}`}
      />
      <P>That renders this — compiled in your browser, right now:</P>
      <LivePreview source={counter.guml} label="live · the example above" />
      <P>
        The markup and classes come from the compiler&rsquo;s own render tree, so they match the
        code the <A href="/docs/compiler/backends">React backend</A> writes. Tailwind is expected in
        the host app; if you would rather not, override the components below.
      </P>

      <H2 id="compile">Compile without rendering</H2>
      <CodeBlock
        lang="tsx"
        filename="anywhere"
        code={`import { check, compile, tree, registry } from "@guml/core";

const { ok, diagnostics } = await check(source);

const { files } = await compile(source, "react");
// files[0] → { path: "Counter.tsx", contents: "import { useState } …" }

const { tree: ui } = await tree(source);   // render tree, for a custom renderer
const slice = await registry(["btn", "card", "list"]); // prompt-sized vocabulary`}
      />
      <Table
        head={["export", "purpose"]}
        rows={[
          [<C key="a">check(source)</C>, "parse + analyse; every diagnostic in one pass"],
          [<C key="b">compile(source, backend)</C>, "framework source (`react`) or a render tree (`json`)"],
          [<C key="c">tree(source)</C>, "the render tree the React runtime consumes"],
          [<C key="d">registry(tags?)</C>, "the component vocabulary, or a slice for a prompt"],
          [<C key="e">formatDiagnostic(d, src)</C>, "CLI-style rendering with a caret"],
          [<C key="f">evaluate / runAction</C>, "binding evaluator and action lowering, for custom renderers"],
          [<C key="g">init(url?)</C>, "warm the wasm module before a user starts typing"],
        ]}
      />

      <H2 id="repair">Mechanical repair</H2>
      <P>
        Diagnostics carry a <C>suggestion</C> when the fix is unambiguous, and this is the half of a
        repair loop that needs no model call at all:
      </P>
      <CodeBlock
        lang="tsx"
        filename="repair.ts"
        code={`import { check, applyAllSuggestions } from "@guml/core";

let source = modelOutput;
for (let round = 0; round < 3; round++) {
  const { ok, diagnostics } = await check(source);
  if (ok) break;

  const fixed = applyAllSuggestions(source, diagnostics);
  if (fixed !== source) {
    source = fixed;      // free round: typos, unknown tags, missing labels
    continue;
  }
  source = await askModelToFix(source, diagnostics);  // paid round
}`}
      />
      <Note tone="tip" title="Try it">
        <p>
          The <A href="/playground">playground</A>&rsquo;s “broken” sample has three errors from
          three different compiler passes, and all three are mechanically fixable. The{" "}
          <strong className="text-chalk">apply fixes</strong> button is this function.
        </p>
      </Note>

      <H2 id="components">Your own components</H2>
      <P>
        Map any tag to your own component and keep GUML&rsquo;s semantics — state, actions,
        bindings and accessible names all still work.
      </P>
      <CodeBlock
        lang="tsx"
        filename="app/page.tsx"
        code={`<Guml
  source={source}
  components={{
    btn: (node, children) => <MyButton onClick={node.actions[0] ? undefined : undefined}>{children}</MyButton>,
    card: (node, children) => <Surface>{children}</Surface>,
  }}
/>`}
      />
      <P>
        This is the same idea as a host-approved component catalog in the agent-UI protocols: the
        document names a tag, the host decides what that tag is.
      </P>

      <H2 id="data">Data and mutations</H2>
      <P>
        A <C>data</C> resource fetches from its declared URL. Seed it instead for previews, tests or
        Storybook:
      </P>
      <CodeBlock
        lang="tsx"
        code={`<Guml
  source={tasksSource}
  data={{ tasks: [{ id: "1", title: "Ship it", done: false }] }}
  baseUrl="https://api.example.com"
/>`}
      />
      <P>
        Optimistic mutations apply immediately and roll back if the request fails — which is exactly
        what <C>optimistic:prepend</C> in the source declares, implemented once in the runtime
        instead of regenerated per screen.
      </P>

      <H2 id="security">Security</H2>
      <UL>
        <LI>
          <strong className="text-chalk">No <C>eval</C>, no <C>new Function</C>.</strong> Bindings go
          through a small recursive-descent evaluator.
        </LI>
        <LI>
          <strong className="text-chalk">Actions are not Turing-complete.</strong> They lower to a
          fixed set of effects: set state, or invoke a declared mutation.
        </LI>
        <LI>
          <strong className="text-chalk">Unknown tags never reach the DOM.</strong> They are a
          compile error, not a fallback render.
        </LI>
      </UL>
      <P>
        That combination is what makes it defensible to render a document produced by an untrusted
        agent — the same reasoning behind A2UI&rsquo;s declarative-only payloads, with the escape
        hatch opt-in rather than impossible.
      </P>

      <H2 id="scope">Scope</H2>
      <Table
        head={["construct", "runtime v0"]}
        rows={[
          ["state, actions, bindings", "yes"],
          ["containers, text, controls, fields", "yes"],
          ["list, with seeded or fetched rows", "yes"],
          ["optimistic mutations with rollback", "yes"],
          ["form, tabs, faq, tier", "renders a labelled gap"],
          ["route, auth, js/raw", "not lowered — reported as a gap"],
        ]}
      />
      <P>
        Anything not lowered renders as a visible gap rather than as approximate markup, matching how
        the CLI reports it. <C>useGumlTree</C> hands you the diagnostics if you would rather handle
        it yourself.
      </P>
    </DocPage>
  );
}
