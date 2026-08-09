import type { Metadata } from "next";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Pkg, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Status and limitations",
  description:
    "What is stable, what may change in a 0.x release, and what GUML deliberately does not do.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/status"
      meter={{ label: "version", value: "0.2.0", tone: "iris" }}
      title="Status and limitations"
      lede="GUML is 0.x. This page says what that means in practice: which parts you can build on, which may change under you, and which things it deliberately will not do."
      toc={[
        { id: "semver", title: "What 0.x means here" },
        { id: "stable", title: "What is stable" },
        { id: "moving", title: "What may change" },
        { id: "scope", title: "Out of scope for v1" },
        { id: "limits", title: "Current limitations" },
        { id: "support", title: "Versions and support" },
      ]}
    >
      <H2 id="semver">What 0.x means here</H2>
      <P>
        Under semantic versioning, <C>0.x</C> carries no compatibility promise: a minor release may
        change the language surface, the emitted output or the crate APIs. That is an accurate
        description of where GUML is, and pretending otherwise would be the more expensive mistake.
      </P>
      <P>
        One thing is promised anyway, because too much depends on it:{" "}
        <strong>diagnostic codes are append-only</strong>. A <C>GUML0042</C> means the same thing in
        every future version. The repair loop keys on them, and renumbering one would break every
        consumer at once.
      </P>

      <H2 id="stable">What is stable</H2>
      <P>
        Covered by tests, exercised by the fixtures, and unlikely to change shape without a good reason.
      </P>
      <Table
        head={["Area", "State"]}
        rows={[
          ["Lexer, parser, error recovery", "every error collected in one pass, not just the first"],
          ["49-tag vocabulary, modifiers, directives", "closed and validated; unknown tags are diagnostics with suggestions"],
          [
            "Seven backends",
            "react, svelte, html, wc, json, a2ui, mcp-ui — from one shared element table, class table and expression lowering",
          ],
          ["50 diagnostic codes", "append-only, machine-readable via --format json"],
          ["Conventions", "loading, empty, error and optimistic states desugared at compile time"],
          ["Themes", "stock Tailwind by default; shadcn and your own by name; a theme below WCAG AA contrast or with no focus ring is refused"],
          ["Tooling", "CLI, language server, formatter, source maps, tree-sitter grammar, VS Code extension"],
          ["Escape hatches", "js and raw compile through unchanged, and are reported by guml capabilities"],
        ]}
      />

      <H2 id="moving">What may change in a 0.x release</H2>
      <UL>
        <LI>
          <strong>Emitted output.</strong> The generated React, Svelte and HTML is expected to improve.
          Do not diff it in your own tests and expect stability; compile and run it instead.
        </LI>
        <LI>
          <strong>Crate APIs.</strong> Everything except <C>guml-compiler</C>&apos;s entry points should
          be treated as internal, published so the driver can be.
        </LI>
        <LI>
          <strong>The tag vocabulary.</strong> Tags may be added. Removing or renaming one is possible
          in 0.x and will be in the changelog if it happens.
        </LI>
        <LI>
          <strong>Theme class strings.</strong> These track shadcn/ui. If you depend on specific
          utilities, pin a version.
        </LI>
      </UL>

      <H2 id="scope">Out of scope for v1</H2>
      <Note tone="warn" title="v1 is client-only, deliberately">
        <P>
          Server code, database schemas and authentication flows are <strong>not</strong> generated.
          This is not a gap waiting to be filled in a patch release — it is a scoping decision, and the
          reason is that attempting all four surfaces at once is the most likely way a project like this
          produces four mediocre ones.
        </P>
      </Note>
      <P>
        Consequently <C>route</C> and <C>auth</C> do not lower. Using them produces a warning and a{" "}
        <C>TODO</C> in the output rather than silently-wrong code — the compiler never emits something
        that looks like it works and does not.
      </P>

      <H2 id="limits">Current limitations</H2>
      <Table
        head={["Limitation", "What you will see", "What to do"]}
        rows={[
          [
            "route and auth do not lower",
            "A warning and a TODO comment in the emitted output",
            "Wire routing and auth in the host application around the emitted components",
          ],
          [
            "Some expressions exceed the action language",
            "GUML actions are not Turing-complete by design",
            "Use a js block. guml capabilities reports every document that does",
          ],
          [
            "Registry packages are React-only",
            "A host component reported rather than emitted in the no-JavaScript backend",
            "Expected: these are components with real behaviour, so static HTML cannot honestly render them",
          ],
          [
            "Registry packages are resolved by file path",
            "--registry node_modules/@guml/shadcn/guml.registry.json",
            "Works today. A config-file form is intended, and the path form will keep working",
          ],
          [
            "Charts, calendars and file upload are not builtins",
            "Unknown tag, with a suggestion",
            <span key="w">
              Install <Pkg name="@guml/widgets" /> — they need dependencies the compiler should not
              carry
            </span>,
          ],
        ]}
      />

      <H2 id="support">Versions and support</H2>
      <UL>
        <LI>
          <strong>Rust 1.85 or later</strong>, edition 2024. Raising this minimum is treated as a
          breaking change and appears in the changelog.
        </LI>
        <LI>
          <strong>Node 20 or later</strong> for the npm package.
        </LI>
        <LI>
          <strong>React 18 or later</strong> for the emitted React and the registry packages.
        </LI>
        <LI>
          Only the latest <C>0.2.x</C> release receives fixes, including security fixes. See the
          security policy in the repository for how to report one privately.
        </LI>
      </UL>
      <P>
        Changes are recorded in <C>CHANGELOG.md</C>. Claims about what a constrained IR does to model
        output — as opposed to what this compiler does — live in <A href="/research">research</A>, and
        are labelled there.
      </P>
    </DocPage>
  );
}
