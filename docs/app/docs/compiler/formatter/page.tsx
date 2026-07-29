import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Formatter",
  description:
    "guml fmt: a line-stream rewriter that runs on invalid input, fixes whitespace errors with no model call, and has a canonical mode for deduplication.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/formatter"
      meter={{ label: "cost of a whitespace fix", value: "0 tokens", tone: "mint" }}
      title="Formatter"
      lede="Most formatters exist for taste. This one exists because a tab character should not cost a model call — it runs before the parser, on input that does not compile."
      toc={[
        { id: "why", title: "Why it runs first" },
        { id: "run", title: "Running it" },
        { id: "rules", title: "What it rewrites" },
        { id: "prose", title: "What it never touches" },
        { id: "canonical", title: "Canonical mode" },
        { id: "guarantees", title: "The two guarantees" },
      ]}
    >
      <H2 id="why">Why it runs first</H2>
      <P>
        In the repair loop, a document that fails to parse costs a full generation to fix. A large
        share of those failures are whitespace: a tab instead of two spaces, a ragged indent, trailing
        space after an action. None of that needs a model.
      </P>
      <P>
        So the formatter is placed <em>before</em> the parser and is built to run on input that does
        not compile. That inverts the usual design: it cannot assume a valid AST, so it is a line
        stream rewriter rather than a pretty-printer over a tree.
      </P>
      <Note tone="warn" title="A line the lexer choked on is reprinted verbatim">
        <p>
          Re-emitting tokens from a failed lex would turn a small syntax error into a mangled line and
          lose the author&rsquo;s work. Any line with a lexical diagnostic is passed through
          untouched, and only its indentation is corrected.
        </p>
      </Note>

      <H2 id="run">Running it</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml fmt [FILES]... [-w|--write] [--check] [--canonical]

# stdin → stdout when no file is given, which is what editors want
cat counter.guml | guml fmt

# rewrite in place
guml fmt fixtures/*.guml --write

# CI: exit 1 if anything is unformatted, print nothing on success
guml fmt fixtures/*.guml --check`}
      />

      <H2 id="rules">What it rewrites</H2>
      <Table
        head={["input", "output", "why"]}
        rows={[
          ["a tab for indentation", "two spaces", <>it is <C key="a">GUML0001</C>; free to fix</>],
          ["four-space or ragged indent", "two spaces per level", "the indent stack decides depth, so this is normalisation, not reflow"],
          ["btn    Add     primary", "btn Add primary", "structured lines collapse to single spaces"],
          [`placeholder = "Add a task…"`, `placeholder="Add a task…"`, "attributes lose their spacing"],
          ["mutation columns under `data`", "aligned", "a `data` block reads as a table, so it is printed as one"],
          ["blank lines between sections", "exactly one", "declaration groups are separated, elements are not"],
        ]}
      />

      <H2 id="prose">What it never touches</H2>
      <P>
        Prose is taken verbatim by the lexer, which is why it costs almost nothing in tokens — and it
        means the formatter must not touch it either. Collapsing runs of spaces inside a sentence
        would be silently editing content.
      </P>
      <CodeBlock
        lang="guml"
        code={`p Two  spaces   inside stay exactly as written.`}
      />
      <P>
        The same applies inside a <C>js</C> or <C>raw</C> block, where the body is another language
        entirely: indentation there is meaningful, so it is reproduced rather than normalised. A
        template literal would change value otherwise.
      </P>

      <H2 id="canonical">Canonical mode</H2>
      <P>
        <C>--canonical</C> strips every discretionary byte: comments, blank lines, declaration order.
        Two documents that mean the same thing become byte-identical.
      </P>
      <P>
        That is not a style preference, it is measurement infrastructure. Inter-run consistency —
        &ldquo;does the model produce the same UI twice?&rdquo; — cannot be measured if the comparison
        is confounded by comment placement, and deduplicating generated documents needs a normal form.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml fmt run-1.guml --canonical | sha256sum
guml fmt run-2.guml --canonical | sha256sum
# identical hashes ⇒ the two generations mean the same thing`}
      />

      <H2 id="guarantees">The two guarantees</H2>
      <UL>
        <LI>
          <strong>Idempotence.</strong> Formatting twice gives the same result as once, in both modes.
          Checked over thousands of generated documents, most of which are invalid.
        </LI>
        <LI>
          <strong>Meaning is preserved.</strong> For any document that parses,{" "}
          <C>ast(fmt(x)) == ast(x)</C>. Compared structurally rather than by span, because
          re-indenting moves every byte offset.
        </LI>
      </UL>
      <P>
        Both are enforced by the fuzz corpus rather than by review — see{" "}
        <A href="/docs/compiler/architecture">Architecture</A>. The second is the one that matters: a
        formatter that changes what a document means is a compiler bug wearing a cosmetic disguise.
      </P>
      <Note>
        <p>
          The formatter also owns syntax classification. <C>guml highlight</C> uses the same lexer and
          registry to answer &ldquo;what colour is this byte&rdquo;, because prose-versus-structure
          depends on the tag — a regex grammar cannot know that. The docs site&rsquo;s highlighter is
          held to the compiler&rsquo;s answer span for span in CI.
        </p>
      </Note>
    </DocPage>
  );
}
