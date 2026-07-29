import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { CodePreview } from "@/components/code-preview";
import { SAMPLES } from "@/lib/samples";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";
import { Badge } from "@/components/ui";
import { MODIFIERS } from "@/lib/fixtures.generated";

export const metadata: Metadata = {
  title: "Modifiers",
  description: "Semantic presentation vocabulary. The compiler owns the mapping to CSS.",
};

const GROUPS: Array<[string, string[], string]> = [
  ["intent", ["primary", "secondary", "outline", "ghost", "quiet", "danger", "featured"], "what this element is for"],
  ["size", ["xs", "sm", "md", "lg", "xl"], "relative scale, not pixels"],
  ["layout", ["center", "start", "end", "between", "wrap", "tight", "loose", "full"], "alignment and density"],
  ["state", ["disabled", "loading", "readonly", "required"], "also available as bound attributes"],
];

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/modifiers"
      meter={{ label: "vocabulary", value: `${MODIFIERS.length} modifiers`, tone: "iris" }}
      title="Modifiers"
      lede="Modifiers say what an element is for. They never say what it looks like — that mapping belongs to the compiler, and it is the single largest token saving in the language."
      toc={[
        { id: "why", title: "Why not classes" },
        { id: "vocabulary", title: "The vocabulary" },
        { id: "bare-vs-bound", title: "Bare vs bound" },
        { id: "themes", title: "Themes" },
        { id: "typos", title: "Typos" },
      ]}
    >
      <H2 id="why">Why not classes</H2>
      <P>
        In the landing-page fixture, Tailwind class strings account for roughly a third of
        React&rsquo;s tokens. They are also the part a model is most likely to get subtly wrong:
        contrast that fails, focus rings dropped, spacing that drifts between sections.
      </P>
      <CodePreview {...SAMPLES["modifiers.intent"]} />
      <P>
        Three tokens of intent in, fourteen tokens of presentation out — and the disabled state and
        hover treatment come along whether or not the author remembered them.
      </P>

      <H2 id="vocabulary">The vocabulary</H2>
      <P>
        Closed, and read straight out of the compiler for this page. Anything outside it is not a
        modifier: it is treated as a label, and if it is a near-miss for a real modifier you get a
        warning with the correction.
      </P>
      {GROUPS.map(([group, items, note]) => (
        <div key={group} className="mt-7">
          <p className="label mb-3">
            {group} — {note}
          </p>
          <div className="flex flex-wrap gap-2">
            {items.map((m) => (
              <Badge key={m} tone={group === "intent" ? "ember" : "neutral"}>
                {m}
              </Badge>
            ))}
          </div>
        </div>
      ))}

      <H2 id="bare-vs-bound">Bare vs bound</H2>
      <P>
        Four words appear in both the modifier vocabulary and the global attribute list. Bare, they
        are static; with a value, they take a binding. The parser decides by looking for <C>=</C>.
      </P>
      <CodeBlock
        lang="guml"
        code={`btn Save disabled                  // always disabled
btn Save disabled={!draft.trim()}  // disabled while the field is empty
list tasks loading                 // force the skeleton, for a screenshot
input email required`}
      />

      <H2 id="themes">Themes</H2>
      <P>
        Because modifiers are semantic, re-theming is a compiler concern rather than a find-and-replace
        across pages. The React backend keeps the whole mapping in one table; a theme pack swaps it
        wholesale, so <C>primary</C> means <em>your</em> primary.
      </P>
      <Table
        head={["modifier", "React backend output (default theme)"]}
        rows={[
          [<C key="a">primary</C>, <C key="b">bg-slate-900 text-white hover:bg-slate-800</C>],
          [<C key="c">outline</C>, <C key="d">border border-slate-300 text-slate-700 hover:bg-slate-50</C>],
          [<C key="e">quiet</C>, <C key="f">text-slate-500 hover:text-slate-900</C>],
          [<C key="g">danger</C>, <C key="h">bg-red-600 text-white hover:bg-red-700</C>],
        ]}
      />
      <Note tone="info" title="This is the enterprise argument">
        <p>
          A prompt asking a model to &ldquo;follow our design system&rdquo; is a request. A compiler
          that owns the mapping is a guarantee — which is why{" "}
          <A href="/docs/compiler/backends">the backend</A> treats the table as part of the language,
          not as a helper.
        </p>
      </Note>

      <H2 id="typos">Typos</H2>
      <P>
        Modifier suggestions use optimal string alignment, so a transposed pair counts as one edit —{" "}
        <C>primry</C> resolves to <C>primary</C>, and <C>crad</C> resolves to <C>card</C>. Plain
        Levenshtein scores a transposition as two edits and would have missed the most common typo
        class entirely.
      </P>
      <UL>
        <LI>
          An unknown <strong className="text-chalk">tag</strong> is an error (<C>GUML0030</C>).
        </LI>
        <LI>
          An unknown lowercase word that is one edit from a modifier is a warning (<C>GUML0031</C>),
          because it might legitimately be a label.
        </LI>
      </UL>
    </DocPage>
  );
}
