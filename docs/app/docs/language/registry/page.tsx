import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Pkg, UL } from "@/components/prose";
import { Badge } from "@/components/ui";
import { COMPONENTS } from "@/lib/fixtures.generated";

export const metadata: Metadata = {
  title: "Component registry",
  description: "The closed, validated tag vocabulary — and the prompt slice a model actually receives.",
};

const KIND_TONE = {
  Container: "iris",
  Text: "neutral",
  Control: "ember",
  Field: "ember",
  Repeater: "mint",
} as const;

const ORDER = ["Container", "Text", "Control", "Field", "Repeater"] as const;

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/registry"
      meter={{ label: "vocabulary", value: `${COMPONENTS.length} components`, tone: "iris" }}
      title="Component registry"
      lede="The registry is the closed vocabulary of tags. It is simultaneously the compiler's type system for elements and the context a model is given — which is why an unknown tag is a compile error rather than a runtime surprise."
      toc={[
        { id: "vocabulary", title: "The vocabulary" },
        { id: "prompt-slice", title: "The prompt slice" },
        { id: "why-closed", title: "Why closed" },
        { id: "entries", title: "What an entry carries" },
        { id: "loading", title: "Loading your own" },
        { id: "packages", title: "The two packages that ship" },
        { id: "schema", title: "The full field reference" },
      ]}
    >
      <H2 id="vocabulary">The vocabulary</H2>
      <P>
        Generated for this page by running <C>guml registry</C> against the compiler, so the table
        cannot drift from what the parser will accept.
      </P>
      {ORDER.map((kind) => {
        const items = COMPONENTS.filter((c) => c.kind === kind);
        if (!items.length) return null;
        return (
          <div key={kind} className="mt-8">
            <div className="mb-3 flex items-center gap-3">
              <Badge tone={KIND_TONE[kind]}>{kind}</Badge>
              <span className="label">{items.length} tags</span>
            </div>
            <div className="overflow-hidden rounded-card border border-line">
              {items.map((c) => (
                <div
                  key={c.name}
                  className="grid gap-1 border-b border-line px-4 py-3 last:border-0 sm:grid-cols-[7rem_1fr] sm:gap-4"
                >
                  <code className="font-mono text-sm text-chalk">{c.name}</code>
                  <p className="text-sm leading-relaxed text-fog">{c.doc}</p>
                </div>
              ))}
            </div>
          </div>
        );
      })}

      <H2 id="prompt-slice">The prompt slice</H2>
      <P>
        A model never receives the whole registry. It receives the entries a task actually needs,
        which is what keeps prompt cost sublinear in vocabulary size — the difference between a
        40-tag language and a 400-tag design system being roughly nothing at the prompt.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo run -q -p guml-cli -- registry --tags btn,card,list

