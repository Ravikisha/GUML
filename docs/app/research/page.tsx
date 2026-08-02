import type { Metadata } from "next";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Research",
  description:
    "What has been measured, what is only hypothesised, and what someone else found — kept separate from the documentation on purpose.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/research"
      meter={{ label: "status", value: "read the caveats", tone: "ember" }}
      title="Research"
      lede="GUML is an intermediate representation, a compiler, and an empirical study of whether the first two help a language model produce working interfaces. This section is the third part. It is deliberately not in the documentation."
      toc={[
        { id: "discipline", title: "Claim discipline" },
        { id: "pages", title: "What is here" },
        { id: "open", title: "What is not settled" },
      ]}
    >
      <H2 id="discipline">Claim discipline</H2>
      <P>
        Everything on these pages sits in exactly one of three tiers, and they are never blurred
        together. This is the rule the project holds itself to internally, so it may as well be the rule
        stated to you.
      </P>
      <Table
        head={["Tier", "What it means", "How to read it"]}
        rows={[
          [
            "Measured",
            "A number produced by running something, with the method named",
            "Trust it to the precision stated, and check which tokenizer produced it",
          ],
          [
            "Hypothesised",
            "A claim we believe and have not tested",
            "Treat as an open question. It is labelled because it is not evidence",
          ],
          [
            "Cited",
            "Someone else's result",
            "Their limitations travel with it, and are named where we use it",
          ],
        ]}
      />
      <Note tone="warn" title="Token counts are estimates unless stated otherwise">
        <P>
          <C>guml tokens</C> is a ~3.6 characters-per-token approximation, not a tokenizer. Where a
          figure comes from it, it says so. <C>tiktoken</C> figures do not appear here at all: it is an
          OpenAI tokenizer and it undercounts Claude, so a comparison built on it would flatter GUML for
          a reason that has nothing to do with GUML.
        </P>
      </Note>

      <H2 id="pages">What is here</H2>
      <UL>
        <LI>
          <A href="/research/measurements">Measurements</A> — the token numbers, the method behind each,
          and every caveat that travels with them.
        </LI>
        <LI>
          <A href="/research/prior-art">Prior art</A> — what already exists, what GUML cannot claim, and
          the narrower-than-it-looks gap that is actually open.
        </LI>
      </UL>

      <H2 id="open">What is not settled</H2>
      <P>
        The central question — whether a constrained IR measurably improves what a model produces,
        rather than merely costing fewer tokens to express — is open. It needs a controlled comparison
        with human grading that has not been run.
      </P>
      <P>
        Nothing in the documentation depends on the answer. The compiler either lowers a construct
        correctly or it does not, and that is settled by tests. This section is about the claims that
        tests cannot settle, which is exactly why it is kept away from the reference pages.
      </P>
    </DocPage>
  );
}
