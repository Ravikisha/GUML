import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Step, Steps, UL } from "@/components/prose";
import { FIXTURES } from "@/lib/fixtures.generated";

export const metadata: Metadata = {
  title: "Quickstart",
  description: "Write your first GUML page and compile it to React in about a minute.",
};

const counter = FIXTURES.find((f) => f.id === "counter")!;

export default function Page() {
  return (
    <DocPage
      pathname="/docs/quickstart"
      meter={{ label: "time", value: "~60 seconds" }}
      title="Quickstart"
      lede="Write eleven lines, compile them, read what the compiler wrote for you."
      toc={[
        { id: "walkthrough", title: "Walkthrough" },
        { id: "what-happened", title: "What just happened" },
        { id: "break-it", title: "Break it on purpose" },
        { id: "next", title: "Next" },
      ]}
    >
      <H2 id="walkthrough">Walkthrough</H2>
      <Steps>
        <Step n={1} title="Write a page">
          <P>
            Save this as <C>counter.guml</C>. Two-space indentation, no closing tags, no imports.
          </P>
          <CodeBlock code={counter.guml} lang="guml" filename="counter.guml" meter="64 tokens" />
        </Step>

        <Step n={2} title="Check it">
          <P>
            <C>check</C> parses and validates without emitting anything. It reports every problem in
            one pass, which is what keeps a model&rsquo;s repair loop to a single round.
          </P>
          <CodeBlock
            lang="bash"
            filename="terminal"
            code={`cargo run -q -p guml-cli -- check counter.guml
# ok: counter.guml (0 warnings)`}
          />
        </Step>

        <Step n={3} title="Compile it">
          <CodeBlock
            lang="bash"
            filename="terminal"
            code={`cargo run -q -p guml-cli -- build counter.guml -o out
# wrote out/Counter.tsx
#
# source ~63 tokens -> emitted ~382 tokens (6.1x expansion, estimates only)`}
          />
          <P>Drop the result into any React + Tailwind project. It has no runtime dependency on GUML.</P>
        </Step>

        <Step n={4} title="Read the output">
          {counter.emitted ? (
            <CodeBlock
              code={counter.emitted}
              lang="tsx"
              filename="out/Counter.tsx"
              maxHeight={420}
              lines
            />
          ) : null}
        </Step>
      </Steps>

      <H2 id="what-happened">What just happened</H2>
      <P>Four things in that output were never written in the source:</P>
      <UL>
        <LI>
          <C>useState</C> and <C>setCount</C> — derived from <C>state count=0</C>.
        </LI>
        <LI>
          Every class string. <C>primary</C>, <C>ghost</C>, <C>quiet</C>, <C>sm</C> and{" "}
          <C>center</C> are semantic modifiers; the compiler owns the mapping to Tailwind. Swap the
          table and you have re-themed every page at once.
        </LI>
        <LI>
          <C>type=&quot;button&quot;</C> on each button, so none of them accidentally submit a form.
        </LI>
        <LI>
          The <C>disabled</C> wiring, from <C>disabled=&#123;!count&#125;</C> — a binding, not a
          hand-managed flag.
        </LI>
      </UL>
      <P>
        Those are tokens the model did not spend and details it could not get wrong. That trade is
        the whole argument; the <A href="/docs/research/measurements">measurements page</A> puts
        numbers and caveats on it.
      </P>

      <H2 id="break-it">Break it on purpose</H2>
      <P>
        Diagnostics are the compiler&rsquo;s main interface to a model, so they are worth seeing
        early. Introduce a typo:
      </P>
      <CodeBlock
        lang="guml"
        filename="counter.guml"
        code={`page Counter
state count=0

crad sm center
  h Clicks`}
      />
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo run -q -p guml-cli -- check counter.guml

error[GUML0030]: unknown tag \`crad\`
  --> counter.guml:4:1
   |
 4 | crad sm center
   | ^^^^
   = help: did you mean \`card\`?
   = suggestion: card`}
      />
      <P>
        The <C>suggestion</C> line is a literal replacement for the highlighted span. Add{" "}
        <C>--format json</C> and a harness can apply it mechanically, with no second model call.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo run -q -p guml-cli -- check counter.guml --format json`}
      />

      <Note tone="warn" title="v0.1 limits">
        <p>
          Resources, repeaters, forms, <C>tabs</C>, <C>faq</C> and <C>tier</C> all parse, but the
          React backend does not lower them yet — they emit a warning and a TODO in the output rather
          than wrong code. See <A href="/docs/compiler/backends">backends</A> for exactly what is
          covered.
        </p>
      </Note>

      <H2 id="next">Next</H2>
      <UL>
        <LI>
          <A href="/docs/language/syntax">Syntax</A> — the full surface, one page.
        </LI>
        <LI>
          <A href="/docs/language/registry">Component registry</A> — the closed tag vocabulary and
          how to print a prompt-sized slice of it.
        </LI>
        <LI>
          <A href="/examples">Examples</A> — three fixtures, every representation side by side.
        </LI>
      </UL>
    </DocPage>
  );
}
