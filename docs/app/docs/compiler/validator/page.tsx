import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Validator",
  description:
    "Static analysis beyond parsing: unknown mutations, illegal assignment targets, dangling anchors, unused declarations, accessibility as hard errors.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/validator"
      meter={{ label: "diagnostic codes", value: "35" }}
      title="Validator"
      lede="Parsing proves a document is well-formed. The validator proves it means something — and it runs on every `check`, because a model cannot act on a rule nobody told it about."
      toc={[
        { id: "why", title: "Why a separate pass" },
        { id: "run", title: "Running it" },
        { id: "checks", title: "What it checks" },
        { id: "a11y", title: "Accessibility as errors" },
        { id: "unused", title: "Unused declarations" },
        { id: "fix", title: "Machine-applicable fixes" },
        { id: "explain", title: "Explaining a code" },
      ]}
    >
      <H2 id="why">Why a separate pass</H2>
      <P>
        The parser answers &ldquo;is this well-formed GUML?&rdquo;. Most of what goes wrong in
        generated UI is not a syntax error — it is a reference to a resource that was never declared,
        a mutation name that does not exist, a link to an anchor nobody defined. Those documents parse
        cleanly and then behave wrongly, which is the failure mode a compiler exists to prevent.
      </P>
      <Note tone="warn" title="Never silently mis-lower">
        <p>
          The rule the whole project rests on: a construct the compiler cannot lower correctly gets a
          diagnostic and a visible marker in the output. Two silent mis-lowerings were found while
          this pass was being written — an unknown HTTP method quietly became <C>GET</C>, and a
          non-route path produced an empty URL. Both compiled, and both were wrong.
        </p>
      </Note>

      <H2 id="run">Running it</H2>
      <P>
        <C>check</C> already runs the same analysis on a single file. <C>validate</C> exists for
        batches — a fixture directory, or a run of generated output being scored.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml validate <paths>... [--strict] [--format human|json]

guml validate fixtures
# ok: fixtures/a.guml (0 warnings)
# …
# 8 of 8 valid

# --strict turns warnings into failures. This is the CI setting, and the
# setting for scoring generated documents: a warning is a defect there.
guml validate fixtures --strict`}
      />

      <H2 id="checks">What it checks</H2>
      <P>
        Seventeen codes in the <C>0061</C>–<C>0084</C> range, on top of the parser&rsquo;s own. Every
        code is append-only, because the repair loop keys on it — renumbering one would silently
        change what a model repairs.
      </P>
      <Table
        head={["area", "what is rejected"]}
        rows={[
          [
            "resources",
            <>
              a mutation name that does not exist on the resource, an unknown <C key="a">type</C>, a
              method that is not an HTTP verb, a path that is not a route
            </>,
          ],
          [
            "state",
            <>
              assigning to something that is not assignable, a value outside an enumerated domain, a
              duplicate declaration
            </>,
          ],
          [
            "anchors",
            <>
              a <C key="b">link</C> to an anchor nobody defines, and two elements defining the same
              one
            </>,
          ],
          [
            "structure",
            <>
              a repeater with no item template (it would render nothing), attributes whose type is
              wrong for the tag
            </>,
          ],
          [
            "expressions",
            <>
              anything outside the expression grammar — <C key="c">{"{a ? b : c}"}</C>,{" "}
              <C key="d">{"{fetch(url)}"}</C> — reported as <C key="e">GUML0023</C> rather than
              forwarded into emitted JavaScript
            </>,
          ],
        ]}
      />
      <Note>
        <p>
          That last row is also the security boundary. Bindings and actions are deliberately not
          Turing-complete, so &ldquo;not covered by the grammar&rdquo; has to mean{" "}
          <em>rejected</em>, never <em>passed through</em>. Anything genuinely arbitrary goes in a{" "}
          <C>js</C> block, where the compiler says plainly that it guarantees nothing.
        </p>
      </Note>

      <H2 id="a11y">Accessibility as errors</H2>
      <P>
        A control with no accessible name is a <em>hard error</em> (<C>GUML0050</C>,{" "}
        <C>GUML0051</C>), not a lint. The compiler owns ARIA plumbing, so if it cannot work out a
        name it has to say so — silently emitting an unlabelled checkbox would push the defect into
        the artifact the author never reads.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`error[GUML0051]: \`input\` has no accessible name
   |
   = help: a placeholder disappears on input and is not an accessible name; add \`aria="…"\``}
      />
      <P>
        Severity is graded by what the compiler can recover on its own. A row control inside a
        repeater can take its name from the row (<C>aria-label={"{item.title}"}</C>), so that case
        compiles; a standalone input cannot, so it does not.
      </P>

      <H2 id="unused">Unused declarations</H2>
      <P>
        <C>GUML0074</C> and <C>GUML0075</C> report a <C>state</C> or <C>data</C> nothing refers to.
        These are warnings rather than errors — the document still compiles — but they are also what
        licenses the optimizer to elide the declaration entirely, so the author is always told before
        anything is dropped.
      </P>
      <P>
        Liveness is deliberately over-approximate: a name mentioned anywhere inside a <C>js</C> block
        counts as used, because the block is another language and the compiler does not parse it.
        Over-counting suppresses a warning; under-counting would delete a declaration the emitted
        code still references.
      </P>

      <H2 id="fix">Machine-applicable fixes</H2>
      <P>
        A diagnostic can carry a <em>suggestion</em>, and <C>guml fix</C> applies every unambiguous
        one with no model in the loop. A typo&rsquo;d tag is a compile error the repair loop should
        never spend a generation on.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml fix counter.guml --write
# GUML0030 line 4: crad → card`}
      />
      <Note tone="warn" title="A suggestion has to be safe to apply blindly">
        <p>
          Suggestions are only attached where the replacement spans exactly the wrong token. An
          earlier version attached a bare name to a whole-line span, so applying{" "}
          <C>GUML0061</C> replaced an entire line with the word <C>save</C>. Fixes that can corrupt a
          document are worse than no fixes, so the guard is now explicit: a single-token span may not
          be replaced by text containing whitespace, and template text like{" "}
          <C>aria=&ldquo;…&rdquo;</C> is never applied literally.
        </p>
      </Note>

      <H2 id="explain">Explaining a code</H2>
      <P>
        Every one of the 35 codes has a title and a written explanation of{" "}
        <em>why the rule exists</em>, reachable without leaving the terminal.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml explain GUML0074      # also accepts 0074, or 74`}
      />
      <P>
        See <A href="/docs/compiler/diagnostics">Diagnostics</A> for the full code list and the JSON
        shape the repair loop consumes.
      </P>
      <UL>
        <LI>
          <C>check</C> — one file, human or JSON output
        </LI>
        <LI>
          <C>validate</C> — many files, with <C>--strict</C> for CI
        </LI>
        <LI>
          <C>fix</C> — apply the unambiguous suggestions
        </LI>
        <LI>
          <C>explain</C> — what a code means and why
        </LI>
      </UL>
    </DocPage>
  );
}
