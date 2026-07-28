import type { Metadata } from "next";
import { Check } from "lucide-react";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, Note, P } from "@/components/prose";
import { Badge } from "@/components/ui";
import { cn } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Roadmap",
  description: "Eight phases, each with a gate. What is done, what is next, and what stops the project.",
};

type Phase = {
  n: number;
  title: string;
  span: string;
  state: "open" | "partial" | "done";
  summary: string;
  items: Array<[string, boolean]>;
  gate: string[];
};

const PHASES: Phase[] = [
  {
    n: 0,
    title: "Kill-or-continue spike",
    span: "2 weeks",
    state: "partial",
    summary:
      "The highest-priority item. The harness is built and self-tested; the generations do not exist yet. No compiler in the loop — this measures whether a model can produce correct GUML at all.",
    items: [
      ["Freeze a v0.1 spec that fits in ≤3,000 context tokens", true],
      ["Ten hand-written task specs across the six benchmark categories", true],
      ["A paired React reference for each, all typechecking under --strict", true],
      ["Prompt assembly with the stable prefix cached", true],
      ["Mechanical scoring: parse validity, escape hatches, tokens, latency", true],
      ["Blind scoresheet and a written rubric", true],
      ["Self-test that scores synthetic generations of known shape", true],
      ["Run 10 × 3 model tiers × 2 example counts", false],
      ["Count with the target model's own tokenizer, not tiktoken", false],
      ["Score semantic correctness, blind, against each checklist", false],
      ["Record the escape-hatch rate", false],
      ["Write up results, negative findings first", false],
    ],
    gate: [
      "≥80% parseable GUML at three examples on the mid tier",
      "≥3× median output-token reduction on structure-heavy tasks",
      "Correctness not worse than the React baseline",
    ],
  },
  {
    n: 1,
    title: "Research and language design",
    span: "4 weeks",
    state: "partial",
    summary: "The survey and the framing are done; the formal grammar and the design log are not.",
    items: [
      ["Literature and landscape survey", true],
      ["Token measurement on three authored fixtures", true],
      ["Framing: IR + compiler study, not a new markdown", true],
      ["A2UI and MCP-UI treated as compile targets", true],
      ["EBNF that matches the parser exactly", false],
      ["Registry schema documented for external packages", false],
      ["Written objective function for the language", false],
      ["Log the rejected syntaxes — they are paper material", false],
    ],
    gate: ["Grammar + registry slice + three examples fit in ≤3,000 tokens across all six categories"],
  },
  {
    n: 2,
    title: "Front end",
    span: "4 weeks",
    state: "partial",
    summary: "Lexer, parser, AST and diagnostics are implemented and tested. The expression language is not.",
    items: [
      ["guml-diagnostics: spans, stable codes, JSON output", true],
      ["guml-syntax: indentation lexer with prose/structure split", true],
      ["guml-ast: typed, span-annotated, serialisable", true],
      ["guml-parser: registry-aware, collects every error in one pass", true],
      ["Directives: page, type, state/store, data + mutations", true],
      ["Elements: positionals, modifiers, attrs, actions, content", true],
      ["91 unit and integration tests green", true],
      ["A real expression parser instead of pass-through", false],
      ["raw / js escape-hatch blocks", false],
      ["route, auth, def directives", false],
      ["Fuzzing: no panics over 1M iterations", false],
    ],
    gate: [
      "100% parse of the fixture set",
      "Recovers from ≥90% of injected single-token mutations",
      "Zero panics under fuzzing",
    ],
  },
  {
    n: 3,
    title: "Compiler core",
    span: "8 weeks",
    state: "partial",
    summary:
      "The React backend and the desugar pass work — the conventions that make the token saving real are now generated rather than promised. Type checking, optimisation and source maps are not.",
    items: [
      ["Driver with one structured result", true],
      ["Backend trait", true],
      ["React backend: containers, text, controls, state, actions", true],
      ["Design-system table owned by the compiler", true],
      ["Unsupported constructs warn rather than mis-lower", true],
      ["JSON UI-tree backend behind the runtime and playground", true],
      ["Resolver-lite: bindings → state, resource, item fields", true],
      ["Accessibility lint as errors, severity graded by recoverability", true],
      ["Desugar: resources, loading/empty/error, optimistic rollback", true],
      ["Expression lowering, mirrored in the runtime with a parity test", true],
      ["Emitted TSX typechecks under tsc --strict", true],
      ["Semantic analyser: types, exhaustiveness", false],
      ["Retry with backoff and a response cache", false],
      ["Optimizer: dead state, binding CSE, registry tree-shake", false],
      ["Source maps GUML → TSX", false],
    ],
    gate: [
      "All three fixtures compile with zero warnings",
      "Emitted code passes a Playwright test per fixture",
      "Zero axe-core violations",
      "Twenty further fixtures compile and pass",
    ],
  },
  {
    n: 4,
    title: "Component registry",
    span: "6 weeks",
    state: "partial",
    summary: "The builtin vocabulary exists. Packages, themes and retrieval do not.",
    items: [
      ["Builtin registry with kinds, attrs, modifiers, suggestions", true],
      ["Prompt-sized registry slices from the CLI", true],
      ["Grow to ~40 primitives, shadcn-backed", false],
      ["Per-entry accessibility contracts", false],
      ["JSON registry packages", false],
      ["Theme packs mapping modifiers to design tokens", false],
      ["Per-entry token-cost metadata", false],
      ["Retrieval layer driven by the task description", false],
    ],
    gate: ["Registry covers ≥90% of element needs across the benchmark without escape hatches"],
  },
  {
    n: 5,
    title: "LLM integration",
    span: "6 weeks",
    state: "partial",
    summary: "Diagnostics are already machine-shaped. The loop around them is not built.",
    items: [
      ["JSON diagnostics designed for machine consumption", true],
      ["Cache-optimised prompt layout", false],
      ["Grammar prompting harness", false],
      ["Grammar-constrained decoding for local models", false],
      ["Repair loop bounded at three rounds", false],
      ["Auto-apply unambiguous suggestions with no model call", false],
      ["Telemetry: tokens in/out, cached, rounds, time-to-valid", false],
    ],
    gate: [
      "≥95% valid GUML within one repair round on the mid tier",
      "Prompt tax reported separately from generation tokens",
    ],
  },
  {
    n: 6,
    title: "GUML-Bench and evaluation",
    span: "10 weeks",
    state: "open",
    summary: "The result that decides whether any of this was worth it.",
    items: [
      ["150 tasks, six categories, with tests and reference screenshots", false],
      ["Nine arms including JSON IR, TOON IR, v0 and a human baseline", false],
      ["Three model tiers — capability is a first-class variable", false],
      ["Harness: tokens, cost, latency, Playwright, axe-core, Lighthouse", false],
      ["Edit-locality benchmark against diff-based React editing", false],
      ["Ablation grid: spec size × examples × decoding × repair × model", false],
      ["Human study, n≥30", false],
      ["Pre-register H1–H6 before running anything", false],
      ["Publish raw generations, not just aggregates", false],
    ],
    gate: ["A statistically significant result on at least three hypotheses — positive or negative"],
  },
  {
    n: 7,
    title: "Second backend and papers",
    span: "8 weeks",
    state: "open",
    summary: "Portability, tooling, and the write-ups.",
    items: [
      ["Svelte backend", false],
      ["Web Components backend", false],
      ["A2UI and MCP-UI emitters", false],
      ["Static HTML/CSS/JS backend", false],
      ["WASM build so the compiler runs in a browser", false],
      ["Language server reusing the same diagnostics", false],
      ["Paper 1: how should LLMs represent user interfaces?", false],
      ["Paper 2: convention as compression", false],
      ["Release the benchmark as a standalone dataset", false],
    ],
    gate: [],
  },
];

