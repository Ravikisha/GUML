import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Pkg, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Configuration and plugins",
  description:
    "guml.json states a project's vocabulary and styling once, so the editor, CI and the formatter cannot disagree. Plugins extend both.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/config"
      meter={{ label: "file", value: "guml.json", tone: "iris" }}
      title="Configuration and plugins"
      lede="A project states its vocabulary and its styling once. Everything that compiles a document — the editor, the formatter, check, CI — reads the same file, because a document that is valid in one and invalid in another is the worst failure a closed vocabulary can have."
      toc={[
        { id: "why", title: "Why a config file" },
        { id: "shape", title: "The shape" },
        { id: "plugins", title: "Plugins" },
        { id: "writing", title: "Writing your own" },
        { id: "theme", title: "Choosing a theme" },
        { id: "pinning", title: "Pinning a version" },
        { id: "precedence", title: "Flags still win" },
        { id: "scope", title: "What it deliberately does not do" },
      ]}
    >
      <H2 id="why">Why a config file</H2>
      <P>
        Registry packages and themes were once reachable only through <C>--registry</C> and{" "}
        <C>--theme</C>. That is enough to prove the design works and not enough to use it: every{" "}
        <C>check</C>, every <C>build</C>, the language server, the formatter and CI each had to be told
        the same paths, and the moment one of them was not, that call compiled against a different
        vocabulary.
      </P>
      <P>
        The point of a <em>closed</em> vocabulary is that everyone agrees on what the words are. So the
        vocabulary belongs to the project, stated once.
      </P>

      <H2 id="shape">The shape</H2>
      <CodeBlock
        lang="json"
        filename="guml.json"
        code={`{
  "$schema": "https://guml.vercel.app/schema/guml.json",
  "plugins": ["@guml/shadcn"],
  "theme": "shadcn",
  "backend": "react",
  "level": "app"
}`}
      />
      <Table
        head={["key", "meaning"]}
        rows={[
          ["plugins", "Packages contributing vocabulary, styling, or both"],
          ["registries", "Explicit paths to registry files, for anything not laid out as a package"],
          ["theme", "A builtin name, a path, or a plugin that ships one. Omitted means tailwind"],
          ["backend", "Default target for guml build"],
          ["level", "core compiles markup only — no state, data, actions or js"],
        ]}
      />
      <P>
        Found by walking up from the document, so <C>guml check src/pages/home.guml</C> works from
        anywhere in the tree. Paths resolve against the config file, never the working directory —
        otherwise the same command would mean different things from different directories, which is the
        bug this file exists to remove.
      </P>
      <P>
        The <C>$schema</C> line is worth keeping: editors autocomplete and validate from it, and{" "}
        <A href="https://guml.vercel.app/schema/guml.json">the published schema</A> is checked against
        the compiler by a test, so it cannot offer a backend that does not resolve.
      </P>

      <H2 id="plugins">Plugins</H2>
      <P>
        A plugin is a package that contributes to the compiler. Each entry is a package name resolved
        through <C>node_modules</C>, or a directory:
      </P>
      <CodeBlock
        lang="json"
        filename="guml.json"
        code={`{
  "plugins": [
    "@guml/shadcn",        // resolved through node_modules
    "./design-system"      // a directory in your repo
  ]
}`}
      />
      <P>The compiler loads whichever of these it finds inside:</P>
      <Table
        head={["file", "contributes"]}
        rows={[
          ["guml.registry.json", "Tags, and the components they lower to"],
          ["guml.theme.json", "The class table — what those tags look like"],
        ]}
      />
      <Note tone="info" title="Why one entry does both">
        <p>
          A design system is normally vocabulary <em>and</em> styling. Naming them separately is two
          chances to install one and forget the other, and the failure mode of forgetting the theme is a
          page full of correct tags rendering completely unstyled — with no error, because every tag
          resolved.
        </p>
      </Note>
      <P>
        A plugin contributing neither file is reported rather than ignored. Silence there would leave
        you believing a vocabulary was loaded, and the failure would surface later as{" "}
        <C>unknown tag</C> pointing at your document instead of at your config.
      </P>

      <H2 id="writing">Writing your own</H2>
      <P>Two files in a directory. Nothing else, and no build step:</P>
      <CodeBlock
        lang="bash"
        filename="design-system/"
        code={`design-system/
  guml.registry.json    # your tags
  guml.theme.json       # your classes
  src/                  # the React components the tags point at`}
      />
      <CodeBlock
        lang="json"
        filename="design-system/guml.registry.json"
        code={`{
  "name": "@acme/design",
  "version": "1.0.0",
  "components": [
    {
      "name": "callout",
      "kind": "container",
      "level": "app",
      "doc": "An inset panel drawing attention to one thing.",
      "element": "Callout",
      "import": "@acme/design",
      "attrs": ["tone"],
      "a11y": { "requires_label": true }
    }
  ]
}`}
      />
      <P>
        <C>element</C> and <C>import</C> are what make it a package rather than a JSON file: the
        compiler emits <C>{"<Callout … />"}</C> and generates the import, so your component — not a DOM
        tag — is what the document lowers to. That is also where any glue lives between GUML&rsquo;s
        calling convention and your component&rsquo;s own API; see{" "}
        <A href="/docs/language/registry">the registry</A> and{" "}
        <Pkg name="@guml/shadcn" />, which does exactly this for 26 tags.
      </P>
      <P>
        The theme file is an ordinary <A href="/docs/compiler/themes">theme document</A>. It must
        declare a focus treatment and a contrast floor, or the compiler refuses it — a themeable
        compiler that let a theme delete focus rings could not keep its accessibility promise.
      </P>
      <UL>
        <LI>
          A plugin may not shadow a builtin tag (<C>GUML0092</C>), so it cannot quietly redefine{" "}
          <C>card</C> underneath you.
        </LI>
        <LI>
          Every tag it declares must lower somewhere. A tag the prompt offers and the compiler cannot
          emit is worse than no tag: the model is told it exists, uses it, and gets a warning.
        </LI>
        <LI>
          Validate before shipping: <C>guml registry --validate ./design-system</C>.
        </LI>
      </UL>

      <H2 id="theme">Choosing a theme</H2>
      <P>Three forms, resolved in this order:</P>
      <Table
        head={["written as", "means"]}
        rows={[
          ['"tailwind" / "shadcn"', "A theme compiled into the binary. Needs nothing installed"],
          ['"./brand.theme.json"', "A theme document of your own"],
          ['"@acme/design"', "A plugin that ships guml.theme.json"],
        ]}
      />
      <P>
        A builtin name wins, deliberately: <C>shadcn</C> is both a shipped theme and a package name, and
        someone typing it means the theme. With no <C>theme</C> at all, a plugin&rsquo;s own theme
        applies if exactly one ships one.
      </P>
      <Note tone="warn" title="Two plugins with themes is an error, not a race">
        <p>
          If two shipped a theme, picking by list position would make the design of every page depend on
          something you never intended to express — and change silently when the list was reordered. It
          is reported, and naming one resolves it.
        </p>
      </Note>

      <H2 id="pinning">Pinning a version</H2>
      <CodeBlock
        lang="json"
        filename="guml.json"
        code={`{
  "registries": [
    { "path": "./vendor/widgets", "version": "0.1.0" }
  ]
}`}
      />
      <P>
        Loading <strong>fails</strong> if the package declares a different version. A registry decides
        which tags a document may use and which classes the compiler emits, so a package that changes
        underneath a project changes what its documents <em>mean</em>.
      </P>
      <P>
        Adding a tag is not even purely additive: a <C>def</C> may not shadow one, so a document that
        defined its own <C>stat</C> stopped compiling the release <C>stat</C> became builtin. That
        happened three times in this repository when the vocabulary grew from 28 entries to 49.
      </P>
      <P>
        Exact equality rather than a range, because a range needs a resolver, a lockfile, and a policy
        for what &ldquo;compatible&rdquo; means for a vocabulary — and semver&rsquo;s answer
        (&ldquo;additive is minor&rdquo;) is the one this project has evidence against.
      </P>

      <H2 id="precedence">Flags still win</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml build page.guml --theme brand.json --registry ./extra.json`}
      />
      <P>
        A one-off override is a real need, and CI should be able to pin explicitly rather than inherit.
        Order is: builtins, then each plugin, then <C>registries</C>, then <C>--registry</C> last.{" "}
        <C>--core</C> composes with all of it — a core host may load extra <em>markup</em> components,
        and any app-level entry is skipped rather than merged, so no package can smuggle behaviour past
        a host that asked for markup only.
      </P>

      <H2 id="scope">What it deliberately does not do</H2>
      <Note tone="warn" title="No network">
        <p>
          <C>guml add</C> takes a path. A registry decides what tags a document may use and what classes
          the compiler emits, so fetching one from a URL at build time would make the compiler&rsquo;s
          output depend on a remote server — a supply-chain surface for a project whose pitch is
          reliability.
        </p>
        <p className="mt-3">
          Packages arrive the way any dependency does: a file, a vendored directory,{" "}
          <C>node_modules</C>. Installed by an explicit command, never resolved implicitly.
        </p>
      </Note>
    </DocPage>
  );
}
