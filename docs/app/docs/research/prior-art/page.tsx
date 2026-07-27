import type { Metadata } from "next";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, H3, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Prior art",
  description: "What already exists, what GUML cannot claim, and the narrow gap that is actually open.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/research/prior-art"
      meter={{ label: "novelty", value: "narrower than it looks", tone: "ember" }}
      title="Prior art"
      lede="The idea of an LLM emitting declarative UI against a closed component catalog is not new — it is standardised and shipping. Here is what GUML cannot claim, and what remains."
      toc={[
        { id: "taken", title: "Already taken" },
        { id: "protocols", title: "The agent-UI protocols" },
        { id: "vega", title: "The strongest analogy" },
        { id: "gap", title: "What is actually open" },
        { id: "reading", title: "Reading list" },
      ]}
    >
      <H2 id="taken">Already taken</H2>
      <P>
        Claiming any of these would get a paper rejected in the first round, so none of them appear in
        GUML&rsquo;s contribution list.
      </P>
      <Table
        head={["claim", "who owns it"]}
        rows={[
          ["Declarative UI emitted by an LLM", "A2UI, MCP-UI, AG-UI; Jelly (CHI 2025); SpecifyUI"],
          ["UI-as-data with host-side renderers", "Server-driven UI at Airbnb, Lyft, Netflix, since 2021"],
          ["Intermediate representations improve LLM app generation", "Athena (IUI 2026), with a user study"],
          ["A compact declarative DSL as an LLM target", "Vega-Lite, with LIDA / VegaChat / Raiven"],
          ["Teaching an unseen DSL in context via its grammar", "Grammar Prompting (NeurIPS 2023)"],
          ["Guaranteeing syntactic validity of generated DSL", "SynCode, Domino, CRANE, xgrammar"],
          ["Compiler-feedback repair loops for generated UI", "DeclarUI"],
          ["Compiling one language to several frontend frameworks", "Svelte, Marko, Mitosis, Stencil"],
          ["A validated closed tag vocabulary", "Markdoc"],
          ["Component-library-constrained generation", "v0 with shadcn/ui"],
          ["Token-efficient serialisation", "TOON — with a mixed empirical record"],
        ]}
      />

      <H2 id="protocols">The agent-UI protocols</H2>
      <P>
        This is the most important section, and the easiest to miss because it is all recent.{" "}
        <strong className="text-chalk">A2UI</strong> was open-sourced by Google in December 2025 with
        AG-UI/CopilotKit, Opal, Gemini Enterprise and Flutter as collaborators. Its stated design
        principles include &ldquo;LLM-friendly and incrementally updateable&rdquo;, and its payload is a
        flat list of components with ID references, restricted to a client-held catalog of pre-approved
        components.
      </P>
      <P>
        <strong className="text-chalk">MCP-UI</strong> extends MCP&rsquo;s embedded-resource spec so
        servers can return UI, rendered in sandboxed iframes or via remote DOM.{" "}
        <strong className="text-chalk">AG-UI</strong> handles bidirectional agent-to-frontend events.
        The concept GUML was originally framed around is, in other words, already a standard.
      </P>
      <H3>What those four share, and GUML does not</H3>
      <UL>
        <LI>
          <strong className="text-chalk">They are JSON.</strong> Measured on the same app, GUML is 44%
          smaller than the minified JSON equivalent. They claim LLM-friendliness without publishing a
          token figure.
        </LI>
        <LI>
          <strong className="text-chalk">They render ephemeral UI inside a host.</strong> None compiles
          to a deployable application you own.
        </LI>
        <LI>
          <strong className="text-chalk">They have no application-logic layer.</strong> No client state
          machine, no derived state, no optimistic mutation semantics. A2UI is deliberately
          non-executable — correct for untrusted remote agents, and insufficient for &ldquo;build me an
          app&rdquo;.
        </LI>
        <LI>
          <strong className="text-chalk">There is no published tokens-versus-accuracy evaluation</strong>{" "}
          for any of them against a code baseline.
        </LI>
      </UL>
      <Note tone="info" title="Which is why they are compile targets">
        <p>
          A2UI and MCP-UI emitters are on the <A href="/docs/research/roadmap">roadmap</A>. Being the
          token-efficient surface syntax and compiler <em>for</em> the agent-UI ecosystem is a better
          position than competing with a multi-vendor standard, and it removes the single largest
          strategic risk to the project.
        </p>
      </Note>

      <H2 id="vega">The strongest analogy</H2>
      <P>
        Vega-Lite is the existence proof. A compact declarative grammar, targeted by LLMs instead of
        plotting code, in a narrow conventional domain — and the literature reports fewer invalid
        outputs than free-form code generation. Raiven even names the mechanism: DSL mediation.
      </P>
      <P>
        GUML is the same move applied to interactive applications rather than charts. The obvious
        counter is equally important: charts are narrow and conventional, and applications are neither.
      </P>

      <H2 id="gap">What is actually open</H2>
      <P>Three things, ranked by how defensible they are:</P>
      <UL>
        <LI>
          <strong className="text-chalk">The measurement.</strong> Token efficiency of a UI IR has never
          been measured against a code baseline. The agent-UI protocols publish no figures; the
          screenshot-to-code benchmarks measure quality and never tokens.
        </LI>
        <LI>
          <strong className="text-chalk">The DSL crossover.</strong> Anka shows a constrained DSL beating
          Python by 40 points on multi-step tasks it had never seen; the low-resource literature shows
          DSLs degrading accuracy. Both are well supported, and nobody has characterised where the
          boundary lies. That is the paper.
        </LI>
        <LI>
          <strong className="text-chalk">Convention as correctness.</strong> Formalising and measuring
          the claim that compiler-supplied conventions improve accessibility and error-state coverage{" "}
          <em>while</em> cutting tokens 8×. &ldquo;Fewer tokens and better a11y&rdquo; is a far stronger
          result than &ldquo;fewer tokens&rdquo;.
        </LI>
      </UL>
      <P>
        Two smaller findings are already in hand: the{" "}
        <A href="/docs/research/measurements">content floor</A>, and edit-locality as an unexplored
        metric for representation choice.
      </P>

      <H2 id="reading">Reading list</H2>
      <UL>
        <LI>
          <A href="https://arxiv.org/abs/2508.20263">
            Athena: Intermediate Representations for Iterative Scaffolded App Generation
          </A>{" "}
          — closest prior art. IUI 2026.
        </LI>
        <LI>
          <A href="https://developers.googleblog.com/introducing-a2ui-an-open-project-for-agent-driven-interfaces/">
            Introducing A2UI
          </A>{" "}
          — Google, December 2025.
        </LI>
        <LI>
          <A href="https://arxiv.org/abs/2512.23214">Anka: A DSL for Reliable LLM Code Generation</A> —
          the +40-point result.
        </LI>
        <LI>
          <A href="https://arxiv.org/abs/2410.03981">
            LLM-based code generation for low-resource and domain-specific languages
          </A>{" "}
          — the counter-evidence.
        </LI>
        <LI>
          <A href="https://arxiv.org/html/2606.09410">Capacity, Not Format</A> — why model scale has to
          be a variable.
        </LI>
        <LI>
          <A href="https://arxiv.org/abs/2305.19234">Grammar Prompting for DSL Generation</A> — NeurIPS
          2023; the required baseline.
        </LI>
        <LI>
          <A href="https://arxiv.org/abs/2409.11667">DeclarUI</A> — compiler-feedback repair, 98%
          compilation success.
        </LI>
        <LI>
          <A href="https://arxiv.org/abs/2403.03163">Design2Code</A> — 484 curated real-world pages; the
          benchmark protocol to reuse.
        </LI>
        <LI>
          <A href="https://arxiv.org/abs/2603.03306">TOON versus JSON</A> — how a token-efficiency claim
          fails when the prompt tax goes unreported.
        </LI>
      </UL>
      <P>
        The full survey, with methodology and a reviewer-style critique, is{" "}
        <C>GUML-Research-Report.md</C> in the repository.
      </P>
    </DocPage>
  );
}
