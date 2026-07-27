import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Bindings & actions",
  description: "Derived values with {expr}, and behaviour with > — deliberately not a general-purpose language.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/bindings"
      meter={{ label: "expression language", value: "pass-through in v0.1", tone: "ember" }}
      title="Bindings &amp; actions"
      lede="Bindings read state. Actions change it. Both are deliberately small — small enough that the compiler can reason about them, and small enough that a model cannot write an effect-dependency bug."
      toc={[
        { id: "bindings", title: "Bindings" },
        { id: "derived", title: "Derived values" },
        { id: "actions", title: "Actions" },
        { id: "mutations", title: "Resource mutations" },
        { id: "effects", title: "Effects" },
        { id: "limits", title: "Deliberate limits" },
      ]}
    >
      <H2 id="bindings">Bindings</H2>
      <P>
        <C>{"{expr}"}</C> anywhere a value is expected: in a positional, an attribute, or inside
        prose.
      </P>
      <CodeBlock
        lang="guml"
        code={`metric {count}
text {title} strike={done}
btn Add primary disabled={!draft.trim()}
head Tasks — {tasks.open.count} open
btn Delete quiet aria="Delete {title}" >tasks.drop`}
      />
      <P>
        Inside a repeater, bare field names resolve against the resource&rsquo;s type, so{" "}
        <C>{"{title}"}</C> in a <C>list tasks</C> is the current task&rsquo;s title. Unknown fields
        are a compile error with a suggestion rather than <C>undefined</C> at runtime.
      </P>

      <H2 id="derived">Derived values</H2>
      <P>
        Bindings are read-only by construction. A derived value is an expression, never a piece of
        state — which is how an entire class of React bug disappears: there is no memo to
        invalidate and no dependency array to keep in sync.
      </P>
      <Table
        head={["form", "meaning"]}
        rows={[
          [<C key="a">{"{tasks.count}"}</C>, "how many items the resource holds"],
          [<C key="b">{"{tasks.open.count}"}</C>, "filtered aggregate"],
          [<C key="c">{"{!draft.trim()}"}</C>, "boolean negation of a state expression"],
          [<C key="d">{"{done}"}</C>, "field of the item in scope"],
          [<C key="e">{"{count > 0}"}</C>, "comparison"],
        ]}
      />
      <Note tone="warn" title="v0.1 passes expressions through">
        <p>
          The expression language is specified — paths, comparison, boolean, arithmetic and
          collection aggregates — but v0.1 forwards the text of a binding to the target framework
          rather than parsing it. Field checking and aggregate lowering arrive with the resolver in
          Phase 3. Until then a mistyped field inside a binding reaches the emitted code.
        </p>
      </Note>

      <H2 id="actions">Actions</H2>
      <P>
        <C>{">"}</C> then statements separated by <C>;</C>. The action must be last on its line,
        because it consumes the remainder.
      </P>
      <Table
        head={["action", "emitted (React backend)"]}
        rows={[
          [<C key="a">{">count++"}</C>, <C key="b">setCount(count + 1)</C>],
          [<C key="c">{">count--"}</C>, <C key="d">setCount(count - 1)</C>],
          [<C key="e">{">count=0"}</C>, <C key="f">setCount(0)</C>],
          [<C key="g">{'>draft=""'}</C>, <C key="h">setDraft(&quot;&quot;)</C>],
          [
            <C key="i">{'>tasks.add{title:draft}; draft=""'}</C>,
            "resource mutation, then a state reset (Phase 3)",
          ],
        ]}
      />
      <P>
        Which DOM event an action binds to comes from the tag, not from the author: <C>btn</C> gets{" "}
        <C>onClick</C>, <C>check</C> and <C>toggle</C> get <C>onChange</C>, <C>form</C> gets{" "}
        <C>onSubmit</C>.
      </P>

      <H2 id="mutations">Resource mutations</H2>
      <CodeBlock
        lang="guml"
        code={`data tasks:Task[] GET /api/tasks
  add  POST   /api/tasks      {title}  optimistic:prepend
  drop DELETE /api/tasks/{id}          optimistic

form >tasks.add{title:draft}; draft=""
btn Delete quiet aria="Delete {title}" >tasks.drop`}
      />
      <P>
        <C>{">tasks.add{title:draft}"}</C> sends the body, applies the optimistic update, and — if
        the request fails — restores the snapshot and surfaces the error. Inside a repeater,{" "}
        <C>{">tasks.drop"}</C> needs no argument: the path parameter is filled from the item in
        scope.
      </P>

      <H2 id="effects">Effects</H2>
      <P>Effects are declared rather than written, so there is no dependency array to get wrong:</P>
      <CodeBlock
        lang="guml"
        code={`on mount >tasks.list
on {filter} >tasks.list`}
      />
      <P>
        A resource fetches on mount by default, so most pages need neither line. They exist for the
        cases where a refetch is genuinely conditional.
      </P>

      <H2 id="limits">Deliberate limits</H2>
      <UL>
        <LI>
          <strong className="text-chalk">Actions are not Turing-complete.</strong> No loops, no
          function definitions, no arbitrary calls. Anything more goes in a <C>js</C> block.
        </LI>
        <LI>
          <strong className="text-chalk">That boundary is also the security boundary.</strong> A
          declarative action set is what makes GUML safe to accept from an untrusted agent — the same
          principle A2UI adopts by refusing executable payloads entirely. GUML makes the escape
          hatch opt-in instead of impossible.
        </LI>
        <LI>
          <strong className="text-chalk">Bindings cannot assign.</strong> If you find yourself
          wanting that, you want state plus an action.
        </LI>
      </UL>
      <P>
        The cost of these limits is real, and it is measured: the fraction of tasks that need an
        escape hatch is tracked and published, because it is the early warning that the language is
        too small. See <A href="/docs/research/measurements">measurements</A>.
      </P>
    </DocPage>
  );
}
