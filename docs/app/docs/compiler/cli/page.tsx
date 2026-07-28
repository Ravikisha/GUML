import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "CLI reference",
  description: "Every guml subcommand: check, build, ast, lex, tokens, registry.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/cli"
      meter={{ label: "subcommands", value: "6" }}
      title="CLI reference"
      lede="Six subcommands. Examples use the long cargo form so they work in a fresh clone; install the binary and drop the prefix if you prefer."
      toc={[
        { id: "check", title: "check" },
        { id: "build", title: "build" },
        { id: "ast", title: "ast" },
        { id: "lex", title: "lex" },
        { id: "tokens", title: "tokens" },
        { id: "registry", title: "registry" },
        { id: "exit-codes", title: "Exit codes" },
      ]}
    >
      <H2 id="check">check</H2>
      <P>Parse and validate without emitting anything. The fast path for editors and repair loops.</P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml check <file> [--format human|json]

guml check fixtures/a.guml
# ok: fixtures/a.guml (0 warnings)

guml check fixtures/b.guml --format json
# [ … full diagnostic array … ]`}
      />
      <Table
        head={["flag", "default", "notes"]}
        rows={[
          [
            <C key="a">--format</C>,
            <C key="b">human</C>,
            <>
              <C>json</C> prints to stdout for a harness; <C>human</C> renders carets to stderr
            </>,
          ],
        ]}
      />

      <H2 id="build">build</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml build <file> [-b react] [-o dir] [--format human|json]

# to stdout
guml build fixtures/a.guml

# to a directory, with a size readout
guml build fixtures/a.guml -o out
# wrote out/Counter.tsx
#
# source ~63 tokens -> emitted ~382 tokens (6.1x expansion, estimates only)`}
      />
      <Table
        head={["flag", "default", "notes"]}
        rows={[
          [<C key="a">-b, --backend</C>, <C key="b">react</C>, "only react in v0.1"],
          [<C key="c">-o, --out</C>, "stdout", "directory is created if missing"],
        ]}
      />
      <Note tone="warn" title="The expansion ratio is an estimate">
        <p>
          That readout uses a ~3.6 chars/token heuristic. For anything that goes in a README or a
          paper, count with the target model&rsquo;s own tokenizer — see{" "}
          <A href="/docs/research/measurements">measurements</A>.
        </p>
      </Note>

      <H2 id="ast">ast</H2>
      <P>
        Dumps the AST as JSON. Useful for tooling, and for the benchmark&rsquo;s consistency metric:
        comparing ASTs across repeated generations measures inter-run variance without being fooled by
        whitespace.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml ast fixtures/a.guml | head -30`}
      />
      <CodeBlock
        lang="json"
        filename="stdout (excerpt)"
        code={`{
  "page": { "name": "Counter", "span": { "line": 1, "col": 1 } },
  "states": [
    { "name": "count", "init": { "Num": 0.0 }, "domain": [] }
  ],
  "tree": [
    { "tag": "card", "positionals": [ { "Modifier": "sm" } ] }
  ]
}`}
      />

      <H2 id="lex">lex</H2>
      <P>
        Prints the token stream per line. Reach for this when a line parses in a way you did not
        expect — usually a swallowed action or a prose-versus-structure surprise.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml lex fixtures/a.guml

  1 indent=0  [Word("page"), Word("Counter")]
  9 indent=4  [Word("btn"), Word("Decrement"), Word("ghost"),
              Word("disabled"), Eq, Brace("!count"), Action("count--")]`}
      />

      <H2 id="tokens">tokens</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml tokens fixtures/*.guml

file                            bytes   lines      ~tokens
a.guml                            242      11           68
b.guml                            655      24          182
c.guml                           1512      42          420
TOTAL                                                  670

note: ~3.6 chars/token heuristic. For anything that goes in a paper or a
README, count with the target model's own tokenizer.`}
      />
      <Note tone="warn" title="Estimates, and labelled as such">
        <p>
          The heuristic reads a little high against the measured cl100k_base figures (64 / 173 / 376).
          It is a dev-loop convenience, never a published number.
        </p>
      </Note>

      <H2 id="registry">registry</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`# everything
guml registry

# just the slice a prompt needs
guml registry --tags btn,card,list`}
      />
      <P>
        The second form is the retrieval path: a model receives only the entries a task needs, which
        is what keeps prompt cost from scaling with vocabulary size. See{" "}
        <A href="/docs/language/registry">the registry</A>.
      </P>

      <H2 id="exit-codes">Exit codes</H2>
      <Table
        head={["code", "meaning"]}
        rows={[
          ["0", "success — warnings do not fail the command"],
          ["1", "at least one error diagnostic, or an unreadable file / unknown backend"],
        ]}
      />
      <UL>
        <LI>
          Diagnostics go to <strong className="text-chalk">stderr</strong> in human format, so{" "}
          <C>guml build f.guml {">"} out.tsx</C> gives a clean file.
        </LI>
        <LI>
          With <C>--format json</C> the diagnostic array goes to{" "}
          <strong className="text-chalk">stdout</strong>, because a harness is the reader.
        </LI>
      </UL>
    </DocPage>
  );
}
