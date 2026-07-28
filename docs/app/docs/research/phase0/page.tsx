import type { Metadata } from "next";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Step, Steps, Table, UL } from "@/components/prose";
import { Panel } from "@/components/ui";

export const metadata: Metadata = {
  title: "Phase 0 gate",
  description: "The two-week experiment that decides whether the rest of GUML gets built.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/research/phase0"
      meter={{ label: "status", value: "harness built · not yet run", tone: "ember" }}
      title="Phase 0 gate"
      lede="One question, two weeks, three pass criteria. If it fails, the project stops and the negative result gets published."
      toc={[
        { id: "question", title: "The question" },
        { id: "tension", title: "Why it is genuinely open" },
        { id: "setup", title: "Setup" },
        { id: "harness", title: "The harness" },
        { id: "gate", title: "The gate" },
        { id: "outcomes", title: "Reading the outcomes" },
        { id: "discipline", title: "Discipline" },
      ]}
    >
      <H2 id="question">The question</H2>
      <Panel className="mt-7 border-iris/25 bg-iris/[0.04] p-6">
        <p className="text-lg leading-relaxed text-chalk">
          Can a model produce valid, semantically correct GUML from a spec in context — and does the
          token saving survive contact with real generations?
        </p>
      </Panel>
      <P>
        Everything downstream assumes yes. Two weeks buys the answer; nine months of building does not
        make it better. No compiler in the loop, no constrained decoding, no repair — this measures the
        representation, not the toolchain.
      </P>

      <H2 id="tension">Why it is genuinely open</H2>
      <P>
        GUML has zero training data by construction. Two well-supported lines of evidence point in
        opposite directions:
      </P>
      <Table
        head={["evidence", "finding", "implication for GUML"]}
        rows={[
          [
            "Low-resource / DSL code generation survey, plus independent replications",
            "Models are measurably worse at languages under-represented in pretraining",
            "GUML should underperform React",
          ],
          [
            "Anka: a constrained DSL for data pipelines, unseen by the model",
            "+40 points over Python on multi-step tasks; 99.9% parse success",
            "A constrained DSL should beat a general-purpose language on exactly the failure mode UI generation is full of",
          ],
          [
            "Capacity, Not Format",
            "Format costs are absorbed by capable models and devastate weak ones",
            "Any result must be reported per model tier, not averaged",
          ],
        ]}
      />
      <P>
        Nobody has characterised where the crossover lies. That is the research contribution, and
        Phase 0 is its first data point.
      </P>

      <H2 id="setup">Setup</H2>
      <Steps>
        <Step n={1} title="Ten tasks">
          <P>
            Two structure-heavy (CRUD, dashboard), two content-heavy (landing, docs), six mixed
            (settings, checkout step, filter panel, team management, pricing, form wizard). Two reuse
            the existing fixtures so results connect to the{" "}
            <A href="/docs/research/measurements">measured numbers</A>. The counter fixture is not a
            task — it is an in-context example, and one document cannot be both the example and the
            answer.
          </P>
        </Step>
        <Step n={2} title="A reference implementation each">
          <P>
            Hand-written React + TS + Tailwind, plus a 12-to-14 item functional-requirements checklist
            that is the scoring instrument rather than documentation. All ten references typecheck
            under <C>--strict</C>.
          </P>
        </Step>
        <Step n={3} title="One prompt, two output rules">
          <P>
            The task text is identical in both arms and never mentions GUML or React; only the output
            rules differ. Asking the baseline for less is the easiest way to fake this result, so
            preflight fails if a task prompt names the language.
          </P>
        </Step>
        <Step n={4} title="Ninety runs">
          <P>
            Sixty GUML (ten tasks × three model tiers × two example counts) plus thirty React. The
            example-count variable does not apply to the baseline, which sees no spec and no examples.
          </P>
        </Step>
      </Steps>

      <H2 id="harness">The harness</H2>
      <P>
        It is built and tested. What it cannot do is call the models or grade the results, so the
        answer to Phase 0 is still unknown rather than pending.
      </P>
      <Table
        head={["stage", "state"]}
        rows={[
          ["Ten task specs with checklists", "done"],
          ["Ten React references, typechecked", "done"],
          ["Prompt assembly, stable prefix cached", "done"],
          ["Preflight: budget, leakage, registry slices", "done"],
          ["Mechanical scoring and gate check", "done"],
          ["Blind scoresheet and rubric", "done"],
          ["Scoring self-test on synthetic generations", "done"],
          ["Ninety generations", "needs an API key"],
          ["Blind correctness scoring", "needs a human grader"],
        ]}
      />
      <P>
        Everything that needs no API key runs from one command: <C>just phase0-verify</C>. That
        covers harness integrity, prompt assembly, and a self-test that scores synthetic generations
        of known shape — a scoring bug would otherwise produce a plausible, wrong answer to the only
        question this phase exists to ask.
      </P>
      <Note tone="tip" title="The prompt fits the budget it promised">
        <p>
          The largest assembled prompt — the landing task with three examples — is about 2,831
          estimated tokens against the 3,000 the spec commits to. Preflight fails if any prompt goes
          over, so the budget cannot quietly drift as the spec grows.
        </p>
      </Note>

      <H2 id="gate">The gate</H2>
      <P>Continue only if all three hold:</P>
      <div className="mt-7 space-y-3">
        {[
          "≥80% of the mid-tier model's generations at three examples are parseable GUML",
          "Median output-token reduction ≥3× against the paired React on structure-heavy tasks",
          "Semantic correctness is not worse than the React baseline on the same tasks",
        ].map((criterion, i) => (
          <div
            key={i}
            className="flex items-start gap-4 rounded-card border border-line bg-ink-raised/60 p-4"
          >
            <span className="mt-0.5 flex size-5 shrink-0 items-center justify-center rounded border border-line-strong font-mono text-[0.65rem] text-fog-dim">
              {i + 1}
            </span>
            <p className="text-sm leading-relaxed text-fog">{criterion}</p>
          </div>
        ))}
      </div>
      <Note tone="warn" title="Do not soften the gate after seeing results">
        <p>
          The hypotheses and the analysis plan get written to a file before anything runs, and are not
          edited afterwards. That rule is the only thing separating this from a post-hoc justification
          exercise — the same team wrote the language, the benchmark, and the compiler, so
          pre-registration is doing real work here.
        </p>
      </Note>

      <H2 id="outcomes">Reading the outcomes</H2>
      <Table
        head={["outcome", "reading", "action"]}
        rows={[
          ["All three pass", "The thesis survives first contact", "Proceed; this becomes the paper's preliminary section"],
          [
            "Tokens win, correctness loses",
            "The low-resource penalty dominates",
            "Try grammar prompting first; if it still loses, the honest framing is a cost optimisation, not a reliability one",
          ],
          [
            "Wins on the small model, vanishes on the frontier one",
            "Expected, per Capacity-Not-Format",
            "Still publishable — and commercially it points at cheap-model-plus-compiler as the product",
          ],
          [
            "Escape-hatch rate above 30%",
            "The vocabulary is too small for real work",
            "Fix the vocabulary before anything else",
          ],
          [
            "Nothing works",
            "The idea does not hold",
            "Publish the negative result — it answers an open question and cost two weeks",
          ],
        ]}
      />

      <H2 id="discipline">Discipline</H2>
      <UL>
        <LI>Measure the escape-hatch rate first, not last. It is the number most likely to sink the idea.</LI>
        <LI>
          Score correctness blind to which arm produced the artifact, by one person, against the same
          checklist.
        </LI>
        <LI>Keep every raw generation. Aggregates without raws are not reproducible.</LI>
        <LI>State model versions, the exact tokenizer, and example counts in the results table.</LI>
        <LI>
          Publish negative findings first in the write-up, not buried under the favourable ones.
        </LI>
      </UL>
      <P>
        The protocol lives at <C>spec/PHASE0.md</C>, the harness at <C>bench/phase0/</C> with its
        scoring rubric alongside it, and results will land at <C>spec/phase0-results.md</C>.
      </P>
    </DocPage>
  );
}
