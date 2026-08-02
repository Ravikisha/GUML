import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { CodePreview } from "@/components/code-preview";
import { SAMPLES } from "@/lib/samples";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Bindings & actions",
  description:
    "Derived values with {expr}, behaviour with >, and declared effects whose dependency cannot be wrong.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/bindings"
      meter={{ label: "expression language", value: "parsed, typed, lowered" }}
      title="Bindings &amp; actions"
      lede="Bindings read state. Actions change it. Both are deliberately small — small enough that the compiler can reason about them, and small enough that a model cannot write an effect-dependency bug."
      toc={[
        { id: "bindings", title: "Bindings" },
        { id: "derived", title: "Derived values" },
        { id: "state-field", title: "How .open and .done resolve" },
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
          [<C key="b">{"{tasks.open.count}"}</C>, "how many are not yet in the terminal state"],
          [<C key="b2">{"{invoices.amount.sum}"}</C>, "sum of one numeric field across the rows"],
          [<C key="c">{"{!draft.trim()}"}</C>, "boolean negation of a state expression"],
          [<C key="d">{"{done}"}</C>, "field of the item in scope"],
          [<C key="e">{"{count > 0}"}</C>, "comparison"],
        ]}
      />
      <P>
        Aggregates are typed. <C>.sum</C> on a string field, <C>.trim</C> on a number, or{" "}
        <C>.open</C> on something that is not a collection are all <C>GUML0065</C> — a compile error
        naming the type it found, not a <C>NaN</C> in the rendered page.
      </P>

      <H2 id="state-field">
        <C>.open</C> and <C>.done</C> find the state field
      </H2>
      <P>
        These two mean <em>not yet in the terminal state</em> and <em>in it</em>. Which field carries
        that state is read from the row type: whichever field is declared <C>bool</C>. An
        invoice&rsquo;s is <C>paid</C>, a task&rsquo;s is <C>done</C>, a message&rsquo;s is{" "}
        <C>read</C>, and all three work without configuration.
      </P>
      <CodeBlock
        lang="guml"
        code={`type Invoice {id, client, amount:number, paid:bool}
data invoices:Invoice[] GET /api/invoices

head Invoices — {invoices.open.count} awaiting payment`}
      />
      <P>
        A row type with <em>no</em> boolean field has no state to filter on, and one with{" "}
        <em>two</em> is ambiguous — <C>paid</C> and <C>overdue</C> are different questions. Both are{" "}
        <C>GUML0065</C>, and the ambiguous case names the fields that collided so you can use{" "}
        <C>where=</C> instead.
      </P>
      <Note tone="warn" title="This used to be a silent wrong answer">
        <p>
          Until the type checker landed, <C>.open</C> required a field literally named <C>done</C>{" "}
          and lowered to <C>{"!it.done"}</C> regardless. A published example modelling invoices with{" "}
          <C>paid:bool</C> compiled to <C>{"invoices.filter((it) => !it.done).length"}</C> — always
          zero, with no diagnostic. Both the rule and the lowering agreed with each other and were
          jointly wrong, which is the failure mode invariant 3 exists to prevent.
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
            "optimistic apply, request, then a state reset — rollback on failure",
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
        The form on its own, with the field and the button that submit it — this one runs, so the
        optimistic insert and the disabled-while-empty rule are the compiler&rsquo;s, not a
        description of them:
      </P>
      <CodePreview {...SAMPLES["bindings.mutations"]} />
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
        cases where a refetch is genuinely conditional. <C>{">tasks.list"}</C> re-runs a
        resource&rsquo;s own GET — every <C>data</C> declaration has one, and it is callable from a
        Reload button too.
      </P>
      <P>
        The trigger <em>is</em> the dependency, which is the whole reason this is a directive rather
        than a <C>js</C> block containing a <C>useEffect</C>. A dependency array is a second list that
        has to agree with the first, and it is wrong in two directions: a missing entry reads stale
        values, a spurious one loops forever. Neither mistake is available here.
      </P>
      <Table
        head={["GUML", "React", "Svelte 5"]}
        rows={[
          [
            <C key="a">{"on mount >tasks.list"}</C>,
            <C key="b">{"useEffect(() => { tasksList(); }, [])"}</C>,
            <C key="c">{"onMount(() => { tasksList(); })"}</C>,
          ],
          [
            <C key="d">{"on {filter} >tasks.list"}</C>,
            <C key="e">{"useEffect(() => { tasksList(); }, [filter])"}</C>,
            <C key="f">{"$effect(() => { void filter; untrack(…) })"}</C>,
          ],
        ]}
      />
      <P>
        The Svelte column is the strongest argument for compiling this. <C>$effect</C> tracks{" "}
        <em>every</em> reactive read in its body, so the obvious translation would re-run whenever
        anything the action touches changes — not when the declared trigger does. Reading the trigger
        and wrapping the body in <C>untrack</C> makes the dependency exactly what was written, and
        nobody writes that by hand.
      </P>
      <P>
        An effect is not a second-class citizen: same action language, same diagnostics.{" "}
        <C>{"on mount >tasks.remove"}</C> is the same <C>GUML0061</C> as the same action on a button, a
        malformed <C>on</C> is <C>GUML0098</C>, and a state read only by a trigger counts as used, so
        the optimizer does not delete it out from under the effect.
      </P>
      <Note tone="warn" title="The static HTML backend cannot honour these">
        <p>
          There is no runtime, so nothing runs them and the page renders as it was on first paint. It
          says so rather than staying quiet — a page that looks complete and silently never refetches
          is the failure invariant 3 exists to prevent.
        </p>
      </Note>

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
        too small. See <A href="/research/measurements">measurements</A>.
      </P>
    </DocPage>
  );
}
