import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "MCP server",
  description:
    "Give a model the compiler instead of a prompt: guml mcp serves the vocabulary, the checker and the repair loop as tools.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/mcp"
      meter={{ label: "command", value: "guml mcp", tone: "iris" }}
      title="MCP server"
      lede="GUML has no training data by construction, so using it meant ~3,000 tokens of spec in every system prompt for a language the model has never seen. This removes that — and gives the model something no prompt can: the compiler that will actually build its output."
      toc={[
        { id: "why", title: "The problem it solves" },
        { id: "install", title: "Setup" },
        { id: "tools", title: "The tools" },
        { id: "loop", title: "What a session looks like" },
        { id: "design", title: "Design notes" },
      ]}
    >
      <H2 id="why">The problem it solves</H2>
      <P>
        Every other way of using GUML starts by teaching it: the spec and the vocabulary go in the
        system prompt, ~3,000 tokens, on every conversation. That is the adoption tax, and it is the
        strongest argument against the whole idea.
      </P>
      <P>
        An MCP server inverts it. The model asks for the dozen tags a task needs —{" "}
        <strong>175 characters instead of 3,808</strong>, measured by the conformance test — and gets
        the rest on demand.
      </P>
      <Note tone="tip" title="The tools that matter are not the vocabulary ones">
        <p>
          A prompt can carry a vocabulary. A prompt cannot tell a model whether what it just wrote is{" "}
          <em>correct</em>. <C>guml_check</C> answers that from the same compiler that will build it,
          before anyone runs the code — and <C>guml_repair</C> fixes the mechanical part without
          spending a turn on it.
        </p>
      </Note>

      <H2 id="install">Setup</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo install guml-cli   # or: pip install guml`}
      />
      <P>Then point a client at it. The server speaks stdio, so there is nothing to host:</P>
      <CodeBlock
        lang="json"
        filename="claude_desktop_config.json"
        code={`{
  "mcpServers": {
    "guml": {
      "command": "guml",
      "args": ["mcp"]
    }
  }
}`}
      />
      <P>
        The same block works in Claude Code, Cursor and anything else speaking MCP. No API key, no
        network, no daemon — the model talks to a compiler on your machine.
      </P>

      <H2 id="tools">The tools</H2>
      <Table
        head={["tool", "for"]}
        rows={[
          ["guml_registry(tags?)", "The vocabulary, or a prompt-sized slice. Call this first"],
          ["guml_spec()", "The language rules. Once per session, ~3,000 tokens"],
          ["guml_check(source)", "Every problem in one pass, with codes, lines and suggestions"],
          ["guml_repair(source)", "Unwrap a fence, format, apply every unambiguous fix. No model call"],
          ["guml_compile(source, backend?)", "Emit React, HTML, Svelte, or any other backend"],
        ]}
      />
      <P>
        The descriptions the server advertises are written for a model rather than for a person
        browsing documentation: they say <em>when</em> to call each tool and what it costs, because
        that is what a caller with no knowledge of GUML needs in order to choose.
      </P>

      <H2 id="loop">What a session looks like</H2>
      <CodeBlock
        lang="text"
        filename="a model building a dashboard"
        code={`→ guml_registry(["card", "metric", "list", "btn"])
← card (Container) — A panel grouping related content.
  metric (Text) — A single prominent number…            (175 chars)

→ guml_check(source)
← DOES NOT COMPILE — 1 error(s). Fix these and check again.
  error [GUML0030] line 4: unknown tag \`crad\`
      replace with: card

→ guml_repair(source)
← Applied 1 fix(es).
  --- repaired document ---
  …
  --- remaining ---
  COMPILES. No problems.

→ guml_compile(source, backend: "html")
← --- Dashboard.html ---
  <!doctype html>…`}
      />
      <P>
        The repair step is the one worth noticing. It is free, it runs no model, and on real
        generations it resolves a share of what a first draft gets wrong — see{" "}
        <C>bench/gen/FINDINGS.md</C> for the measured numbers and their caveats.
      </P>

      <H2 id="design">Design notes</H2>
      <UL>
        <LI>
          <strong>A document that does not compile is not a protocol error.</strong> It is a
          successful call whose result says what is wrong, because the model has to read it and try
          again. Protocol errors are reserved for protocol problems — an unknown method, a malformed
          message — so a client can tell the two apart.
        </LI>
        <LI>
          <strong>The verdict comes first.</strong> Every check result opens with{" "}
          <C>COMPILES</C> or <C>DOES NOT COMPILE</C>. A model handed a list of warnings and left to
          infer whether the document is usable will usually infer wrong and rewrite something that was
          already correct.
        </LI>
        <LI>
          <strong>No SDK, and no new dependencies.</strong> MCP over stdio is newline-delimited
          JSON-RPC with five methods; the alternative was pulling an async runtime into a binary whose
          whole dependency tree is 79 lines. The conformance test speaks the wire format rather than
          calling internals, so a change that would break a real client breaks the test.
        </LI>
        <LI>
          <strong>Nothing but protocol messages reaches stdout.</strong> One stray print anywhere in
          the compiler corrupts the stream and the client reports a disconnection with no explanation.
          That is checked on every run.
        </LI>
      </UL>
      <P>
        Related: <A href="/docs/python">the Python package</A> exposes the same primitives as a
        library, and <A href="/docs/compiler/cli">the CLI</A> exposes them as commands. All three call
        one compiler.
      </P>
    </DocPage>
  );
}