btn (Control) — Button. First positional is the label; \`>\` gives the action.
card (Container) — Bordered surface grouping related content. Optional title as first positional.
list (Repeater) — Renders one child template per item of a resource. Loading, empty and error states are compiled in.`}
      />
      <P>
        Each doc line is written for a model rather than for a docs site: terse, and it states{" "}
        <em>when</em> to use the tag. Prescriptive descriptions measurably outperform descriptive
        ones for component selection, and every word costs prompt tokens on every request.
      </P>

      <H2 id="why-closed">Why closed</H2>
      <UL>
        <LI>
          <strong className="text-chalk">Hallucination becomes a compile error.</strong> An invented
          component cannot reach runtime; it fails at <C>guml check</C> with a suggestion attached.
        </LI>
        <LI>
          <strong className="text-chalk">The parser needs it.</strong> A tag&rsquo;s kind decides
          whether the line remainder is prose or structure, so resolution happens during parsing.
        </LI>
        <LI>
          <strong className="text-chalk">It is the coverage ceiling.</strong> If the registry cannot
          express something, the author needs an escape hatch — so registry coverage is tracked as a
          number, not a feeling.
        </LI>
      </UL>
      <Note tone="info" title="Prior art worth naming">
        <p>
          A validated closed tag set is Markdoc&rsquo;s core idea, and a host-approved component
          catalog is how Google&rsquo;s A2UI keeps agent-generated UI safe. GUML&rsquo;s contribution
          is not the closed catalog — it is the token surface over it and the compiler behind it. See{" "}
          <A href="/research/prior-art">prior art</A>.
        </p>
      </Note>

      <H2 id="entries">What an entry carries</H2>
      <CodeBlock
        lang="json"
        filename="an entry, as a host would write it"
        code={`{
  "name": "btn",
  "kind": "control",
  "level": "core",
  "attrs": ["busy", "type"],
  "a11y": { "requires_label": true, "focusable": true },
  "doc": "Button. First positional is the label; \`>\` gives the action."
}`}
      />
      <UL>
        <LI>
          <C>kind</C> — drives parsing, not just rendering.
        </LI>
        <LI>
          <C>attrs</C> — accepted beyond the global set; anything else warns (<C>GUML0032</C>).
        </LI>
        <LI>
          <C>level</C> — <A href="/docs/language/levels">core or app</A>. Defaults to <C>core</C>, so a
          hand-written entry is markup unless it says otherwise.
        </LI>
        <LI>
          <C>a11y</C> — the accessibility contract, as data. <C>requires_label</C> makes a control with
          no text label and no <C>aria</C> a hard error (<C>GUML0050</C>); <C>role</C>,{" "}
          <C>focusable</C> and <C>announces_state</C> state what the compiler must guarantee. This is
          what extends the accessibility promise past the builtin vocabulary — a third-party component
          declares its contract instead of the guarantee stopping at tags we shipped.
        </LI>
        <LI>
          <C>doc</C> — the line that goes into a model&rsquo;s context.
        </LI>
      </UL>

      <H2 id="loading">Loading your own</H2>
      <P>
        A registry is a JSON document, and <C>--registry</C> merges it with the builtins. Both shapes are
        accepted: <C>{"{\"components\": [ … ]}"}</C>, or a bare array. The builtin vocabulary is itself
        such a document (<C>crates/guml-registry/components.json</C>), which is deliberate: your package
        travels the same load path the builtins do, so there is no second, better-supported path you are
        not on.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml registry --validate design-system.json   # audit: every problem at once
