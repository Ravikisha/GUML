import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Elements",
  description: "Tag kinds, children, repeaters, and the tags whose children are content lines.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/elements"
      meter={{ label: "kinds", value: "5" }}
      title="Elements"
      lede="Every tag has a kind, and the kind decides how the rest of its line and its children are read."
      toc={[
        { id: "kinds", title: "Tag kinds" },
        { id: "containers", title: "Containers" },
        { id: "text", title: "Text" },
        { id: "controls", title: "Controls and fields" },
        { id: "repeaters", title: "Repeaters" },
        { id: "content-children", title: "Content-line children" },
      ]}
    >
      <H2 id="kinds">Tag kinds</H2>
      <Table
        head={["kind", "line remainder", "children", "tags"]}
        rows={[
          [
            "Container",
            "structured",
            "elements",
            <C key="a">card row col section nav hero footer form tabs</C>,
          ],
          ["Text", "prose (verbatim)", "none", <C key="b">h h1 h2 p text metric head empty</C>],
          ["Control", "structured", "none", <C key="c">btn link check toggle</C>],
          ["Field", "structured", "none", <C key="d">input select</C>],
          ["Repeater", "structured", "item template", <C key="e">list table</C>],
        ]}
      />
      <P>
        The kind is not cosmetic: it is what resolves the ambiguity between{" "}
        <C>btn Decrement ghost</C> (a label and a modifier) and <C>p Press the button.</C> (prose).
        See <A href="/docs/language/registry">the registry</A> for the authoritative list.
      </P>

      <H2 id="containers">Containers</H2>
      <CodeBlock
        lang="guml"
        code={`card sm center
  h Clicks
  row center
    btn Decrement ghost >count--
    btn Increment primary >count++`}
      />
      <P>
        Layout comes from the tag plus modifiers — <C>row center</C>, <C>col</C>, <C>cols=3</C> —
        never from utility classes. A container&rsquo;s first quoted positional becomes its title:
      </P>
      <CodeBlock
        lang="guml"
        code={`card "Ship in minutes" | Describe the page, get a deployable build.`}
      />

      <H2 id="text">Text</H2>
      <P>
        Text tags take the line remainder as prose. Bindings inside prose still interpolate, so a
        heading can carry a live count:
      </P>
      <CodeBlock
        lang="guml"
        code={`head Tasks — {tasks.open.count} open
metric {count}
empty Nothing here yet.`}
      />
      <UL>
        <LI>
          <C>metric</C> is for a single large number — counters, KPI tiles.
        </LI>
        <LI>
          <C>empty</C> belongs to the enclosing repeater: it is the message shown when the resource
          comes back with nothing.
        </LI>
        <LI>
          A text tag with an <C>=</C> on the line is parsed structurally, which is how{" "}
          <C>text {"{title}"} strike=&#123;done&#125;</C> works.
        </LI>
      </UL>

      <H2 id="controls">Controls and fields</H2>
      <CodeBlock
        lang="guml"
        code={`btn Add primary disabled={!draft.trim()} busy="Adding…"
link Features #features
check {done} >tasks.save
input draft placeholder="Add a task…"`}
      />
      <Table
        head={["tag", "first positional", "behaviour"]}
        rows={[
          ["btn", "label", <>action runs on click; <C key="a">busy</C> gives the pending label</>],
          ["link", "label", "target is a /route or #anchor"],
          ["check", "the bound field", "action runs on change"],
          ["toggle", "the bound field", "on/off switch"],
          ["input", "the state it binds", "text field; kind= for email, number, password"],
          ["select", "the enumerated state it binds", "options come from the state's domain"],
        ]}
      />
      <Note tone="warn" title="Labels are not optional">
        <p>
          A control with no text label needs <C>aria=&quot;…&quot;</C>. Missing it is{" "}
          <C>GUML0050</C> — a hard error, not a lint warning. That rule is where the
          &ldquo;convention as correctness&rdquo; claim is actually enforced.
        </p>
      </Note>

      <H2 id="repeaters">Repeaters</H2>
      <CodeBlock
        lang="guml"
        code={`list tasks where={filter}
  check {done} >tasks.save
  text {title} strike={done}
  btn Delete quiet aria="Delete {title}" >tasks.drop
  empty Nothing here yet.`}
      />
      <P>
        A repeater&rsquo;s children are the template for one item. Inside it, bare field names
        resolve against the resource&rsquo;s type, and mutations know which record they act on. The
        compiler supplies the loading skeleton, the empty state, the error banner, and the keys.
      </P>
      <Table
        head={["attribute", "meaning"]}
        rows={[
          [<C key="a">where=&#123;filter&#125;</C>, "filter by an enumerated state"],
          [<C key="b">sort=</C>, "field to order by"],
          [<C key="c">of=</C>, "iterate a nested collection instead of the resource root"],
        ]}
      />

      <H2 id="content-children">Content-line children</H2>
      <P>
        Two tags take content lines rather than elements, because wrapping every perk or question in
        its own tag is exactly the per-line overhead GUML exists to remove.
      </P>
      <CodeBlock
        lang="guml"
        code={`tier Pro $24/mo "For working developers" cta="Go Pro" /signup featured
  Unlimited projects
  Custom domains
  Email support

faq open=1
  Can I export the code? | Yes. Every build is plain source.
  Do I need a card to try it? | No. The free tier needs no payment details.`}
      />
      <P>
        In a <C>faq</C>, the <C>|</C> splits question from answer. In a <C>tier</C>, each line is one
        perk.
      </P>
    </DocPage>
  );
}
