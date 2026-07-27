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
      meter={{ label: "status", value: "not yet run", tone: "ember" }}
      title="Phase 0 gate"
      lede="One question, two weeks, three pass criteria. If it fails, the project stops and the negative result gets published."
      toc={[
        { id: "question", title: "The question" },
        { id: "tension", title: "Why it is genuinely open" },
        { id: "setup", title: "Setup" },
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
            (settings, checkout step, filter panel, team management, pricing, form wizard). Three reuse
            the existing fixtures so results connect to the{" "}
            <A href="/docs/research/measurements">measured numbers</A>.
          </P>
        </Step>
        <Step n={2} title="A reference implementation each">
          <P>
            Hand-written React + TS + Tailwind, plus a functional-requirements checklist per task.
          </P>
        </Step>
        <Step n={3} title="A prompt harness">
          <P>
            Spec, a registry slice from <C>guml registry --tags …</C>, and N in-context examples.
          </P>
        </Step>
        <Step n={4} title="Thirty runs per representation">
          <P>Ten tasks × three model tiers × two example counts, for GUML and for the React baseline.</P>
        </Step>
      </Steps>

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
            className="flex items-start gap-4 rounded-card border border-white/8 bg-ink-raised/60 p-4"
          >
            <span className="mt-0.5 flex size-5 shrink-0 items-center justify-center rounded border border-white/15 font-mono text-[0.65rem] text-fog-dim">
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
        The protocol lives at <C>spec/PHASE0.md</C> in the repository; results will land at{" "}
        <C>spec/phase0-results.md</C>.
      </P>
    </DocPage>
  );
}
