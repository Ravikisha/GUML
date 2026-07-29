import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, UL } from "@/components/prose";
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
        { id: "packages", title: "Still planned" },
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
          <A href="/docs/research/prior-art">prior art</A>.
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
        accepted: <C>{"{\"components\": [ … ]}"}</C>, or a bare array.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml check page.guml --registry design-system.json
guml build page.guml --registry design-system.json --core`}
      />
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

      <H2 id="packages">Still planned</H2>
      <UL>
        <LI>
          <A href="/docs/compiler/themes">Theme packs</A> mapping modifiers to an organisation&rsquo;s
          design tokens — shipped; per-tag token metadata is not.
        </LI>
        <LI>Per-entry token-cost metadata, used by the optimizer and reported by the benchmark.</LI>
        <LI>
          A retrieval layer that picks the slice from a task description instead of a hand-written
          list.
        </LI>
        <LI>A version field on a registry document, so a host can pin the vocabulary it loaded.</LI>
      </UL>
      <P>
        The React backend maps onto shadcn/ui primitives, so &ldquo;grow the registry&rdquo; mostly
        means describing components that already exist rather than designing new ones.
      </P>
    </DocPage>
  );
}
