import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, H3, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Install",
  description: "Build the GUML compiler from source with a stable Rust toolchain.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/install"
      meter={{ label: "requires", value: "rust 1.85+ · edition 2024" }}
      title="Install"
      lede="There is no published crate yet. Build from source — it takes one command and about a minute of cold compile."
      toc={[
        { id: "requirements", title: "Requirements" },
        { id: "build", title: "Build" },
        { id: "verify", title: "Verify" },
        { id: "install-the-binary", title: "Install the binary" },
        { id: "optional", title: "Optional tools" },
        { id: "editor", title: "Editor setup" },
      ]}
    >
      <H2 id="requirements">Requirements</H2>
      <Table
        head={["tool", "version", "why"]}
        rows={[
          ["Rust", "1.85 or newer (stable)", <>Edition 2024 and let-chains are both used</>],
          ["cargo", "ships with Rust", "Workspace build and test runner"],
          [
            "just",
            "optional",
            <>
              Convenience recipes only. Every one is a single <C>cargo</C> command.
            </>,
          ],
        ]}
      />
      <P>
        The repository pins the toolchain in <C>rust-toolchain.toml</C>, so <C>rustup</C> installs
        the right components — including the <C>wasm32-unknown-unknown</C> target — the first time
        you build.
      </P>

      <H2 id="build">Build</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`git clone https://github.com/guml-lang/guml
cd guml
cargo build --workspace`}
      />

      <H2 id="verify">Verify</H2>
      <P>
        The test suite is the fastest confirmation that everything resolved. Both bugs found while
        the compiler was first written were caught here, so a green run is meaningful rather than
        ceremonial.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo test --workspace
# 49 passed; 0 failed

cargo run -q -p guml-cli -- build fixtures/a.guml
# emits Counter.tsx to stdout`}
      />

      <H2 id="install-the-binary">Install the binary</H2>
      <P>
        To get a <C>guml</C> on your PATH instead of typing the cargo invocation each time:
      </P>
      <CodeBlock lang="bash" filename="terminal" code={`cargo install --path crates/guml-cli`} />
      <P>
        Every example in these docs uses the long form (<C>cargo run -q -p guml-cli -- …</C>) so it
        works in a fresh clone without installing anything.
      </P>

      <H2 id="optional">Optional tools</H2>
      <H3>just</H3>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo install just

just test          # cargo test --workspace
just demo          # build fixtures/a.guml
just diagnose      # JSON diagnostics for fixtures/b.guml
just registry      # print the component vocabulary`}
      />
      <H3>Lint and format</H3>
      <P>
        CI runs with warnings denied, so run both before pushing. Formatting settings live in{" "}
        <C>rustfmt.toml</C>.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings`}
      />

      <H2 id="editor">Editor setup</H2>
      <UL>
        <LI>
          <strong className="text-chalk">Rust:</strong> rust-analyzer, no configuration needed.
        </LI>
        <LI>
          <strong className="text-chalk">
            <C>.guml</C> files:
          </strong>{" "}
          no editor support yet. A tree-sitter grammar and a <C>tower-lsp</C> language server are
          Phase 7 on the <A href="/docs/research/roadmap">roadmap</A>; the language server will
          surface the same diagnostics the compiler gives a model.
        </LI>
        <LI>
          <strong className="text-chalk">Indentation:</strong> two spaces, spaces only. Tabs are a
          compile error (<C>GUML0001</C>), which the bundled <C>.editorconfig</C> keeps you clear of.
        </LI>
      </UL>

      <Note tone="tip" title="No Node required">
        <p>
          The compiler is pure Rust. Node is only needed for this documentation site and, later, for
          the benchmark harness that drives Playwright and axe-core.
        </p>
      </Note>
    </DocPage>
  );
}
