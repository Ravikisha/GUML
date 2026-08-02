import type { Metadata } from "next";
import { DocPage } from "@/components/doc-page";
import { NumberTicker } from "@/components/motion-bits";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";
import { FIXTURES } from "@/lib/fixtures.generated";
import { commas, reduction } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Measurements",
  description: "The token numbers behind GUML, and every caveat that travels with them.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/research/measurements"
      meter={{ label: "tokenizer", value: "cl100k_base · authored fixtures" }}
      title="Measurements"
      lede="Three fixtures, written twice, counted with a real tokenizer. Here is what that shows, and — more importantly — what it does not."
      toc={[
        { id: "headline", title: "The headline" },
        { id: "vs-json", title: "Against a JSON IR" },
        { id: "content-floor", title: "The content floor" },
        { id: "amortisation", title: "The prompt tax" },
        { id: "latency", title: "Latency" },
        { id: "caveats", title: "What this does not show" },
        { id: "protocol", title: "Measurement protocol" },
      ]}
    >
      <H2 id="headline">The headline</H2>
      <div className="mt-7 overflow-x-auto rounded-card border border-line">
        <table className="w-full min-w-[32rem] text-left text-sm">
          <thead>
            <tr className="border-b border-line bg-chalk/[0.02]">
              {["fixture", "React + TS + Tailwind", "GUML", "reduction", "ratio"].map((h) => (
                <th key={h} className="label px-4 py-2.5 font-normal">
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="font-mono">
            {FIXTURES.map((f) => (
              <tr key={f.id} className="border-b border-line last:border-0">
                <td className="px-4 py-3 text-chalk">{f.title}</td>
                <td className="px-4 py-3 tabular-nums text-ember">{commas(f.tokens.react)}</td>
                <td className="px-4 py-3 tabular-nums text-iris">{commas(f.tokens.guml)}</td>
                <td className="px-4 py-3 tabular-nums text-mint">
                  <NumberTicker value={reduction(f.tokens.react, f.tokens.guml)} />%
                </td>
                <td className="px-4 py-3 tabular-nums text-fog">
                  {(f.tokens.react / f.tokens.guml).toFixed(1)}×
                </td>
              </tr>
            ))}
            <tr className="bg-chalk/[0.02]">
              <td className="px-4 py-3 font-mono text-chalk">total</td>
              <td className="px-4 py-3 font-mono tabular-nums text-ember">3,457</td>
              <td className="px-4 py-3 font-mono tabular-nums text-iris">613</td>
              <td className="px-4 py-3 font-mono tabular-nums text-mint">82%</td>
              <td className="px-4 py-3 font-mono tabular-nums text-fog">5.6×</td>
            </tr>
          </tbody>
        </table>
      </div>
      <P>
        The same figures hold within a percentage point under <C>o200k_base</C>, so this is not a
        tokenizer artifact.
      </P>

      <H2 id="vs-json">Against a JSON IR</H2>
      <P>
        &ldquo;Just emit JSON&rdquo; is the first objection anyone raises, so it is measured too. The
        task fixture was re-encoded as a declarative UI spec in the style of A2UI and
        server-driven UI, then minified.
      </P>
      <Table
        head={["representation", "tokens", "vs GUML"]}
        rows={[
          ["GUML", "178", "—"],
          ["JSON UI IR, minified", "315", "+80%"],
          ["JSON UI IR, pretty-printed", "533", "+205%"],
          ["React + TS + Tailwind", "1,441", "+709%"],
        ]}
      />
      <P>
        Quotes, braces and repeated keys are not free. GUML is 45% smaller than the minified JSON for
        identical semantics — which matters because the agent-UI protocols shipping today are all
        JSON and all claim to be LLM-friendly without publishing a token figure.
      </P>

      <H2 id="content-floor">The content floor</H2>
      <Note tone="warn" title="This is the finding that constrains everything else">
        <p>
          Of the landing page&rsquo;s 376 GUML tokens, <strong className="text-chalk">232 are
          irreducible prose</strong> — headlines, feature copy, FAQ answers. Structural overhead is
          144 tokens against React&rsquo;s ~1,416.
        </p>
      </Note>
      <P>Compression is therefore bounded by content, not by language design:</P>
      <UL>
        <LI>
          <strong className="text-chalk">Structure-heavy</strong> artifacts (CRUD, dashboards, forms)
          approach 8×.
        </LI>
        <LI>
          <strong className="text-chalk">Content-heavy</strong> artifacts (landing pages, docs) settle
          near 2–3× as the copy dominates.
        </LI>
      </UL>
      <P>
        Any single average across those two categories is misleading, so this site never reports one
        and the benchmark is specified to break results out per category.
      </P>

      <H2 id="amortisation">The prompt tax</H2>
      <P>
        The language spec has to be in the model&rsquo;s context, and that is a real cost. Assume a
        generous 3,000 tokens of spec plus registry slice plus examples. Under prompt caching it reads
        at roughly a tenth of the input rate.
      </P>
      <Table
        head={["item", "tokens", "cost at Opus-tier rates"]}
        rows={[
          ["spec + registry + examples (cached input)", "~3,000", "≈ $0.0015 per request"],
          ["React generation (output)", "1,441", "$0.0360"],
          ["GUML generation (output)", "178", "$0.0045"],
          ["saving per generation", "—", "≈ $0.0315"],
        ]}
      />
      <P>
        The spec pays for itself on the first request, at roughly twenty to one. Output tokens cost
        about five times input on current frontier models, which is why the saving concentrates exactly
        where the redundancy was.
      </P>

      <H2 id="latency">Latency</H2>
      <P>
        This is the strongest practical argument, and it is not about money. Output tokens decode
        sequentially, so an 8× reduction in output is roughly an 8× reduction in generation time: at
        60 tokens per second the task fixture goes from about 24 seconds to about 3.
      </P>

      <H2 id="caveats">What this does not show</H2>
      <UL>
        <LI>
          <strong className="text-chalk">These are authored artifacts, not model generations.</strong>{" "}
          They show what the representation costs, not what a model can produce.
        </LI>
        <LI>
          <strong className="text-chalk">Both sides were written by the same person.</strong> A
          favourable bias in the GUML encoding cannot be ruled out.
        </LI>
        <LI>
          <strong className="text-chalk">Correctness is unmeasured.</strong> 82% fewer tokens with
          worse functional correctness would be a worse system. Whether correctness improves, holds, or
          degrades has not been tested. This is the single largest caveat on this page, and no figure
          below is evidence about it.
        </LI>
        <LI>
          <strong className="text-chalk">Editing is unmeasured.</strong> Agents patch files rather than
          regenerating them, so the honest comparison for iteration is against diff-based React
          editing — not against full regeneration.
        </LI>
        <LI>
          <strong className="text-chalk">The escape-hatch rate is unknown.</strong> Every{" "}
          <C>raw</C> or <C>js</C> block spends compression back. A benchmark of only expressible tasks
          would inflate every number above.
        </LI>
      </UL>

      <H2 id="protocol">Measurement protocol</H2>
      <P>Rules this project holds itself to, and the reasons they exist:</P>
      <UL>
        <LI>
          Count with the target model&rsquo;s own tokenizer. <C>tiktoken</C> is an OpenAI tokenizer
          and undercounts Claude by roughly 15–20% on text and more on code, so its numbers never go
          in a paper or a README.
        </LI>
        <LI>Split input from output, and cached from uncached. A blended number is not a measurement.</LI>
        <LI>
          Report the prompt tax and the break-even artifact size below which raw React is simply
          cheaper. Omitting that is the documented failure mode that sank comparable claims for the
          TOON serialisation format.
        </LI>
        <LI>Report per category. Never one average.</LI>
        <LI>Report the escape-hatch rate.</LI>
        <LI>Disclose authorship bias in the same line as the result.</LI>
      </UL>
      <P>
        The nine-arm benchmark that turns these preliminary numbers into a result — React, HTML, JSON
        IR, TOON IR, v0, human expert, and three GUML configurations across three model tiers — is
        specified but has not been run. The numbers on this page are token counts over authored
        artifacts; they are not a result about model behaviour, and nothing here should be read as one.
      </P>
    </DocPage>
  );
}
