import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Conformance levels",
  description:
    "core is markup: no I/O, no state, no behaviour, safe to render from an untrusted source. app adds resources, actions and mutations.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/levels"
      meter={{ label: "levels", value: "2" }}
      title="Conformance levels"
      lede="One language, two levels — the way CommonMark and GFM are one language with two levels. `core` is markup you can render from a stranger. `app` is a framework, and is not."
      toc={[
        { id: "why", title: "Why the split exists" },
        { id: "core", title: "What core contains" },
        { id: "app", title: "What app adds" },
        { id: "using", title: "Compiling at a level" },
        { id: "errors", title: "Rejection, not filtering" },
        { id: "raw", title: "Where raw sits" },
      ]}
    >
      <H2 id="why">Why the split exists</H2>
      <P>
        Markdown is safe to embed because of what it refuses to contain. A host can render Markdown from
        an unknown author because there is nothing in the document to <em>run</em>. That refusal is the
        reason it ended up everywhere.
      </P>
      <P>
        GUML spans two categories. <C>card</C>, <C>h1</C>, <C>p</C>, <C>tier</C>, <C>faq</C> are markup.{" "}
        <C>data tasks:Task[] GET /api/tasks</C> and <C>{">tasks.add{title:draft}"}</C> are not — they
        declare network requests and mutations on the host&rsquo;s behalf. Accepting the first from an
        agent is ordinary; accepting the second is a decision.
      </P>
      <Note tone="warn" title="Deciding this later would be impossible">
        <p>
          Every document written in the meantime would have to be re-classified, and the answer for each
          one would depend on which constructs happened to be in it. The split is cheap now and
          permanently expensive after adoption.
        </p>
      </Note>

      <H2 id="core">What core contains</H2>
      <P>
        Containers, text, controls, content blocks, and everything the theme does with them. A core
        document has no state, no resources, no actions and no <C>js</C>.
      </P>
      <CodeBlock
        lang="guml"
        filename="landing.guml — valid core"
        code={`page Landing title="Northwind" lang=en

hero
  h1 Build the interface, skip the boilerplate
  p Describe the page, get a deployable build.

section #pricing cols=3
  tier Pro $24/mo "For working developers" cta="Go Pro" /signup featured
    Unlimited projects
    Custom domains

faq open=1
  Can I export the code? | Yes. Every build is plain source.`}
      />
      <P>
        That compiles, renders, and needs no runtime at all — the <A href="/docs/compiler/backends">
        static HTML backend
        </A>{" "}
        turns it into one file with no JavaScript. <C>faq</C> stays interactive, because{" "}
        <C>&lt;details&gt;</C> always was.
      </P>

      <H2 id="app">What app adds</H2>
      <Table
        head={["construct", "why it is app-level"]}
        rows={[
          [<C key="a">state</C>, "mutable state needs a runtime"],
          [<C key="b">data</C>, "declares a network request"],
          [<>an action (<C key="c">{">"}</C>)</>, "runs code on an event"],
          [<C key="d">js</C>, "arbitrary code by construction"],
          [
            <>
              <C key="e">list</C>, <C key="f">table</C>
            </>,
            "a repeater has nothing to iterate without a resource, so it is not in the core vocabulary at all",
          ],
        ]}
      />

      <H2 id="using">Compiling at a level</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`# app is the default — nothing changes for an existing document
guml check tasks.guml

# markup only
guml check untrusted.guml --core
guml build untrusted.guml --core --backend html`}
      />
      <P>
        The level is carried by the <em>registry</em>, not by a separate flag that a call site could
        forget to thread through: at the core level an app-level tag is genuinely not in the vocabulary.
        A host embedding GUML passes <C>Registry::core()</C> and gets the guarantee by construction.
      </P>
      <Note>
        <p>
          A <A href="/docs/language/registry">loaded registry</A> composes with this. A core host may add
          its own markup components, and any app-level entry in that registry document is skipped rather
          than merged — a registry cannot smuggle behaviour past a host that asked for markup.
        </p>
      </Note>

      <H2 id="errors">Rejection, not filtering</H2>
      <P>
        An app-level construct at the core level is <C>GUML0091</C>, an <strong>error</strong>. The
        tempting alternative — strip the <C>data</C> line and compile the rest — would hand the host a
        page that looks complete and fetches nothing.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml check tasks.guml --core

error[GUML0091]: \`data\` is an app-level construct, and this document is being
                 compiled at the core level
  --> tasks.guml:4:1
   = help: a resource declares a network request; \`guml explain GUML0091\`
           describes the two levels`}
      />

      <H2 id="raw">Where raw sits</H2>
      <P>
        <C>raw html</C> is allowed at the core level and <C>js</C> is not. The line is the same one a
        Markdown renderer draws when it decides whether inline HTML is in scope: markup from an untrusted
        source is a containable risk that a sanitiser can address, and arbitrary script is not.
      </P>
      <UL>
        <LI>
          Both are still reported as <C>GUML0090</C>, so the escape-hatch rate stays measurable.
        </LI>
        <LI>
          A host that will not accept <C>raw</C> either can reject the document on that code.
        </LI>
      </UL>
      <P>
        See <A href="/docs/compiler/validator">the validator</A> for the rest of the diagnostics, and{" "}
        <C>spec/STABILITY.md</C> for what a level promises over time — a tag may not change level, because
        that would invalidate a document which was correct when it was written.
      </P>
    </DocPage>
  );
}
