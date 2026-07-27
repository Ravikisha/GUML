import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { C, H2, LI, Note, P, Table, UL } from "@/components/prose";
import { Badge } from "@/components/ui";

export const metadata: Metadata = {
  title: "Diagnostics",
  description: "Every GUML diagnostic code, and why they are designed as a machine interface first.",
};

type Row = [string, string, "error" | "warning", string];

const CODES: Row[] = [
  ["GUML0001", "tabs are not allowed for indentation", "error", "Lexical"],
  ["GUML0002", "unterminated string literal", "error", "Lexical"],
  ["GUML0003", "unterminated `{` group", "error", "Lexical"],
  ["GUML0004", "unexpected character", "error", "Lexical"],
  ["GUML0010", "inconsistent dedent", "warning", "Layout"],
  ["GUML0011", "unexpected indentation", "error", "Layout"],
  ["GUML0020", "expected a tag name at the start of the line", "error", "Syntax"],
  ["GUML0021", "expected a value after `=`", "error", "Syntax"],
  ["GUML0022", "modifier appears after the action and was swallowed by it", "error", "Syntax"],
  ["GUML0030", "unknown tag", "error", "Resolution"],
  ["GUML0031", "unknown modifier", "warning", "Resolution"],
  ["GUML0032", "tag does not accept this attribute", "warning", "Resolution"],
  ["GUML0033", "binding or action refers to something undeclared", "error", "Resolution"],
  ["GUML0040", "state declared more than once", "error", "Semantics"],
  ["GUML0041", "file has no `page` directive", "warning", "Semantics"],
  ["GUML0050", "icon control without a label", "error", "Accessibility"],
  ["GUML0051", "field with no accessible name (placeholder only: warning)", "error", "Accessibility"],
];

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/diagnostics"
      meter={{ label: "codes", value: `${CODES.length} · append-only` }}
      title="Diagnostics"
      lede="Diagnostics are the compiler's primary interface to a model, and its secondary interface to you. That ordering explains most of their design."
      toc={[
        { id: "shape", title: "The shape of one" },
        { id: "json", title: "JSON output" },
        { id: "principles", title: "Four principles" },
        { id: "codes", title: "Every code" },
        { id: "a11y", title: "Accessibility as errors" },
      ]}
    >
      <H2 id="shape">The shape of one</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`error[GUML0030]: unknown tag \`buton\`
  --> page.guml:4:1
   |
 4 | buton Save primary
   | ^^^^^
   = help: did you mean \`btn\`?
   = suggestion: btn`}
      />
      <P>
        Four parts, and each has a job: a stable code to key on, a span that slices to real source
        text, a <C>help</C> line saying how to fix it, and — when the fix is unambiguous — a{" "}
        <C>suggestion</C> that is a literal replacement for the highlighted span.
      </P>

      <H2 id="json">JSON output</H2>
      <P>
        <C>--format json</C> exists for the repair loop, not for humans. A harness feeds this array
        back to the model verbatim, and applies <C>suggestion</C> fields mechanically without
        spending another generation.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo run -q -p guml-cli -- check page.guml --format json`}
      />
      <CodeBlock
        lang="json"
        filename="stdout"
        code={`[
  {
    "code": "unknown_tag",
    "id": "GUML0030",
    "severity": "error",
    "message": "unknown tag \`buton\`",
    "span": { "start": 42, "end": 47, "line": 4, "col": 1 },
    "help": "did you mean \`btn\`?",
    "suggestion": "btn"
  }
]`}
      />

      <H2 id="principles">Four principles</H2>
      <UL>
        <LI>
          <strong className="text-chalk">Complete in one pass.</strong> The parser never returns early.
          Each repair round is a full model generation, so reporting one error at a time turns a
          one-round loop into an N-round one.
        </LI>
        <LI>
          <strong className="text-chalk">Spans must be real.</strong> Every span slices to the text it
          names, with 1-based line and column, so a patch tool can apply a fix positionally. This is
          asserted in tests.
        </LI>
        <LI>
          <strong className="text-chalk">Machine-actionable.</strong> A stable code, plus a literal
          replacement whenever the correction is unambiguous.
        </LI>
        <LI>
          <strong className="text-chalk">Codes are append-only.</strong> They are a public contract:
          the repair loop and the benchmark both key on them, so a code never changes meaning.
        </LI>
      </UL>
      <Note tone="tip" title="Suggestions use optimal string alignment">
        <p>
          A transposed pair counts as one edit, so <C>crad</C> → <C>card</C> is caught. Plain
          Levenshtein scores that as two edits and misses the most common typo class — which is
          exactly the bug the compiler&rsquo;s own tests found on the first run.
        </p>
      </Note>

      <H2 id="codes">Every code</H2>
      <div className="mt-7 overflow-hidden rounded-card border border-white/8">
        {CODES.map(([id, message, severity, group], i) => {
          const prev = CODES[i - 1];
          const newGroup = !prev || prev[3] !== group;
          return (
            <div key={id}>
              {newGroup && (
                <p className="label border-b border-white/8 bg-white/[0.02] px-4 py-2">{group}</p>
              )}
              <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1 border-b border-white/8 px-4 py-3 last:border-0">
                <code className="font-mono text-sm text-chalk">{id}</code>
                <Badge tone={severity === "error" ? "ember" : "neutral"}>{severity}</Badge>
                <p className="min-w-[14rem] flex-1 text-sm text-fog">{message}</p>
              </div>
            </div>
          );
        })}
      </div>

      <H2 id="a11y">Accessibility as errors</H2>
      <P>
        <C>GUML0050</C> and <C>GUML0051</C> are errors, not warnings. A control with no accessible
        name does not compile.
      </P>
      <CodeBlock
        lang="guml"
        code={`btn Delete quiet >tasks.drop                        // fine: has a text label
btn quiet icon=trash >tasks.drop                    // GUML0050
btn quiet icon=trash aria="Delete {title}" >tasks.drop  // fine`}
      />
      <P>
        Severity is graded by how much the compiler can recover on the author&rsquo;s behalf. A
        control inside a repeater row that renders a text binding is named from that binding — the
        same call a person makes when labelling a row checkbox from the row&rsquo;s title — so it
        passes. A field whose only hint is a <C>placeholder</C> is a <em>warning</em>: a placeholder
        disappears on input and is not an accessible name, but it is not nothing either. A control
        with nothing at all is an error.
      </P>
      <P>
        This is where the &ldquo;convention as correctness&rdquo; claim stops being rhetorical. The
        benchmark asserts zero axe-core violations on emitted output, which is only achievable if the
        language refuses to express an unlabelled control in the first place.
      </P>
      <Table
        head={["severity", "meaning", "exit code"]}
        rows={[
          ["error", "the file does not compile", "1"],
          ["warning", "compiles; something is dropped, unsupported, or suspicious", "0"],
        ]}
      />
      <P>
        Unsupported-but-parsed constructs are warnings on purpose: an honest partial compiler is
        useful, while one that quietly emits wrong code is worse than none.
      </P>
    </DocPage>
  );
}
