import type { Metadata } from "next";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Privacy",
  description: "What this site collects, what it does not, and how to change your mind.",
};

export default function Page() {
  return (
    // The root layout owns the `main` landmark and the skip-link target; a second one here would
    // be a document with two, which is the fault this project checks for in its own emitted output.
    <div className="mx-auto max-w-(--container-page) px-6 py-12 md:px-10">
      <article className="min-w-0 max-w-3xl pb-16">
        <h1 className="display-wide text-heading-lg text-chalk">Privacy</h1>
        <p className="mt-6 max-w-[62ch] text-lg leading-relaxed text-fog">
          Short, because there is not much to say. This is a documentation site for a compiler; it has
          no accounts, no sign-in, and nothing to sell.
        </p>

        <div className="mt-12">
          <H2 id="analytics">Analytics</H2>
          <P>
            Page views are counted with Google Analytics 4. Consent Mode is set to{" "}
            <strong>denied by default</strong>, which means that until you press Accept, GA4 stores no
            cookie and no identifier on your device. It sends a cookieless ping so aggregate traffic
            can be modelled, and nothing that could identify you leaves the page.
          </P>
          <P>
            Accepting allows <C>analytics_storage</C>: an identifier that lets a returning reader be
            told apart from a new one. That is the only thing the banner asks for, and the only thing
            that changes.
          </P>
          <Note tone="info" title="Advertising signals are denied unconditionally">
            <p>
              <C>ad_storage</C>, <C>ad_user_data</C> and <C>ad_personalization</C> are set to denied and
              are never updated, whatever you choose. This site runs no advertising and has no reason to
              ever grant them, so they are stated explicitly rather than left to a default that could
              change.
            </p>
          </Note>
          <P>
            Your choice is kept in <C>localStorage</C> under <C>guml-consent</C>, on your own device.
            It is never transmitted — not to us, not to Google. Clearing your site data removes it and
            you will be asked again.
          </P>

          <H2 id="chat">The chat demo</H2>
          <P>
            <A href="/chat">/chat</A> sends what you type to a language model to generate GUML. What
            you send is processed by that provider under their terms. Do not paste anything
            confidential into a public demo — here or anywhere.
          </P>
          <P>
            To stop one visitor exhausting a shared quota, requests are rate-limited by network address
            and by a signed cookie holding a random identifier. The cookie carries no personal data and
            exists only to count.
          </P>

          <H2 id="playground">The playground</H2>
          <P>
            <A href="/playground">/playground</A> compiles GUML entirely in your browser — the compiler
            is WebAssembly running on your machine. What you type there is never sent anywhere.
          </P>

          <H2 id="summary">In one table</H2>
          <Table
            head={["what", "where it goes", "needs consent"]}
            rows={[
              ["Page views", "Google Analytics, cookieless until you accept", "for the identifier only"],
              ["Your consent choice", "localStorage on your device; never sent", "no — it is how we honour it"],
              ["Chat prompts", "the model provider, to answer them", "you send it by using it"],
              ["Rate-limit counter", "our server; an address and a random id", "no — it is how the demo stays up"],
              ["Playground input", "nowhere; it never leaves your browser", "no"],
            ]}
          />

          <H2 id="not">What there is none of</H2>
          <UL>
            <LI>No advertising, and no advertising identifiers.</LI>
            <LI>No third-party trackers, no social pixels, no session recording.</LI>
            <LI>No accounts, so nothing to log in to and nothing to leak.</LI>
            <LI>No fonts, scripts or images loaded from anywhere but this origin and Google Analytics.</LI>
          </UL>
          <P>
            That last one is enforced rather than promised: a Content-Security-Policy on every response
            blocks any request to a host not listed, so a third-party script could not load even if one
            were added by mistake.
          </P>

          <H2 id="change">Changing your mind</H2>
          <P>
            Clear this site&rsquo;s data in your browser and the banner returns. Or use a content
            blocker — the site is built to work without analytics loading at all, and nothing on it
            depends on that request succeeding.
          </P>

          <H2 id="contact">Contact</H2>
          <P>
            This is an open-source project. Questions and corrections belong in an issue on{" "}
            <A href="https://github.com/guml-lang/guml/issues">GitHub</A>.
          </P>
        </div>
      </article>
    </div>
  );
}
