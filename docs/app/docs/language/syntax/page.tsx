import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { CodePreview } from "@/components/code-preview";
import { SAMPLES } from "@/lib/samples";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, H3, LI, Note, P, Table, UL } from "@/components/prose";
import { FIXTURES } from "@/lib/fixtures.generated";

export const metadata: Metadata = {
  title: "Syntax",
  description: "The whole GUML surface: lines, indentation, positionals, attributes, actions, prose.",
};

const counter = FIXTURES.find((f) => f.id === "counter")!;

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/syntax"
      meter={{ label: "spec budget", value: "≤ 3,000 tokens", tone: "iris" }}
      title="Syntax"
      lede="One page per file. Lines are significant, indentation is nesting, and the compiler owns everything conventional."
      toc={[
        { id: "shape", title: "Shape of a file" },
        { id: "lines", title: "Lines and layout" },
        { id: "anatomy", title: "Anatomy of a line" },
        { id: "prose", title: "Prose" },
        { id: "actions", title: "Actions" },
        { id: "rules", title: "Rules that prevent most errors" },
        { id: "grammar", title: "Grammar" },
      ]}
    >
      <H2 id="shape">Shape of a file</H2>
      <CodeBlock code={counter.guml} lang="guml" filename="counter.guml" lines />
      <P>
        Directives first (<C>page</C>, <C>type</C>, <C>data</C>, <C>state</C>), then the element
        tree. There are no imports, no closing tags, and no class names anywhere in the language.
      </P>

      <H2 id="lines">Lines and layout</H2>
      <UL>
        <LI>
          <strong className="text-chalk">Two spaces per level.</strong> Nesting is indentation; an
          element&rsquo;s children are the following lines indented further.
        </LI>
        <LI>
          <strong className="text-chalk">Tabs are an error</strong> (<C>GUML0001</C>). The lexer
          recovers and keeps going so you still see the rest of the file&rsquo;s problems.
        </LI>
        <LI>
          <strong className="text-chalk">Blank lines and </strong>
          <C>{"//"}</C>{" "}
          <strong className="text-chalk">comments never affect layout.</strong> Use them freely to
          group directives.
        </LI>
        <LI>
          <strong className="text-chalk">One statement per line.</strong> There is no line
          continuation, which is what keeps the grammar small enough to teach in context.
        </LI>
      </UL>

      <H2 id="anatomy">Anatomy of a line</H2>
      <CodeBlock
        lang="guml"
        code={`btn Decrement ghost disabled={!count} >count--
│   │         │     │                 └─ action: takes the rest of the line
│   │         │     └─ attribute: name=value
│   │         └─ modifier: from the closed vocabulary
│   └─ positional: the label
└─ tag: must resolve in the component registry`}
      />
      <P>Order is free, with one exception: the action must come last, because it swallows the line.</P>

      <H3>Positionals</H3>
      <Table
        head={["form", "meaning", "example"]}
        rows={[
          ["Word or \"quoted words\"", "label or title text", <C key="a">btn Save</C>],
          ["a known modifier", "presentation intent", <C key="b">primary</C>],
          ["{expr}", "a binding", <C key="c">{"{count}"}</C>],
          ["/path", "a route target", <C key="d">/signup</C>],
          ["#id", "an anchor", <C key="e">#features</C>],
        ]}
      />
      <P>
        A slash only starts a route at the beginning of a token, so <C>$24/mo</C> stays one word.
        That rule exists because pricing tables were the first thing to break without it.
      </P>

      <H3>Attributes</H3>
      <P>
        <C>name=value</C>, where value is a string, number, bare word, or <C>{"{binding}"}</C>. A
        word that is both a modifier and an attribute — <C>disabled</C>, <C>loading</C>,{" "}
        <C>readonly</C>, <C>required</C> — is static when bare and bound when it takes a value:
      </P>
      <CodeBlock
        lang="guml"
        code={`btn Save disabled                 // always disabled
btn Save disabled={!draft.trim()} // disabled while the field is empty`}
      />

      <H2 id="prose">Prose</H2>
      <P>
        For text tags the whole line remainder is prose, taken verbatim. No quoting, no escaping —
        which is why prose costs almost nothing in GUML and why the compression floor on a
        content-heavy page is the copy itself.
      </P>
      <CodePreview {...SAMPLES["syntax.prose"]} />
      <UL>
        <LI>
          Text tags: <C>h</C> <C>h1</C> <C>h2</C> <C>p</C> <C>text</C> <C>metric</C> <C>head</C>{" "}
          <C>empty</C>.
        </LI>
        <LI>
          Any other tag takes content after <C>|</C>.
        </LI>
        <LI>
          Bindings still interpolate inside prose: <C>{"{tasks.open.count}"}</C> above is live.
        </LI>
        <LI>
          Quote prose only when it contains <C>|</C> or <C>=</C>.
        </LI>
      </UL>
      <Note tone="info" title="Why the parser needs the registry">
        <p>
          Whether a line&rsquo;s remainder is prose or structure depends on the tag&rsquo;s kind, and
          kinds live in the registry. That is why parsing and resolution are interleaved rather than
          sequential — a detail worth knowing when reading{" "}
          <A href="/docs/compiler/architecture">the architecture</A>. A text tag with an <C>=</C> on
          the line is parsed structurally instead, which is how <C>text {"{title}"} strike=
          {"{done}"}</C> works.
        </p>
      </Note>

      <H2 id="actions">Actions</H2>
      <P>
        <C>{">"}</C> introduces behaviour and consumes the rest of the line. Statements are separated
        by <C>;</C>.
      </P>
      <CodeBlock
        lang="guml"
        code={`btn Increment primary >count++
btn Reset quiet >count=0
form >tasks.add{title:draft}; draft=""
check {done} >tasks.save`}
      />
      <Note tone="warn" title="The action must be last">
        <p>
          Because <C>{">"}</C> takes the remainder of the line, a modifier written after it gets
          swallowed into the action body. The compiler catches the common case — an action body
          ending in a known modifier reports <C>GUML0022</C> — but do not rely on it.
        </p>
      </Note>

      <H2 id="rules">Rules that prevent most errors</H2>
      <UL>
        <LI>
          Start with <C>page &lt;Name&gt;</C>; it names the emitted component.
        </LI>
        <LI>
          Declare <C>type</C>, <C>data</C> and <C>state</C> before the tree.
        </LI>
        <LI>
          Never write presentation. Use modifiers and let the compiler decide colours and spacing.
        </LI>
        <LI>
          Never hand-write loading, empty, error or rollback logic. Declare the resource with{" "}
          <C>optimistic:</C> and give <C>empty</C> a message.
        </LI>
        <LI>
          Bindings are derived, never assigned. <C>{"{tasks.open.count}"}</C> is a binding, not a
          state.
        </LI>
        <LI>
          If something is not expressible, say so rather than inventing a tag — an unknown tag is a
          compile error, by design.
        </LI>
      </UL>

      <H2 id="grammar">Grammar</H2>
      <P>
        The normative grammar is <C>spec/grammar.ebnf</C> in the repository. It is not
        documentation: it is the artifact fed to grammar prompting and to grammar-constrained
        decoding, so it has to match the parser exactly.
      </P>
      <CodeBlock
        lang="text"
        filename="spec/grammar.ebnf (excerpt)"
        code={`element      ::= TAG positional* attribute* [ action ] [ content ] NEWLINE
                 [ INDENT ( element+ | text_line+ ) DEDENT ] ;

positional   ::= WORD | STRING | NUMBER | MODIFIER | binding | ROUTE | ANCHOR ;
attribute    ::= IDENT "=" value ;
action       ::= ">" REST_OF_LINE ;
content      ::= "|" REST_OF_LINE | REST_OF_LINE ;`}
      />
    </DocPage>
  );
}
