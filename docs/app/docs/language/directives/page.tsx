import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, LI, Note, P, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Directives",
  description: "page, type, state, data and the mutations that come with them.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/language/directives"
      meter={{ label: "implemented", value: "4 of 7" }}
      title="Directives"
      lede="Directives declare everything that is not markup: the page, its data shapes, its state, and its resources."
      toc={[
        { id: "page", title: "page" },
        { id: "type", title: "type" },
        { id: "state", title: "state" },
        { id: "data", title: "data" },
        { id: "mutations", title: "Mutations" },
        { id: "planned", title: "Planned" },
      ]}
    >
      <H2 id="page">page</H2>
      <CodeBlock lang="guml" code={`page Counter`} />
      <P>
        Names the file. The React backend converts it to PascalCase for the component and the
        filename, so <C>page task list</C> emits <C>TaskList.tsx</C>. Omitting it is a warning
        (<C>GUML0041</C>), not an error — the file still compiles, as <C>Page</C>.
      </P>

      <H2 id="type">type</H2>
      <CodeBlock lang="guml" code={`type Task {id, title, done:bool, createdAt:date}`} />
      <P>
        A shape, used to check bindings and to drive codegen — a <C>date</C> field gets formatted, a{" "}
        <C>bool</C> gets a checkbox. Fields default to <C>string</C>, so most of them need no
        annotation at all.
      </P>
      <Table
        head={["annotation", "meaning"]}
        rows={[
          [<C key="a">string</C>, "the default; omit it"],
          [<C key="b">bool</C>, "checkbox or toggle when bound to a control"],
          [<C key="c">int</C>, "numeric input, tabular alignment"],
          [<C key="d">date</C>, "formatted output"],
        ]}
      />
      <Note tone="info" title="Types exist to catch mistakes, not to prove soundness">
        <p>
          The type system is intentionally gradual and unsound. Its job is to turn a model&rsquo;s
          wrong field name into a compile error with a suggestion — not to satisfy a metatheory.
        </p>
      </Note>

      <H2 id="state">state</H2>
      <CodeBlock
        lang="guml"
        code={`state count=0                  // number
state draft=""                 // string
state filter=all|open|done     // enumerated domain; first value is initial`}
      />
      <P>
        <C>state</C> is page-scoped. <C>store</C> takes the same form for app-scoped state. An
        enumerated domain is more than a default: it gives the compiler an exhaustive set, which is
        what lets <C>tabs filter</C> render a segmented control with no options written by hand.
      </P>
      <P>
        Declaring the same name twice is an error (<C>GUML0040</C>). Derived values are{" "}
        <A href="/docs/language/bindings">bindings</A>, not state — there is deliberately no way to
        declare a memo.
      </P>

      <H2 id="data">data</H2>
      <CodeBlock
        lang="guml"
        code={`data tasks:Task[] GET /api/tasks
  add  POST   /api/tasks         {title}  optimistic:prepend
  save PATCH  /api/tasks/{id}    {done}   optimistic
  drop DELETE /api/tasks/{id}             optimistic`}
      />
      <P>
        One resource declaration, and the compiler generates the fetch layer, request cancellation,
        retry with backoff, the cache, loading and error state, the optimistic apply, and the
        rollback on failure. In the task fixture that is most of the 1,259-token difference between
        the GUML and React versions.
      </P>
      <Table
        head={["part", "form", "notes"]}
        rows={[
          ["name", <C key="a">tasks</C>, "how the tree refers to it"],
          ["type", <C key="b">:Task[]</C>, "optional; enables field checking on bindings"],
          ["method + url", <C key="c">GET /api/tasks</C>, "the list endpoint"],
          [
            "children",
            "one mutation per line",
            <>
              indented under the <C>data</C> line
            </>,
          ],
        ]}
      />

      <H2 id="mutations">Mutations</H2>
      <CodeBlock lang="guml" code={`name METHOD /url [{body,fields}] [optimistic[:strategy]]`} />
      <Table
        head={["strategy", "behaviour"]}
        rows={[
          [<C key="a">optimistic:prepend</C>, "insert at the head of the list immediately"],
          [<C key="b">optimistic:append</C>, "insert at the tail immediately"],
          [<C key="c">optimistic</C>, "replace in place (the default strategy)"],
          ["omitted", "wait for the server round trip before updating"],
        ]}
      />
      <P>
        A path parameter in the URL — <C>/api/tasks/&#123;id&#125;</C> — is filled from the item in
        scope, so a mutation inside a <C>list</C> already knows which record it is acting on. Body
        fields are read from state or from the surrounding item.
      </P>
      <P>
        Mutations are invoked from actions: <C>{">tasks.drop"}</C>,{" "}
        <C>{">tasks.add{title:draft}"}</C>.
      </P>

      <H2 id="planned">Planned</H2>
      <Table
        head={["directive", "purpose", "status"]}
        rows={[
          [<C key="a">route</C>, "map a path to a page, with guards", "Phase 2"],
          [<C key="b">auth</C>, "provider plus per-route guards", "Phase 2"],
          [<C key="c">def</C>, "user-defined components", "Phase 2"],
          [<C key="d">js</C> , "escape hatch for expressions and handlers", "Phase 2"],
          [<C key="e">raw</C>, "verbatim target-framework code", "Phase 2"],
        ]}
      />
      <Note tone="warn" title="Server, database and auth are deliberately out of v1">
        <p>
          Attempting client, server, schema and policy in one language at once is the most likely way
          this project fails. v1 is client-only; the rest waits for evidence. The{" "}
          <A href="/docs/research/roadmap">roadmap</A> says where each piece lands.
        </p>
      </Note>
      <UL>
        <LI>
          The escape hatches matter beyond convenience: the fraction of tasks that need them is a
          tracked metric, because it is the early warning that the vocabulary is too small.
        </LI>
      </UL>
    </DocPage>
  );
}
