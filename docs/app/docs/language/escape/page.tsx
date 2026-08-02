import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Escape hatches",
  description:
    "js and raw hand a block of the target language straight through to the output, verbatim and unchecked — and report themselves so the escape rate stays countable.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/escape"
      meter={{ label: "hatches", value: "2" }}
      title="Escape hatches"
      lede="A closed vocabulary is only honest if there is a way out of it. `js` and `raw` hand a block of the target language through untouched — and each one reports itself, so the rate at which people need them is a number rather than a feeling."
      toc={[
        { id: "why", title: "Why a hatch has to exist" },
        { id: "js", title: "js" },
        { id: "raw", title: "raw" },
        { id: "verbatim", title: "Verbatim means verbatim" },
        { id: "countable", title: "Every block reports itself" },
        { id: "boundary", title: "The security boundary" },
      ]}
    >
      <H2 id="why">Why a hatch has to exist</H2>
      <P>
        GUML&rsquo;s tag set is closed and its actions are deliberately not Turing-complete. Both are
        load-bearing — they are what let the compiler own presentation and what makes a document from
        an untrusted source safe to render. But a language that closes the door and provides no other
        exit does not get fewer escapes; it gets workarounds that are invisible to measurement.
      </P>
      <P>
        So there are two hatches, and they are outside the component vocabulary rather than tags
        within it. <C>guml registry</C> does not list them, and a typo for <C>jsx</C> will never be
        suggested as <C>js</C>.
      </P>

      <H2 id="js">
        <C>js</C>
      </H2>
      <P>
        An indented block of JavaScript, hoisted <em>above</em> the component&rsquo;s return. Use it
        for a helper an expression needs and the expression language cannot say.
      </P>
      <CodeBlock
        lang="guml"
        code={`page Report

js
  const fmt = (n) => n.toFixed(2);

metric {fmt(total)}`}
      />
      <P>
        A name referenced from inside a block counts as a use, so a state declaration read only by
        hatch code does not trip the dead-declaration warning (<C>GUML0074</C>). The optimizer cannot
        see into the block, so it assumes the worst — which is the safe direction.
      </P>

      <H2 id="raw">
        <C>raw</C>
      </H2>
      <P>
        Target-framework markup, placed <em>in the tree</em> where it is written. An optional target
        names which backend it is for.
      </P>
      <CodeBlock
        lang="guml"
        code={`raw react
  <SomeThirdPartyChart data={rows} />

raw svelte
  {#if x}<Custom />{/if}`}
      />
      <P>
        A block addressed to another backend is <strong>skipped, not an error</strong>. Compiling that
        document for React emits the chart and drops the Svelte block entirely; compiling for Svelte
        does the reverse. One document can therefore carry the per-target code it needs without any
        backend receiving a file it cannot parse.
      </P>

      <H2 id="verbatim">Verbatim means verbatim</H2>
      <P>
        A block body is not lexed as GUML. Nothing inside is validated, reformatted, escaped or
        renamed:
      </P>
      <UL>
        <LI>
          <C>{"//"}</C> is the <em>host language&rsquo;s</em> comment and is preserved, not dropped
          the way a GUML comment line is.
        </LI>
        <LI>
          Braces, tabs and template literals survive: <C>{"if (a) { b(); }"}</C> is a line of
          JavaScript, not a GUML brace group.
        </LI>
        <LI>
          Relative indentation inside the block is preserved, so a multi-line template literal comes
          out with its own layout intact.
        </LI>
        <LI>
          A blank line does not end the block. It ends at the first line that dedents out of it.
        </LI>
      </UL>
      <Note tone="warn" title="A hatch that mangles its contents is not a hatch">
        <p>
          This is why the body is held as raw lines rather than parsed and reprinted. If a block were
          silently reformatted, people would stop using it and go around the language instead — and
          the escape-hatch rate, which is one of the numbers this project is trying to measure, would
          quietly become meaningless.
        </p>
      </Note>

      <H2 id="countable">Every block reports itself</H2>
      <Table
        head={["code", "severity", "meaning"]}
        rows={[
          [
            <C key="a">GUML0090</C>,
            "note",
            "an escape hatch was used, with how many lines it contained",
          ],
        ]}
      />
      <P>
        One note per block, never an error — a hatch that failed the build would be useless. Because
        it is a diagnostic, the rate is readable from <C>guml check --format json</C> across a corpus
        rather than by grepping source, which is exactly how the{" "}
        <A href="/research/measurements">measurements</A> track it.
      </P>

      <H2 id="boundary">The security boundary</H2>
      <P>
        The JSON backend feeds the browser runtime — the thing that renders the{" "}
        <A href="/playground">playground</A>, and the path a document from an untrusted agent would
        travel. There, a block body is <strong>dropped</strong> rather than passed through.
      </P>
      <CodeBlock
        lang="json"
        code={`{ "tag": "js-placeholder", "note": "not run in the preview" }`}
      />
      <P>
        The reason is the same one that keeps actions away from Turing-completeness: a <C>js</C> block
        that reached a client-side <C>eval</C> would erase the guarantee that makes{" "}
        <A href="/docs/language/levels">core</A> safe to embed. Code in a hatch runs when a developer
        compiles and ships it, never because a preview rendered it. That is also why <C>js</C> is an{" "}
        <C>app</C>-level construct and <C>raw</C> is not: markup that a backend passes through is
        still markup, but a script is not.
      </P>
    </DocPage>
  );
}
