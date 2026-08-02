import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Capabilities and CSP",
  description:
    "What a document will actually do — origins, script, storage, escape-hatch rate — and a Content-Security-Policy derived from it.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/capabilities"
      meter={{ label: "posture", value: "derived, not declared", tone: "ember" }}
      title="Capabilities and CSP"
      lede="`core` versus `app` answers one question — may an untrusted agent send me this at all — and it is far too coarse to act on. A host needs to know which origins a document will contact, whether it contains script, and what its escape-hatch rate is, because those are the terms a Content-Security-Policy is written in. The compiler already knows all three exactly."
      toc={[
        { id: "why", title: "Why one bit is not enough" },
        { id: "manifest", title: "The manifest" },
        { id: "csp", title: "The generated CSP" },
        { id: "inert", title: "The safe-render gate" },
        { id: "budget", title: "The escape-hatch budget" },
        { id: "not", title: "What this does not do" },
      ]}
    >
      <H2 id="why">Why one bit is not enough</H2>
      <P>
        The <A href="/docs/language/levels">conformance level</A> is a compile-time restriction: a{" "}
        <C>core</C> registry has no app-level tags in it, so an untrusted document cannot use one. That is
        the right first question and it answers exactly one thing.
      </P>
      <P>
        It does not tell a host that this particular document fetches from{" "}
        <C>https://api.example.com</C> and nowhere else, that it contains no script at all, or that 11% of
        its lines are inside an escape hatch. Those are facts about the <em>artifact</em>, and they are
        what a host actually has to make a decision with. Reading them off a network log afterwards is not
        a security posture.
      </P>

      <H2 id="manifest">The manifest</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml capabilities fixtures
guml capabilities page.guml --format json`}
      />
      <CodeBlock
        lang="text"
        filename="output"
        code={`fixtures/b.guml — Tasks (app)
  network: self
    GET    /api/tasks (tasks)
    POST   /api/tasks (tasks.add)  mutating
    PATCH  /api/tasks/{id} (tasks.save)  mutating
    DELETE /api/tasks/{id} (tasks.drop)  mutating
  component \`tabs\` needs a runtime
  component \`list\` needs a runtime

fixtures/c.guml — Landing (core)
  inert: no script, no network, markup only`}
      />
      <P>
        Every field is derived from the AST. There is no manifest for an author to write and therefore none
        to get wrong, and nothing to keep in sync with the code — which is the same argument the{" "}
        <A href="/docs/language/registry">registry</A> makes about a component declaring its own contract.
      </P>
      <Table
        head={["field", "what it answers"]}
        rows={[
          ["network", "Every distinct origin the document will contact. `self` for a same-origin path."],
          ["requests", "Each request individually, with the resource or mutation that issues it, and whether it changes server state — decided by HTTP method, so a mutation called `fetch` is still counted."],
          ["script", "True when a `js` block is present: arbitrary code the compiler does not check."],
          ["rawMarkup", "True when a `raw` block is present: host markup the compiler does not escape."],
          ["storage", "Reported as false rather than omitted, so a consumer can tell “no” from “unknown”."],
          ["escapes", "Counts and the share of lines. A rising rate is the early warning that the vocabulary is hitting an expressiveness cliff."],
          ["components", "Registry components that declare a capability of their own — including a loaded third-party one, which a hardcoded list here could never cover."],
        ]}
      />

      <H2 id="csp">The generated CSP</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml capabilities fixtures/c.guml --csp html`}
      />
      <CodeBlock
        lang="text"
        filename="output"
        code={`default-src 'none'; connect-src 'none'; script-src 'none';
style-src 'unsafe-inline' /* the html backend inlines the theme; it has no build step */;
img-src 'self' data:; form-action 'none'; frame-ancestors 'none'; base-uri 'none'`}
      />
      <P>
        <strong>Generated, not documented.</strong> Prose telling a host &ldquo;add the origins your
        document uses to <C>connect-src</C>&rdquo; puts the compiler&rsquo;s own knowledge in a paragraph
        and asks a human to reproduce it. The compiler has the exact list, so <C>connect-src</C> is exactly
        that list — which means a request the document did not declare is blocked by the browser rather
        than merely unexpected.
      </P>
      <Note tone="info" title="It says `'none'` when it can, and says why when it cannot">
        <P>
          A document with no behaviour gets <C>script-src &apos;none&apos;</C>. That is worth far more than{" "}
          <C>&apos;self&apos;</C>: it is the difference between &ldquo;we did not need it&rdquo; and
          &ldquo;we did not restrict it&rdquo;.
        </P>
        <P>
          Where the compiler&rsquo;s <em>own</em> output needs a loosening, the policy carries the reason
          inline. The static-HTML backend inlines the theme stylesheet, because it has no build step by
          design and so cannot run a utility-class compiler — a real requirement, stated, rather than{" "}
          <C>unsafe-inline</C> added silently to every policy.
        </P>
      </Note>
      <P>
        The policy is a property of the <em>output</em>, not of the source, which is why it takes a backend
        argument. The same document compiled to React needs <C>style-src &apos;self&apos;</C>.
      </P>

      <H2 id="inert">The safe-render gate</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml capabilities untrusted.guml --assert-inert
# not safe to render untrusted — untrusted.guml: contains a \`js\` block; contacts self; needs a runtime`}
      />
      <P>
        One command, one answer, for the host embedding a document that arrived from an agent. It exits
        non-zero unless every document is <strong>inert</strong>: markup only, no script, no network. And it
        names <em>which</em> property failed — &ldquo;not inert&rdquo; sends a reader back to the manifest,
        and naming the reason is the difference between a gate and an obstacle.
      </P>
      <P>
        Worth having alongside <C>--core</C> rather than instead of it. The level is a compile-time
        restriction on the vocabulary; this is a fact about the artifact in front of you.
      </P>

      <H2 id="budget">The escape-hatch budget</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml capabilities fixtures --max-escapes 3`}
      />
      <P>
        <C>js</C> and <C>raw</C> are the two constructs where every guarantee stops. A rising escape-hatch
        rate is the early warning that the vocabulary is too small — and a warning nothing fails on is a
        statistic, so this is the number CI exits non-zero on.
      </P>
      <P>
        It is a <strong>ratchet</strong>. The repository runs at 3, which is <C>d.guml</C>&rsquo;s count,
        and <C>d.guml</C> exists in order to exercise <C>js</C> and <C>raw</C>. Lower it when the
        vocabulary grows enough to make a hatch unnecessary; never raise it without saying which construct
        could not be expressed.
      </P>

      <H2 id="not">What this does not do</H2>
      <UL>
        <LI>
          <strong>It does not sign anything.</strong> Signed registry and theme packages need a signing
          scheme and a key-distribution story, and picking either without input would be inventing policy
          rather than implementing it. The mitigation that needs no such decision is already in place:{" "}
          <A href="/docs/compiler/cli">
            <C>guml add</C>
          </A>{" "}
          takes a path and never a URL, so a registry cannot be fetched from a remote server at build time.
        </LI>
        <LI>
          <strong>It does not sandbox.</strong> The manifest tells a host what to permit; enforcing it is
          the host&rsquo;s CSP, iframe or worker. A compiler cannot enforce a policy on code it has already
          emitted.
        </LI>
        <LI>
          <strong>It does not audit a <C>js</C> body.</strong> The whole point of an escape hatch is that
          the compiler does not look inside. It reports that one is present, counts it, and refuses to call
          the document inert — which is the honest limit of what it can say.
        </LI>
      </UL>
    </DocPage>
  );
}
