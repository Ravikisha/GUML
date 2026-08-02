import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "CLI reference",
  description:
    "Every guml subcommand: check, build, validate, fix, repair, fmt, add, explain, where, highlight, ast, lex, tokens, registry.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/cli"
      meter={{ label: "subcommands", value: "12" }}
      title="CLI reference"
      lede="Twelve subcommands. Examples use the bare binary; prefix with `cargo run -q -p guml-cli --` to run them in a fresh clone."
      toc={[
        { id: "check", title: "check" },
        { id: "build", title: "build" },
        { id: "validate", title: "validate" },
        { id: "fix", title: "fix" },
        { id: "repair", title: "repair" },
        { id: "fmt", title: "fmt" },
        { id: "add", title: "add" },
        { id: "explain", title: "explain" },
        { id: "where", title: "where" },
        { id: "highlight", title: "highlight" },
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
# source ~68 tokens -> emitted ~485 tokens (7.1x expansion, estimates only)`}
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
          <A href="/research/measurements">measurements</A>.
        </p>
      </Note>

      <H2 id="validate">validate</H2>
      <P>
        The same analysis as <C>check</C>, built for batches: a directory is searched for{" "}
        <C>*.guml</C>. <C>--strict</C> turns warnings into failures, which is the CI setting and the
        setting for scoring generated output.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml validate <paths>... [--strict] [--format human|json]

guml validate fixtures
# 8 of 8 valid`}
      />
      <P>
        See <A href="/docs/compiler/validator">Validator</A> for what it checks.
      </P>

      <H2 id="fix">fix</H2>
      <P>
        Applies every unambiguous diagnostic suggestion with no model in the loop. A typo&rsquo;d tag
        should never cost a generation.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml fix <files>... [-w|--write]

guml fix counter.guml --write
# GUML0030 line 4: crad → card`}
      />

      <H2 id="repair">repair</H2>
      <P>
        What to run on raw model output. <C>fix</C> only applies edits the compiler described, so it still
        fails on the <em>packaging</em> a model wraps around a document — a code fence, a markdown rule, a
        closing &ldquo;This page counts clicks.&rdquo; sentence. <C>repair</C> strips those first, then
        formats, then fixes: three deterministic layers, bounded at three re-check rounds, and still no
        model call.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml repair [FILES]... [-w|--write] [--rounds 3] [--format human|json]

# a generation pipeline pipes straight in
llm-generate | guml repair > page.guml

# what each layer did, for telemetry
guml repair page.guml --format json`}
      />
      <P>
        A fenced, prose-wrapped, HTML-shaped generation goes from seven errors to zero this way. Every
        layer is guarded: one that would <em>raise</em> the error count is discarded rather than kept —
        the same rule the measured model round uses, applied to the free layers too, because
        &ldquo;deterministic&rdquo; is not the same as &ldquo;always an improvement&rdquo;.
      </P>
      <Note tone="info" title="Why this is not a flag on fix">
        <C>fix</C> only ever applies edits the compiler described precisely. <C>repair</C> also{" "}
        <em>deletes</em> — a fence, trailing prose. That is a different promise, and it should not become
        the default behaviour of an existing command by accident. The exit code is non-zero when errors
        remain, so a pipeline can branch on &ldquo;does this need a model round&rdquo; without parsing
        anything.
      </Note>

      <H2 id="fmt">fmt</H2>
      <P>
        Reads stdin when no file is given, which is what editors want. Runs on input that does not
        compile, because fixing a tab character should not cost a model call.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml fmt [FILES]... [-w|--write] [--check] [--canonical]

guml fmt fixtures/*.guml --check     # CI: exit 1 if unformatted
cat counter.guml | guml fmt          # stdin → stdout`}
      />
      <P>
        See <A href="/docs/compiler/formatter">Formatter</A>, including what <C>--canonical</C> is for.
      </P>

      <H2 id="add">add</H2>
      <P>
        Installs a <A href="/docs/language/registry">registry package</A> into <C>guml.json</C> after
        auditing it — so the project&rsquo;s vocabulary is stated once and the editor, the formatter,{" "}
        <C>check</C> and CI cannot disagree about what the words are.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml add <path> [--dry-run]
guml registry --validate <path>      # audit without installing
guml registry --docs > VOCABULARY.md # generated reference for your vocabulary

guml add ./design-system.json
# @acme/design-system 2.1.0: 2 component(s)
#   callout figure-block
#   ~31 est. prompt tokens for the whole package
#   no errors
# added ./design-system.json to guml.json`}
      />
      <P>
        The audit reports <em>every</em> problem at once, for the same reason the parser does: an author
        fixing five entries should not need five runs. It is also checked against the vocabulary already
        installed — two packages can each be valid alone and collide with each other, and finding that out
        at install time is the whole reason to have an install step.
      </P>
      <Note tone="warn" title="A path, never a URL">
        A registry decides which tags a document may use and which classes the compiler emits, so
        resolving one over the network at build time would make compiler output depend on a remote server.
        That is the wrong trade for a project whose claim is reliability. Packages arrive the way any
        dependency does — a file, a vendored directory, <C>node_modules</C> — and are installed by an
        explicit command rather than fetched implicitly.
      </Note>

      <H2 id="explain">explain</H2>
      <P>
        What a diagnostic code means and <em>why the rule exists</em>. Accepts the full id or just the
        number.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml explain GUML0074
guml explain 0074
guml explain 74`}
      />

      <H2 id="where">where</H2>
      <P>
        Which GUML line produced a line of emitted code, resolved through the source map — so a stack
        trace can be answered without reading VLQ by eye.
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
        See <A href="/docs/compiler/source-maps">Source maps</A>.
      </P>

      <H2 id="highlight">highlight</H2>
      <P>
        Classifies every byte for syntax highlighting using the real lexer and registry, because
        prose-versus-structure depends on the tag. This is what the docs site, the playground and the
        language server all colour from.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml highlight counter.guml
# [{"start":0,"end":4,"line":1,"class":"directive","lsp":"keyword"}, …]`}
      />

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
          The heuristic reads a little high against the measured cl100k_base figures (64 / 178 / 376).
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
