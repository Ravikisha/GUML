import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Source maps",
  description:
    "Source Map v3 from GUML to emitted TSX: line granularity, inlined sourcesContent, and `guml where` for looking a line up from the terminal.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/source-maps"
      meter={{ label: "granularity", value: "line" }}
      title="Source maps"
      lede="One `data` line becomes about sixty lines of TSX. Without a map, every stack trace and every breakpoint points at code the author never wrote — which is an adoption blocker, not a nicety."
      toc={[
        { id: "why", title: "Why they matter here" },
        { id: "emit", title: "Emitting one" },
        { id: "where", title: "Looking a line up" },
        { id: "granularity", title: "Why line, not column" },
        { id: "coverage", title: "What is mapped" },
        { id: "format", title: "The format" },
      ]}
    >
      <H2 id="why">Why they matter here</H2>
      <P>
        Expansion is the point of the compiler and the problem for debugging. The counter fixture is 11
        lines of GUML and 39 lines of TSX; the task fixture&rsquo;s single <C>data</C> declaration
        expands to state, an aborting fetch effect, one callback per mutation with optimistic apply and
        rollback, and loading, empty and error rendering.
      </P>
      <P>
        When something throws inside that, the useful question is not &ldquo;which generated line&rdquo;
        but &ldquo;which line did I write&rdquo;. That is the whole job of the map.
      </P>

      <H2 id="emit">Emitting one</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml build counter.guml -o out --source-map
# wrote out/Counter.tsx
# wrote out/Counter.tsx.map`}
      />
      <P>
        <C>sourcesContent</C> is inlined, so the map is self-contained: a debugger can show the
        original GUML without the <C>.guml</C> file being present or resolvable.
      </P>

      <H2 id="where">Looking a line up</H2>
      <P>
        A map is a JSON blob of base64 VLQ, which is not something to read by eye. <C>guml where</C>
        answers the question directly:
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml where <file> <emitted-line> [--backend react]

guml where fixtures/b.guml 155
# fixtures/b.guml:21
#   21 | check {done} >tasks.save`}
      />
      <P>
        The emitted line number is 1-based, matching what a stack trace or an editor reports.
      </P>

      <H2 id="granularity">Why line, not column</H2>
      <P>
        Source Map v3 can carry columns, and this one deliberately does not. A single GUML line becomes
        a <em>region</em> of TSX — <C>btn Add primary disabled=&#123;!draft.trim()&#125;</C> becomes a
        button element whose class list, type, disabled expression and children come from different
        parts of one line. There is no honest column to name.
      </P>
      <Note tone="warn" title="A precise-looking wrong answer is worse than a coarse right one">
        <p>
          A map claiming column precision would send a debugger to an arbitrary character inside the
          generated line. It would look authoritative and be meaningless. Line granularity is the
          claim the compiler can actually support.
        </p>
      </Note>

      <H2 id="coverage">What is mapped</H2>
      <P>
        Every declaration and every element, nested ones included. A mapping is a range: an emitted
        line inherits the last mark at or before it.
      </P>
      <Table
        head={["construct", "resolves to"]}
        rows={[
          ["a `state` declaration", <>its <C key="a">useState</C> line</>],
          ["a `data` declaration", "the whole resource block — the mapping that matters most, because a failed fetch should point at the declaration"],
          ["a top-level element", "its opening JSX line"],
          ["an element inside a row template", <>its own line, not the <C key="b">list</C> above it</>],
          ["a repeater's `<ul>`, `.map(` and closing tags", <>the <C key="c">list</C> line that owns them</>],
          ["a `js` block", <>nothing in the JSX — it is hoisted into the component body, so marking it there would attribute the next element&rsquo;s line to the block</>],
        ]}
      />
      <P>
        That fourth row is the one worth having. Before nesting was mapped, everything inside a{" "}
        <C>list</C> resolved to the <C>list</C> line, so three different constructs shared one
        attribution — a valid map that opened the right file at the wrong line.
      </P>

      <H2 id="format">The format</H2>
      <UL>
        <LI>
          Standard <strong>Source Map v3</strong>: <C>version</C>, <C>sources</C>,{" "}
          <C>sourcesContent</C>, <C>names</C>, <C>mappings</C>.
        </LI>
        <LI>
          <C>mappings</C> is base64 VLQ with one group per emitted line, semicolon-separated, exactly
          as the specification describes.
        </LI>
        <LI>
          Segments carry generated column <C>0</C>, source index <C>0</C>, and the original line.
          Original column is always <C>0</C>, following from the granularity decision above.
        </LI>
        <LI>
          Serialisation happens in the driver rather than the backend, because it needs the source{" "}
          <em>text</em> and only the driver holds it.
        </LI>
      </UL>
      <P>
        Any tool that reads Source Map v3 will read this — browser devtools, a bundler, an error
        reporter. See <A href="/docs/compiler/backends">Backends</A> for which backends produce one.
      </P>
    </DocPage>
  );
}
