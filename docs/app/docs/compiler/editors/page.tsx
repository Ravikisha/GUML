import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Editor support",
  description:
    "guml-lsp: diagnostics on keystroke, completion from the registry, hover that explains a rule, semantic tokens from the real lexer. Plus the VS Code extension.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/editors"
      meter={{ label: "check latency budget", value: "< 2 ms" }}
      title="Editor support"
      lede="A language server, and a VS Code extension that speaks to it. Everything an editor shows comes from the compiler itself — a second implementation of a language rule is a second answer waiting to disagree."
      toc={[
        { id: "why", title: "Why the compiler answers" },
        { id: "features", title: "What the server provides" },
        { id: "vscode", title: "VS Code" },
        { id: "other", title: "Other editors" },
        { id: "latency", title: "Latency" },
        { id: "missing", title: "Not implemented yet" },
      ]}
    >
      <H2 id="why">Why the compiler answers</H2>
      <P>
        Whether the remainder of a line is prose or structure depends on the tag, resolved against the
        component registry. No regex grammar can decide that, so an editor that guesses will colour{" "}
        <C>Decrement</C> as a modifier on one line and as prose on the next.
      </P>
      <P>
        Every answer the server gives — diagnostics, completions, hover text, token colours,
        formatting — is produced by the same crates the CLI uses. The vocabulary an editor offers is
        the vocabulary the compiler accepts, by construction rather than by maintenance.
      </P>

      <H2 id="features">What the server provides</H2>
      <Table
        head={["capability", "source of truth"]}
        rows={[
          [
            "diagnostics on change",
            <>
              the full <C key="a">check</C> pass, so an editor sees every error in one go rather than
              the first
            </>,
          ],
          ["completion", "the component registry: tags, then that tag's modifiers and attributes"],
          [
            "hover",
            <>
              the registry doc line for a tag, a resource&rsquo;s mutations for a{" "}
              <C key="b">data</C> name, and for a diagnostic, the same text as{" "}
              <C key="c">guml explain</C>
            </>,
          ],
          ["semantic tokens", <><C key="d">guml highlight</C> — the real lexer, delta-encoded</>],
          ["formatting", <><C key="e">guml fmt</C>, whole document</>],
          ["document symbols", "declarations and the element tree"],
          ["code actions", "the machine-applicable diagnostic suggestions"],
        ]}
      />
      <Note tone="warn" title="Positions are UTF-16 code units">
        <p>
          LSP counts positions in UTF-16 code units; the compiler&rsquo;s spans are byte offsets.
          Anything containing an em dash or an emoji desynchronises if that conversion is skipped, and
          the symptom is highlighting that drifts further right as the line goes on. The conversion is
          tested directly.
        </p>
      </Note>

      <H2 id="vscode">VS Code</H2>
      <P>
        The extension in <C>editors/vscode/</C> starts the server and ships a TextMate grammar for
        colour before the server attaches. That grammar is <em>generated</em> from{" "}
        <C>guml registry</C> — 27 tags, 8 prose tags, 24 modifiers — so it cannot drift from the
        compiler either.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`# build the server, then the extension
cargo build --release -p guml-lsp
cd editors/vscode && pnpm install && pnpm run build

# the extension finds the binary in target/release, then target/debug`}
      />
      <P>
        Point <C>guml.serverPath</C> at a binary explicitly if it lives somewhere else.
      </P>

      <H2 id="other">Other editors</H2>
      <P>
        <C>guml-lsp</C> speaks LSP over stdio, so any client can drive it. Two things make wiring one
        up straightforward:
      </P>
      <UL>
        <LI>
          <C>guml fmt</C> reads stdin and writes stdout, which is the shape most editor
          format-on-save hooks want, with no server needed.
        </LI>
        <LI>
          <C>guml check --format json</C> and <C>guml highlight</C> both emit machine-readable output,
          enough for diagnostics and colour in an editor with no LSP client at all.
        </LI>
      </UL>

      <H2 id="latency">Latency</H2>
      <P>
        The server calls <C>check</C> on every keystroke, so the compiler holds a budget:{" "}
        <C>check</C> under 2 ms and <C>build</C> under 10 ms on a 200-line document, measured by a
        criterion benchmark rather than asserted.
      </P>
      <Note>
        <p>
          That benchmark has already earned its place. A per-element allocation in the React backend
          pushed <C>check</C> from 1.77 ms to 2.47 ms — over budget — and every test still passed. Only
          the benchmark noticed.
        </p>
      </Note>

      <H2 id="missing">Not implemented yet</H2>
      <P>
        Named plainly, because an editor feature that half works is worse than one that is absent:
      </P>
      <UL>
        <LI>
          <strong>Rename</strong> — renaming a <C>state</C> or <C>data</C> declaration and its
          references.
        </LI>
        <LI>
          <strong>Go to definition</strong> — from a binding to the declaration it resolves against.
        </LI>
        <LI>
          <strong>Range formatting</strong> — the server formats whole documents only.
        </LI>
        <LI>
          <strong>A tree-sitter grammar</strong> — for editors that colour with tree-sitter rather
          than TextMate.
        </LI>
      </UL>
      <P>
        See <A href="/docs/status">status and limitations</A> for what is stable across a 0.x release.
      </P>
    </DocPage>
  );
}
