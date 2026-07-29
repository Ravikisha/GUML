import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { CodePreview } from "@/components/code-preview";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";
import { SAMPLES } from "@/lib/samples";

export const metadata: Metadata = {
  title: "User components",
  description:
    "def declares a compile-time component: positional parameters, substituted into bindings, attributes and prose, expanded before codegen so it works in every backend.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/components"
      meter={{ label: "runtime cost", value: "0", tone: "mint" }}
      title="User components"
      lede="`def` is the one way to stop repeating yourself. It is a compile-time macro, not a framework concept — by the time a backend sees the tree, there is no trace that a `def` was involved."
      toc={[
        { id: "shape", title: "Declaring one" },
        { id: "params", title: "Where a parameter substitutes" },
        { id: "macro", title: "Why a macro, not a component" },
        { id: "level", title: "Levels" },
        { id: "refused", title: "What it refuses to do" },
        { id: "slots", title: "Slots, and why not yet" },
      ]}
    >
      <H2 id="shape">Declaring one</H2>
      <P>
        <C>def &lt;name&gt; &lt;params…&gt;</C> and an indented body. Parameters are positional, and a
        call supplies them in order.
      </P>
      <CodePreview {...SAMPLES["defs.stat"]} />
      <P>
        A <C>def</C> must appear before it is used, like every other directive — and it adds a name to
        the vocabulary rather than replacing one, so <C>stat</C> is now a tag the compiler knows and an
        unknown tag is still an error.
      </P>

      <H2 id="params">Where a parameter substitutes</H2>
      <Table
        head={["place", "example"]}
        rows={[
          [<>a binding positional</>, <C key="a">h {"{label}"}</C>],
          [<>an attribute value</>, <C key="b">input draft aria={"{label}"}</C>],
          [<>inside prose</>, <C key="c">p Total {"{value}"} this quarter</C>],
        ]}
      />
      <P>
        Substitution is <strong>by value</strong>, and the kind of the argument carries through: a quoted
        string becomes text, a binding stays a binding. So <C>stat &quot;Revenue&quot; {"{revenue}"}</C>{" "}
        produces a literal heading and a live metric from the same body.
      </P>
      <P>
        Any other <C>{"{name}"}</C> in the body refers to the surrounding document as usual — a def can
        read page state directly without taking it as a parameter. Parameters shadow, exactly the way a
        repeater&rsquo;s row fields shadow page state.
      </P>

      <H2 id="macro">Why a macro, not a component</H2>
      <P>
        Expansion happens in the compiler, before resolution, validation and codegen. Three consequences,
        and they are the reason for the choice:
      </P>
      <UL>
        <LI>
          <strong>It works in every backend.</strong> Nothing in <C>guml-codegen</C> knows what a{" "}
          <C>def</C> is, so a user component works in the <A href="/docs/compiler/backends">
          no-JavaScript HTML backend
          </A>{" "}
          with no extra support — which a runtime component could not do.
        </LI>
        <LI>
          <strong>Every existing check applies.</strong> The resolver, the accessibility lint and the type
          checker see expanded markup, so an unlabelled control inside a def body is still{" "}
          <C>GUML0051</C> and a binding that names nothing is still <C>GUML0033</C>.
        </LI>
        <LI>
          <strong>The output is identical to writing it inline.</strong> Not approximately — byte for
          byte. A test asserts it.
        </LI>
      </UL>
      <Note>
        <p>
          It also keeps GUML&rsquo;s &ldquo;no imports, no framework concepts&rdquo; property intact. A{" "}
          <C>def</C> is a way of not repeating yourself, not a module system.
        </p>
      </Note>

      <H2 id="level">Levels</H2>
      <P>
        There is no <C>level</C> on a <C>def</C>, because there is nothing to declare: a def{" "}
        <em>inherits</em> its <A href="/docs/language/levels">conformance level</A> from its body. A body
        of markup is core; a body containing an action is app-level by virtue of containing one, and at
        the core level that is <C>GUML0091</C> on the line inside the def where the action sits.
      </P>

      <H2 id="refused">What it refuses to do</H2>
      <P>
        A macro that quietly drops what it cannot handle is the worst possible shape for this feature, so
        each of these is an error rather than an omission:
      </P>
      <Table
        head={["code", "case"]}
        rows={[
          [<C key="a">GUML0093</C>, "the name is already a builtin tag, or another def"],
          [<C key="b">GUML0094</C>, "wrong number of arguments — arity is exact, with no defaults"],
          [
            <C key="c">GUML0095</C>,
            "recursion, direct or mutual. Expansion is compile-time, so there is no base case to stop at — the diagnostic names the cycle",
          ],
          [<C key="d">GUML0096</C>, "an empty body, which would make every call site vanish"],
          [
            <C key="e">GUML0097</C>,
            "a parameter inside an action body, or children at a call site",
          ],
        ]}
      />
      <Note tone="warn" title="Why a parameter in an action is refused">
        <p>
          Actions lower to JavaScript. Substituting a parameter into one means deciding whether the
          argument is a variable reference or a literal — and the call site does not answer that
          question. Guessing would produce code that compiles and does the wrong thing, which is exactly
          the outcome this compiler exists to prevent. Put the action at the call site, where its scope is
          unambiguous.
        </p>
      </Note>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`error[GUML0095]: \`a\` expands into itself: a → b → a
   = help: expansion happens at compile time, so there is no base case to stop at;
           a repeating structure wants \`list\` over a resource`}
      />

      <H2 id="slots">Slots, and why not yet</H2>
      <P>
        A call may not take children. That is a real limitation — it means a <C>def</C> cannot wrap
        content — and it is deliberate: allowing children later is <em>additive</em>, so deferring costs
        nothing, while shipping a slot design that turns out wrong would be permanent under the{" "}
        <A href="/docs/research/roadmap">stability policy</A>.
      </P>
      <P>
        Until then, children at a call site are <C>GUML0097</C> rather than silently discarded. An unused
        parameter is a warning on the same reasoning as an unused <C>state</C>: free to notice, almost
        always a mistake.
      </P>
    </DocPage>
  );
}
