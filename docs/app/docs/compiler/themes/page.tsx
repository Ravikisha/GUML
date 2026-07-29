import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Themes",
  description:
    "The mapping from semantic vocabulary to presentation, as data. Load a theme and every document compiled with it is re-themed — and the accessibility contract is enforced at load time.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/compiler/themes"
      meter={{ label: "classes in source", value: "0", tone: "mint" }}
      title="Themes"
      lede="GUML's contract is Markdown's: the document carries meaning, the host carries appearance. A theme is the second half of that — the CSS, except it is data the compiler reads."
      toc={[
        { id: "why", title: "Why appearance is data" },
        { id: "shape", title: "The shape of a theme" },
        { id: "rules", title: "How rules resolve" },
        { id: "contract", title: "The accessibility contract" },
        { id: "dark", title: "Light and dark" },
        { id: "integration", title: "Making your build see the classes" },
        { id: "css", title: "Stylesheets and the HTML backend" },
        { id: "using", title: "Loading one" },
      ]}
    >
      <H2 id="why">Why appearance is data</H2>
      <P>
        A <C>.guml</C> file has no classes in it. That is the token lever — class attributes were roughly
        a third of React&rsquo;s tokens in the landing fixture — and it is a correctness guarantee, because
        a model cannot get a presentational decision wrong if it never makes one.
      </P>
      <P>
        But for a while the other half was missing. The mapping was a <C>match</C> statement compiled into
        the binary, so &ldquo;the compiler owns presentation&rdquo; meant &ldquo;we own presentation, and
        you cannot have it&rdquo;. A colour literal inside a compiler is a theme nobody can override.
      </P>
      <Note>
        <p>
          The claim this makes true: swapping the table re-themes every page compiled with it. That was
          always the argument for owning presentation — it just was not reachable from outside until the
          table became a file.
        </p>
      </Note>

      <H2 id="shape">The shape of a theme</H2>
      <CodeBlock
        lang="json"
        filename="brand.json"
        code={`{
  "name": "brand",
  "contract": {
    "focus_visible": "focus-visible:outline focus-visible:outline-2 focus-visible:outline-brand-700",
    "min_contrast": 4.6,
    "disabled": "disabled:opacity-50"
  },
  "rules": [
    { "tag": "card", "base": "rounded-2xl bg-cream p-8 shadow-md" },
    { "tag": "h",    "base": "font-serif text-2xl text-ink" },
    { "tag": "btn",  "base": "rounded-full px-5 py-2 font-medium" },
    { "tag": "btn",  "when": ["primary"], "add": "bg-brand-700 text-cream", "group": "intent" },
    { "tag": "btn",  "add": "border border-ink/20 text-ink", "group": "intent" },
    { "tag": "*",    "when": ["full"], "add": "w-full" }
  ]
}`}
      />
      <P>
        Nothing here is Tailwind-specific. The values are class strings, and what they mean is the
        host&rsquo;s business — utility classes, BEM names, CSS modules, whatever the pipeline consumes.
      </P>

      <H2 id="rules">How rules resolve</H2>
      <Table
        head={["field", "meaning"]}
        rows={[
          [<C key="a">tag</C>, <>the tag this rule styles, or <C key="b">*</C> for every tag</>],
          [<C key="c">when</C>, "modifiers that must all be present for the rule to apply"],
          [
            <>
              <C key="d">base</C> / <C key="e">add</C>
            </>,
            "the classes contributed; two names for the same thing, because `base` reads better on the rule that establishes a tag's baseline",
          ],
          [
            <C key="f">group</C>,
            "rules sharing a group are mutually exclusive — first match wins, and a rule with an empty `when` is that group's fallback",
          ],
        ]}
      />
      <P>
        <C>group</C> is what makes <C>btn primary danger</C> pick one intent instead of concatenating two
        background colours. Tag rules apply before <C>*</C> rules, so a wildcard lands at the end of the
        class list.
      </P>

      <H2 id="contract">The accessibility contract</H2>
      <P>
        Handing the class table to a host trades one of the project&rsquo;s two claims against the other.
        The token saving is unaffected — the source is identical either way — but the correctness guarantee
        is not: a theme can specify unreadable colour pairs, or remove focus rings, and the compiler would
        emit it obediently.
      </P>
      <P>So a theme has to declare what it promises, and a theme that does not is refused:</P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml build page.guml --theme weak.json

