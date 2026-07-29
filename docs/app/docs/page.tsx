import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { CodePreview } from "@/components/code-preview";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";
import { FIXTURES } from "@/lib/fixtures.generated";
import { commas } from "@/lib/utils";

export const metadata: Metadata = {
  title: "What GUML is",
  description:
    "GUML is an intermediate representation between a prompt and framework source code, plus a compiler that expands it.",
};

const counter = FIXTURES.find((f) => f.id === "counter")!;
const tasks = FIXTURES.find((f) => f.id === "tasks")!;

export default function Page() {
  return (
    <DocPage
      pathname="/docs"
      meter={{ label: "status", value: "v0.1 · pre-Phase-0", tone: "ember" }}
      title="What GUML is"
      lede={
        <>
          GUML is a representation that sits between a prompt and framework source code — small
          enough for a model to emit in one shot, and specific enough for a compiler to expand into
          a working app.
        </>
      }
      toc={[
        { id: "the-idea", title: "The idea" },
        { id: "what-it-is-not", title: "What it is not" },
        { id: "how-it-reads", title: "How it reads" },
        { id: "state-of-play", title: "State of play" },
        { id: "where-to-go", title: "Where to go next" },
      ]}
    >
      <H2 id="the-idea">The idea</H2>
      <P>
        When a model builds a screen today, it emits the final artifact: JSX, hooks, effect
        dependencies, Tailwind class strings, ARIA attributes. Most of that is mechanically
        derivable. The model is being paid — in tokens, latency, and error probability — to
        hand-write a compiler&rsquo;s output.
      </P>
      <P>
        GUML moves that work to a compiler. The model writes what it actually decided; everything
        conventional is generated. On the task-CRUD fixture that is {commas(tasks.tokens.react)}{" "}
        tokens of React replaced by {commas(tasks.tokens.guml)} tokens of markup, with the loading,
        empty, error and rollback behaviour generated rather than remembered.
      </P>

      <H2 id="what-it-is-not">What it is not</H2>
      <UL>
        <LI>
          <strong className="text-chalk">Not a new markdown.</strong> MDX, Markdoc, A2UI and
          Vega-Lite already occupy that framing. GUML is an intermediate representation with a
          compiler, and the interesting contribution is the measurement, not the syntax.
        </LI>
        <LI>
          <strong className="text-chalk">Not a runtime.</strong> Nothing ships to the browser.
          GUML compiles away entirely, which is what separates it from server-driven UI and from
          agent-UI protocols that need a client renderer.
        </LI>
        <LI>
          <strong className="text-chalk">Not a replacement for React.</strong> React is the
          compile target. The thing GUML competes with is un-representational code emission.
        </LI>
        <LI>
          <strong className="text-chalk">Not a rival to A2UI or MCP-UI.</strong> Those are compile
          targets on the roadmap. GUML&rsquo;s difference is the token surface, an application-logic
          layer, and output you can deploy and own.
        </LI>
      </UL>

      <H2 id="how-it-reads">How it reads</H2>
      <P>
        Indentation is nesting. No closing tags, no imports, no class names. The first bare word on
        a line is a tag; <C>{">"}</C> introduces behaviour and takes the rest of the line.
      </P>
      <CodePreview
        code={counter.guml}
        lang="guml"
        filename="counter.guml"
        meter="64 tokens · 11 lines"
      />
      <P>
        That compiles to a React component with the design system, the state setters, and the
        conditional disable all supplied by the compiler:
      </P>
      {counter.emitted ? (
        <CodeBlock
          code={counter.emitted}
          lang="tsx"
          filename="Counter.tsx — emitted"
          meter="~489 tokens · 18 lines"
          maxHeight={380}
        />
      ) : null}

      <H2 id="state-of-play">State of play</H2>
      <Table
        head={["component", "state"]}
        rows={[
          ["Lexer, AST, parser, diagnostics", "working, tested"],
          ["Component registry, typo suggestions", "working · 27 primitives"],
          ["React backend", "containers, text, controls, state, actions, bindings, layout"],
          [
            "Resources, repeaters, forms, tabs, optimistic mutations",
            "lowered: fetch and cancel, loading, empty, error, optimistic apply, snapshot rollback",
          ],
          [
            "Expression lowering",
            "GUML expressions → JS, mirrored in the runtime with a parity test",
          ],
          ["Expression parsing", "still pass-through (Phase 2)"],
          ["Phase 0 harness", "built and self-tested · needs an API key and a grader"],
          ["GUML-Bench, LLM repair loop, second backend", "not started"],
        ]}
      />
      <Note tone="warn" title="Read the status honestly">
        <p>
          The compiler front end works and has 49 passing tests. The research question the project
          exists to answer — whether a model can produce correct GUML, and whether that improves or
          degrades correctness against a React baseline — is{" "}
          <A href="/docs/research/phase0">still open</A>. Claims about it are labelled as hypotheses
          throughout these docs.
        </p>
      </Note>

      <H2 id="where-to-go">Where to go next</H2>
      <UL>
        <LI>
          <A href="/docs/quickstart">Quickstart</A> — clone, test, compile a fixture, read the
          output.
        </LI>
        <LI>
          <A href="/docs/language/syntax">Syntax</A> — the whole surface in one page.
        </LI>
        <LI>
          <A href="/docs/research/measurements">Measurements</A> — the numbers and every caveat that
          travels with them.
        </LI>
      </UL>
    </DocPage>
  );
}