const STATE_LABEL = {
  open: { text: "not started", tone: "ember" },
  partial: { text: "in progress", tone: "iris" },
  done: { text: "done", tone: "mint" },
} as const;

export default function Page() {
  return (
    <DocPage
      pathname="/docs/research/roadmap"
      meter={{ label: "phases", value: "8 · each with a gate" }}
      title="Roadmap"
      lede="Eight phases, mapped one-to-one onto the research report. Each has a gate, and a gate is a hard stop rather than a suggestion."
      toc={PHASES.map((p) => ({ id: `phase-${p.n}`, title: `Phase ${p.n} — ${p.title}` }))}
    >
      <Note tone="warn" title="Phase 0 comes first for a reason">
        <p>
          Phases 1 through 5 already have work checked off because the compiler was built before the
          gate was run — useful for making the idea concrete and testable, but it does not change the
          ordering. Phase 0 still decides whether the remaining work happens. See{" "}
          <A href="/docs/research/phase0">the gate</A>.
        </p>
      </Note>

      {PHASES.map((phase) => {
        const done = phase.items.filter(([, d]) => d).length;
        const state = STATE_LABEL[phase.state];
        return (
          <section key={phase.n} id={`phase-${phase.n}`} className="mt-16 scroll-mt-28">
            <div className="flex flex-wrap items-center gap-3">
              <span className="font-mono text-sm text-fog-dim">Phase {phase.n}</span>
              <h2 className="display-narrow text-2xl font-bold tracking-tight text-chalk">
                {phase.title}
              </h2>
              <Badge tone={state.tone}>{state.text}</Badge>
              <span className="label ml-auto">
                {done}/{phase.items.length} · {phase.span}
              </span>
            </div>

            <P>{phase.summary}</P>

            <ul className="mt-5 space-y-1.5">
              {phase.items.map(([item, complete]) => (
                <li key={item} className="flex items-start gap-3 text-sm leading-relaxed">
                  <span
                    className={cn(
                      "mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-[3px] border",
                      complete ? "border-mint/40 bg-mint/10" : "border-line-strong",
                    )}
                  >
                    {complete ? <Check className="size-2.5 text-mint" /> : null}
                  </span>
                  <span className={complete ? "text-fog-dim line-through decoration-fog-dim/40" : "text-fog"}>
                    {item}
                  </span>
                </li>
              ))}
            </ul>

            {phase.gate.length > 0 && (
              <div className="mt-6 rounded-card border border-ember/20 bg-ember/[0.04] p-4">
                <p className="label mb-2 text-ember/70">gate</p>
                <ul className="space-y-1.5">
                  {phase.gate.map((g) => (
                    <li key={g} className="text-sm leading-relaxed text-fog">
                      {g}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </section>
        );
      })}

      <H2 id="cross-cutting">Always on</H2>
      <P>
        CI with <C>fmt</C>, <C>clippy -D warnings</C> and the test suite; <C>criterion</C> benches
        guarding compile latency, because the compiler sits in the repair-loop hot path; every claim in
        the README traceable to a test or a measurement; and the escape-hatch rate tracked
        continuously, since a rising number is the early warning that the expressiveness cliff is being
        hit.
      </P>
    </DocPage>
  );
}