error: theme \`weak\` declares \`min_contrast\` 3, below the WCAG AA floor of 4.5
       for body text`}
      />
      <UL>
        <LI>
          <C>focus_visible</C> is <strong>applied by the compiler</strong> to every focusable control, not
          written into each rule — so a theme cannot forget it on one control.
        </LI>
        <LI>
          <C>disabled</C> is applied to form controls only; on an <C>&lt;a&gt;</C> the utility would be
          inert.
        </LI>
        <LI>
          <C>min_contrast</C> is a declaration, checked at load time against the WCAG AA floor. It is the
          theme author asserting a property of their palette, not the compiler measuring it.
        </LI>
      </UL>
      <Note tone="warn" title="A caveat worth stating plainly">
        <p>
          The contrast floor is enforced as a <em>declaration</em>. A theme that claims 4.6 and ships an
          unreadable pair will pass. Measuring real ratios needs the resolved colour values, which the
          compiler does not have when the classes are opaque strings — so this catches carelessness, not
          dishonesty.
        </p>
      </Note>

      <H2 id="dark">Light and dark</H2>
      <P>
        The shipped theme emits <C>dark:</C> variants, so compiled output adapts to the reader&rsquo;s
        colour scheme without the document knowing anything about it. Only colour rules have variants —
        layout and type are scheme-independent, which keeps the table small.
      </P>
      <CodeBlock
        lang="tsx"
        filename="emitted from `card sm center`"
        code={`<div className="rounded-xl border border-slate-200 dark:border-slate-800
                bg-white dark:bg-slate-900 p-6 shadow-sm …">`}
      />
      <P>
        For the React backend those are ordinary Tailwind variants, so they follow whatever dark strategy
        the host already uses. The <C>html</C> backend has no build step, so its stylesheet implements
        both schemes directly: a <C>prefers-color-scheme</C> block for the default, and a{" "}
        <C>{'[data-theme="dark"]'}</C> block so a host with a theme toggle can force one. An explicit
        light choice wins over an OS set to dark.
      </P>
      <Note>
        <p>
          A theme is free to be single-scheme. Omit the <C>dark:</C> variants and the output is light-only
          — which is what the shipped theme was until it became data, because a light-only theme was the
          only thing a hardcoded table could reasonably be.
        </p>
      </Note>

      <H2 id="integration">Making your build see the classes</H2>
      <P>
        The one integration problem worth knowing about in advance. A utility-class framework generates
        only the classes it can find in your source — and GUML&rsquo;s classes are decided by the theme
        and produced by the <em>compiler at runtime</em>. Nothing in your repository contains them, so
        your build strips exactly the styles the compiler emits.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`# every class the active theme can emit, one per line
guml theme --classes

# feed it to a Tailwind @source, or a safelist
guml theme --classes > src/guml-classes.txt`}
      />
      <Note tone="warn" title="The failure is silent, not loud">
        <p>
          A missing class is not an error — it is a rule that does not exist, so the element renders
          unstyled. This site had it: <C>border-slate-200</C>, <C>text-slate-900</C>, <C>rounded-xl</C>{" "}
          and <C>divide-slate-200</C> were absent from its stylesheet for as long as live previews had
          existed, and a white panel behind the preview made the result look plausible enough that nobody
          checked. It is now generated by <C>pnpm gen:theme</C> and a CI step fails when it goes stale.
        </p>
      </Note>

      <H2 id="css">Stylesheets and the HTML backend</H2>
      <P>
        A theme may carry a <C>css</C> field. The <A href="/docs/compiler/backends">static HTML backend
        </A> has no build step by design, so it cannot run a utility-class compiler — it inlines this
        stylesheet instead, and the emitted document depends on nothing at render time.
      </P>
      <P>
        The shipped <C>slate</C> theme includes one implementing exactly the utilities it emits, and a test
        fails if a rule gains a class the stylesheet does not have. A theme with no <C>css</C> still works
        for the React backend, where the host&rsquo;s own pipeline processes the classes; asking the HTML
        backend for an inline style without one is reported rather than silently emitting an unstyled page.
      </P>

      <H2 id="using">Loading one</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml build page.guml --theme brand.json
guml build page.guml --theme brand.json --backend html    # inlines brand.json's css
guml build page.guml --backend html-cdn                   # Tailwind CDN, previews only`}
      />
      <P>
        One theme per process, applied for the whole run — a document styled two ways would be worse than
        either. See <A href="/docs/language/modifiers">modifiers</A> for the vocabulary a theme keys on,
        and <C>spec/STABILITY.md</C> for why a modifier may be added but never re-pointed at a different
        meaning.
      </P>
    </DocPage>
  );
}
