import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Architecture",
  description: "The compiler pipeline, the crate graph, and the invariants that hold it together.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/architecture"
      meter={{ label: "workspace", value: "9 crates · 67 tests" }}
      title="Architecture"
      lede="A Rust workspace with one entry point, because the thing calling it is usually a repair loop that wants a single structured answer."
      toc={[
        { id: "pipeline", title: "The pipeline" },
        { id: "crates", title: "Crate graph" },
        { id: "lexer", title: "Why the lexer is line-oriented" },
        { id: "recovery", title: "Error recovery" },
        { id: "loop", title: "The LLM loop" },
        { id: "budget", title: "Performance budget" },
      ]}
    >
      <H2 id="pipeline">The pipeline</H2>
      <CodeBlock
        lang="text"
        code={`GUML source
   ↓  Lexer          indentation-sensitive, line-oriented, error-recovering
   ↓  Parser         recursive descent, registry-aware
   ↓  AST            typed, span-annotated, serialisable
   ↓  Resolver       bindings → state / resource / item fields   ✓
   ↓  Semantic       accessible names, undeclared references     ✓
   ↓                 type check, exhaustiveness                  (Phase 3)
   ↓  Desugar        conventions: states, rollback, effects, ARIA (Phase 3)
   ↓  Optimizer      dead state, binding CSE, registry tree-shake (Phase 3)
   ↓  Codegen        pluggable backend
   ↓  Emit           React + TS · Svelte · Web Components · static HTML`}
      />
      <P>
        Everything unmarked is implemented and tested, including the resolver-lite and accessibility
        passes that emit `GUML0033`, `GUML0050` and `GUML0051`. The desugar pass marked Phase 3 is
        what turns a parsed resource into fetch code with rollback — until it lands, those constructs
        parse and then report themselves as unsupported in the React backend. The browser runtime
        implements them directly instead, which is why the playground can render a task list.
      </P>

      <H2 id="crates">Crate graph</H2>
      <Table
        head={["crate", "owns", "depends on"]}
        rows={[
          ["guml-diagnostics", "Span, Code, Diagnostic, JSON + human rendering", "—"],
          ["guml-syntax", "the indentation-sensitive line lexer", "diagnostics"],
          ["guml-ast", "typed span-annotated AST", "diagnostics"],
          ["guml-registry", "closed tag vocabulary, modifiers, typo suggestions", "—"],
          ["guml-parser", "recursive descent, registry-aware, error-recovering", "syntax, ast, registry"],
          ["guml-codegen", "Backend trait, React backend, design-system table", "ast, registry"],
          ["guml-compiler", "the driver: one structured result", "all of the above"],
          ["guml-cli", "the guml binary", "compiler, syntax, registry"],
          ["guml-wasm", "WebAssembly bindings for browsers and Node", "compiler, codegen, registry"],
        ]}
      />
      <Note tone="info" title="One deliberate non-dependency">
        <p>
          <C>guml-codegen</C> must not depend on <C>guml-parser</C> — that would be a cycle through
          the driver. Codegen unit tests build ASTs by hand; end-to-end tests from source text live in{" "}
          <C>crates/guml-compiler/tests/</C>.
        </p>
      </Note>

      <H2 id="lexer">Why the lexer is line-oriented</H2>
      <P>
        GUML contains an ambiguity no lexer can settle alone. In <C>btn Decrement ghost</C> the
        remainder is a label and a modifier; in <C>p Press the button.</C> the remainder is prose.
        Which one applies depends on the tag&rsquo;s kind, and kinds live in the registry — a
        resolution concern.
      </P>
      <P>
        So the lexer emits, per line, both a structured token list <em>and</em> the raw text, and lets
        the parser choose. <C>Line::rest_from</C> is the bridge. That design is also why prose is
        free: text is never escaped, because the lexer never has to interpret it.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`cargo run -q -p guml-cli -- lex fixtures/a.guml

  1 indent=0  [Word("page"), Word("Counter")]
  2 indent=0  [Word("state"), Word("count"), Eq, Num("0")]
  4 indent=0  [Word("card"), Word("sm"), Word("center")]
  5 indent=2  [Word("h"), Word("Clicks")]`}
      />

      <H2 id="recovery">Error recovery</H2>
      <P>
        The parser never returns early on an error. It collects every problem in one pass and keeps
        building the tree around the damage.
      </P>
      <P>
        This is not politeness. The producer of GUML is usually a model, so each round trip costs a
        full generation: a parser that reports one error at a time converts a one-round repair loop
        into an N-round one. Recovery is a tested invariant, not a nicety.
      </P>

      <H2 id="loop">The LLM loop</H2>
      <CodeBlock
        lang="text"
        code={`prompt + spec (cached) + registry slice (retrieved)
        ↓
   model emits GUML  ←──────────────┐
        ↓                            │  JSON diagnostics:
   grammar-constrained decode        │  span + message + suggestion
        ↓                            │
   parse → semantic → compile ───────┘  bounded at 3 rounds
        ↓
   emitted app → headless render → visual / a11y / Lighthouse checks`}
      />
      <P>Three published mechanisms, composed, each doing one job:</P>
      <UL>
        <LI>
          <strong className="text-chalk">Grammar prompting</strong> teaches an unseen DSL in context.
        </LI>
        <LI>
          <strong className="text-chalk">Grammar-constrained decoding</strong> guarantees syntactic
          validity — for local and open models. Hosted APIs expose no client-side CFG masking, so
          those arms use structured output plus the repair loop instead. That limitation is real and
          gets stated rather than glossed.
        </LI>
        <LI>
          <strong className="text-chalk">Compiler-feedback repair</strong> handles semantic errors,
          and applies unambiguous suggestions without another model call.
        </LI>
      </UL>
      <P>
        One further finding shapes the design: constrain the <em>emission</em>, not the reasoning.
        Let the model think in free text, then emit constrained GUML.
      </P>

      <H2 id="budget">Performance budget</H2>
      <Table
        head={["operation", "budget", "why"]}
        rows={[
          ["guml check, 200 lines", "< 2 ms", "runs on every repair-loop iteration and LSP keystroke"],
          ["guml build, 200 lines", "< 10 ms", "interactive builds inside a sandbox"],
          ["registry prompt slice", "< 1 ms", "runs per generation request"],
        ]}
      />
      <P>
        Regressions are a CI failure, not a follow-up ticket. It is also why the compiler is Rust and
        why the dependency list is four crates long — see{" "}
        <A href="/docs/compiler/backends">backends</A> for the WASM story that falls out of the same
        choice.
      </P>
    </DocPage>
  );
}