guml add design-system.json                   # audit, then record in guml.json
guml check page.guml                          # vocabulary comes from guml.json
guml build page.guml --registry other.json    # a one-off override still wins`}
      />
      <P>
        <C>guml add</C> is the difference between a design system being loadable and being{" "}
        <em>usable</em>. Before <C>guml.json</C>, every <C>check</C>, every <C>build</C>, the editor and
        CI each had to be handed the same <C>--registry</C> path — and the moment one of them was not,
        that call compiled against a different vocabulary. A document valid in the editor and invalid in
        CI is the worst failure a closed vocabulary can have, because the entire point of closing it is
        that everyone agrees what the words are.
      </P>
      <Note tone="info" title="An entry declares what it lowers to">
        <P>
          Validation without codegen is half a feature: <C>guml check</C> accepted a document using{" "}
          <C>callout</C> and <C>guml build</C> warned <em>does not yet lower tag `callout`</em>. It was
          right to — nothing had told it what a <C>callout</C> is.
        </P>
        <CodeBlock
          lang="json"
          filename="design-system.json"
          code={`{
  "name": "@acme/design-system",
  "version": "2.1.0",
  "components": [
    { "name": "figure-block", "kind": "container", "doc": "Figure with a caption.",
      "element": "figure" },

    { "name": "callout", "kind": "container", "doc": "Highlighted aside for a caveat.",
      "attrs": ["tone"], "positionals": ["title"],
      "element": "Callout", "import": "@acme/design-system" }
  ]
}`}
        />
        <P>
          A <strong>lowercase</strong> <C>element</C> is an HTML element and is emitted directly with the
          theme&rsquo;s classes. A <strong>PascalCase</strong> one is your own component: the compiler
          emits <C>{"<Callout tone=\"warning\">"}</C> and generates the import, for the tags the document
          actually uses. That is the right division of labour — a compiler that tried to reimplement your
          component would get it subtly wrong, and you already have the implementation. Case is the
          signal, the same rule JSX itself uses, so there is no second convention to learn.
        </P>
      </Note>
      <P>
        This is what makes the vocabulary extensible without a fork. Until it existed, every new tag meant
        recompiling the compiler — a requirement no markup language can impose on the applications
        embedding it.
      </P>
      <Note tone="warn" title="Three things are rejected rather than accepted quietly">
        <UL>
          <LI>
            <strong>Shadowing a builtin.</strong> A registry may add tags; it may not redefine{" "}
            <C>btn</C>. Otherwise the same document renders differently depending on which registry was
            loaded, with no diagnostic — the exact failure a closed vocabulary exists to prevent.
          </LI>
          <LI>
            <strong>An unusable name.</strong> The lexer reads a tag as a bare lowercase word, so{" "}
            <C>My Tag</C> could be registered and never matched by any document.
          </LI>
          <LI>
            <strong>An app-level entry in a core host.</strong> Skipped, not merged, so a registry cannot
            smuggle behaviour past a host that asked for markup only.
          </LI>
        </UL>
      </Note>
      <P>
        <C>Registry::to_json</C> serialises the vocabulary a host accepts, which is how a host publishes
        its contract rather than describing it in prose.
      </P>

      <H2 id="packages">The two packages that ship</H2>
      <P>
        Some components cannot be builtins, because no honest lowering exists without a decision the
        registry should not make for you. A <C>chart</C> emitting a bare <C>{"<div>"}</C> would be a
        promise the compiler does not keep, and a builtin has to lower in <em>all seven</em> backends
        including the no-JavaScript one. These are exactly what <C>element</C> and <C>import</C> are for.
      </P>
      <UL>
        <LI>
          <Pkg name="@guml/widgets" /> — <C>chart</C>, <C>calendar</C>, <C>date</C>, <C>upload</C>,{" "}
          <C>command</C>. Five entries, kept small so the whole package can be read end to end. This is
          the worked example the rest of this page describes.
        </LI>
        <LI>
          <Pkg name="@guml/shadcn" /> — 26 tags over all 61 shadcn/ui components, for the ones GUML has no
          builtin for: <C>popover</C>, <C>tooltip</C>, <C>dropdown</C>, <C>collapsible</C>,{" "}
          <C>carousel</C>, <C>textarea</C>, <C>radio</C>, <C>slider</C>, <C>combobox</C> and the rest.
          Roughly 600 estimated prompt tokens for the whole slice.
        </LI>
      </UL>
      <P>
        Both are checked the same way, and the check is worth copying if you write your own: compile the
        package&rsquo;s own example and typecheck the <em>emitted</em> TSX against the real components.
        Declaring an attribute proves nothing about whether the component accepts it. That gate found
        three compiler bugs on <Pkg name="@guml/widgets" /> and four on <Pkg name="@guml/shadcn" />, including a tag
        declared for a component that does not exist upstream and a <C>radio</C> emitted with no options
        at all — bound correctly to a state and offering the reader no way to change it.
      </P>
      <Note tone="warn" title="Adding a tag is not purely additive">
        <P>
          A <C>def</C> may not shadow a tag, so a document that defined its own <C>stat</C> component
          stopped compiling the release <C>stat</C> became builtin. Growing the vocabulary from 28 to 49
          broke exactly that in three places in this repository. The failure mode is the acceptable one —
          compile time, <C>GUML0093</C>, the name in the message, a one-word fix — but it is a breakage
          rather than an addition for any document already using the name.
        </P>
        <P>
          So <strong>pin it</strong>. A <C>guml.json</C> registry entry may be{" "}
          <C>{'{ "path": "./vendor/widgets", "version": "0.1.0" }'}</C>, and loading <em>fails</em> rather
          than warns when the package declares a different version — a document compiled against the wrong
          vocabulary is not a degraded build, it is a different document. <C>guml add</C> writes the pin for
          you, from the version it just audited.
        </P>
        <P>
          Exact equality, not a range. A range needs a resolver, a lockfile and a policy for what
          &ldquo;compatible&rdquo; means for a vocabulary — and semver&rsquo;s answer, that additive changes
          are a minor bump, is the one this project has evidence against. This paragraph used to end by
          advising you to pin, before any of it existed; advice a tool cannot carry out is worse than no
          advice.
        </P>
      </Note>
      <H2 id="schema">The full field reference</H2>
      <P>
        This page is the tour. <C>spec/REGISTRY.md</C> is the contract: every field, every default, every
        rejection, and what each one is checked by. It covers the parts a package author needs and this page
        only gestures at — <C>children</C> constraints (<C>allow</C> / <C>deny</C> / <C>require</C>, with{" "}
        <C>deny: [&quot;*&quot;]</C> for a leaf), <C>capabilities</C>, <C>slots</C>, <C>since</C>, and what{" "}
        <C>guml registry --validate</C> reports before you install anything.
      </P>
    </DocPage>
  );
}
