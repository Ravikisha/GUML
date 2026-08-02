#!/usr/bin/env node
/**
 * The edit-locality benchmark.
 *
 * # The measurement, and why it is separate
 *
 * Every token figure in this project so far compares *writing an app from scratch*. That is not what
 * happens after the first prompt. What happens is: "make the delete button red", "add a due date column",
 * "filter by owner too" — and the interesting question becomes how many tokens a *change* costs.
 *
 * The naive comparison is rigged in GUML's favour and must not be used: regenerating a whole React file
 * against regenerating a whole GUML file just restates the compression ratio. Nobody edits React by
 * regenerating it. They send a diff.
 *
 * So the baseline here is **diff-based React editing** — the smallest unified diff that makes the change —
 * and the treatment is **diff-based GUML editing**. That is the honest comparison, and it is the one where
 * GUML's advantage is least obvious in advance: a one-line GUML change and a one-line React change are
 * both one line. Where GUML should win is the changes that are one line in GUML and *fifteen* in React,
 * because the compiler owns the fifteen — adding a filter, adding an optimistic mutation, adding a loading
 * state.
 *
 * A category of change where GUML does *not* win is a real finding and gets reported as one.
 *
 * # What a scripted modification is
 *
 * A named change, with a before and an after, hand-written in both representations. Hand-written because
 * the point is to measure the *minimum* cost of the change in each representation, and a model's diff is
 * not minimal. This measures the representations, not the models; `run.mjs` measures the models.
 *
 * Run from the repository root:
 *
 *   node bench/guml-bench/edit-locality.mjs
 *   node bench/guml-bench/edit-locality.mjs --json
 */
import { approxTokens, median } from "./metrics.mjs";

/**
 * A scripted modification, in both representations.
 *
 * `guml` and `react` are the *diffs* — added and removed lines only, which is what a model editing by diff
 * actually emits. Counting the whole file would measure regeneration, which is the comparison this
 * benchmark exists to avoid.
 */
const EDITS = [
  {
    id: "restyle-control",
    category: "cosmetic",
    what: "Make the delete button quiet instead of danger",
    // The case GUML should *not* win, and it is here first on purpose. A one-word change either way.
    guml: `-  btn Delete danger aria="Delete {title}" >tasks.drop
+  btn Delete quiet aria="Delete {title}" >tasks.drop`,
    react: `-              className="rounded-md bg-red-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-red-700"
+              className="rounded-md px-3 py-1.5 text-sm font-medium text-slate-500 hover:text-slate-900"`,
  },
  {
    id: "add-column",
    category: "structure",
    what: "Add a due-date column to the table",
    guml: `+  text {due}`,
    react: `+            <span className="flex-1 text-sm text-slate-900">{item.due}</span>`,
  },
  {
    id: "add-filter",
    category: "state",
    what: "Add an owner filter beside the existing status filter",
    // The shape GUML should win on: a filter is a declaration, and the memo plus the dependency array plus
    // the predicate chain are the compiler's.
    guml: `+state owner=all|ada|grace
+tabs owner
-list tasks where={filter}
+list tasks where={owner}`,
    react: `+  const [owner, setOwner] = useState<"all" | "ada" | "grace">("all");
+
+  const visibleTasks = useMemo(
+    () =>
+      owner === "all"
+        ? tasks.filter((it) => filter === "open" ? !it.done : filter === "done" ? it.done : true)
+        : tasks.filter(
+            (it) =>
+              it.owner === owner &&
+              (filter === "open" ? !it.done : filter === "done" ? it.done : true),
+          ),
+    [tasks, filter, owner],
+  );
+
+      <div role="tablist" className="flex items-center gap-2">
+        {(["all", "ada", "grace"] as const).map((value) => (
+          <button
+            key={value}
+            type="button"
+            role="tab"
+            aria-pressed={owner === value}
+            onClick={() => setOwner(value)}
+            className="rounded-md border border-slate-300 px-4 py-2 text-sm font-medium"
+          >
+            {value}
+          </button>
+        ))}
+      </div>`,
  },
  {
    id: "add-mutation",
    category: "state",
    what: "Add an archive action with an optimistic update and rollback",
    guml: `   drop DELETE /api/tasks/{id}          optimistic
+  archive POST /api/tasks/{id}/archive optimistic
+  btn Archive quiet aria="Archive {title}" >tasks.archive`,
    react: `+  const tasksArchive = useCallback(
+    async (item: Task) => {
+      const snapshot = tasks;
+      setTasks((prev) => prev.filter((it) => it !== item));
+      try {
+        const res = await retrying(\`/api/tasks/\${item.id}/archive\`, {
+          method: "POST",
+          headers: { "Content-Type": "application/json" },
+          body: JSON.stringify({}),
+        });
+        if (!res.ok) throw new Error(\`Request failed: \${res.status}\`);
+        invalidate("/api/tasks");
+      } catch (err: unknown) {
+        setTasks(snapshot);
+        setTasksError(err instanceof Error ? err.message : "Could not save");
+      }
+    },
+    [tasks],
+  );
+
+          <button
+            type="button"
+            aria-label={\`Archive \${item.title}\`}
+            onClick={() => { void tasksArchive(item); }}
+            className="rounded-md px-3 py-1.5 text-sm font-medium text-slate-500"
+          >
+            Archive
+          </button>`,
  },
  {
    id: "add-empty-state",
    category: "convention",
    what: "Add an empty state to a list that had none",
    guml: `+  empty Nothing here yet.`,
    react: `-        <ul className="mt-6 divide-y divide-slate-200">
-          {visibleTasks.map((item) => (
+        {visibleTasks.length === 0 ? (
+          <p className="mt-10 text-center text-sm text-slate-500">Nothing here yet.</p>
+        ) : (
+          <ul className="mt-6 divide-y divide-slate-200">
+            {visibleTasks.map((item) => (`,
  },
  {
    id: "add-prose",
    category: "content",
    what: "Add a paragraph of marketing copy",
    // The content floor, and the case where the two should be nearly equal: the prose itself is the cost.
    guml: `+p Northwind compiles a page description into a deployable build. No boilerplate to review, no framework to learn.`,
    react: `+        <p className="mt-1 text-sm text-slate-500">
+          Northwind compiles a page description into a deployable build. No boilerplate to review, no
+          framework to learn.
+        </p>`,
  },
  {
    id: "rename-field",
    category: "structure",
    what: "Rename a bound field across the document",
    guml: `-  text {title} strike={done}
+  text {name} strike={done}`,
    react: `-            <span className="flex-1 text-sm">{item.title}</span>
+            <span className="flex-1 text-sm">{item.name}</span>`,
  },
  {
    id: "add-loading-state",
    category: "convention",
    what: "Add a loading skeleton to a fetch that had none",
    guml: `(no edit — the compiler emits it from the \`data\` declaration)`,
    react: `+        {tasksLoading ? (
+          <ul className="mt-6 space-y-2">
+            {[0, 1, 2].map((n) => (
+              <li key={n} className="h-12 animate-pulse rounded-md bg-slate-100" />
+            ))}
+          </ul>
+        ) : (`,
    // The interesting one, and it needs saying out loud rather than showing as a zero: this change is not
    // *cheaper* in GUML, it is impossible to need. The document already declared a resource, and the
    // loading state came with it. A benchmark that scored this as "0 tokens vs 6 lines" would be measuring
    // the right thing and describing it wrongly.
    note: "Not an edit in GUML: a resource's loading state is generated from the declaration, so there is nothing to add.",
  },
];

/** Added and removed lines in a diff, ignoring context. */
function diffLines(diff) {
  return diff
    .split("\n")
    .filter((l) => l.startsWith("+") || l.startsWith("-"))
    .map((l) => l.slice(1));
}

function measure(edit) {
  const gumlLines = edit.note ? [] : diffLines(edit.guml);
  const reactLines = diffLines(edit.react);
  const gumlTokens = gumlLines.reduce((a, l) => a + approxTokens(l), 0);
  const reactTokens = reactLines.reduce((a, l) => a + approxTokens(l), 0);
  return {
    id: edit.id,
    category: edit.category,
    what: edit.what,
    gumlLines: gumlLines.length,
    reactLines: reactLines.length,
    approxGumlTokens: gumlTokens,
    approxReactTokens: reactTokens,
    // `null` rather than `Infinity` when the GUML edit is empty: an infinite ratio is not a measurement,
    // and the note explains what happened instead.
    ratio: gumlTokens === 0 ? null : Number((reactTokens / gumlTokens).toFixed(2)),
    note: edit.note ?? null,
  };
}

const results = EDITS.map(measure);

if (process.argv.includes("--json")) {
  const ratios = results.map((r) => r.ratio).filter((r) => r !== null);
  console.log(
    JSON.stringify(
      {
        counter: "approx (~3.6 chars/token) — never a published figure; see metrics.mjs",
        baseline: "diff-based React editing, not regeneration",
        edits: results,
        medianRatio: median(ratios),
        note: `${EDITS.length} scripted modifications. The report specifies 50 tasks × 3 modifications; this is a seed set that exercises the harness and is not a publishable sample.`,
      },
      null,
      2,
    ),
  );
  process.exit(0);
}

console.log("Edit locality — diff-based React editing vs diff-based GUML editing\n");
console.log(
  "Tokens are a ~3.6 chars/token estimate. The baseline is the smallest unified diff, not a",
);
console.log("regeneration: nobody edits React by regenerating it.\n");
console.log(
  `${"edit".padEnd(20)}${"category".padEnd(13)}${"GUML".padStart(6)}${"React".padStart(7)}${"ratio".padStart(8)}`,
);
console.log("-".repeat(54));
for (const r of results) {
  const ratio = r.ratio === null ? "     —" : `${r.ratio.toFixed(2)}×`.padStart(8);
  console.log(
    `${r.id.padEnd(20)}${r.category.padEnd(13)}${String(r.approxGumlTokens).padStart(6)}${String(r.approxReactTokens).padStart(7)}${ratio}`,
  );
}

const ratios = results.map((r) => r.ratio).filter((r) => r !== null);
console.log("-".repeat(54));
console.log(`median ratio over ${ratios.length} comparable edits: ${median(ratios)?.toFixed(2)}×\n`);

for (const r of results.filter((x) => x.note)) {
  console.log(`  ${r.id}: ${r.note}`);
}

// The two things a reader must not miss.
console.log(
  `\n${EDITS.length} scripted modifications. The report specifies 50 tasks × 3; this is a seed set that`,
);
console.log("exercises the harness and is not a publishable sample.");
console.log(
  "\nBoth sides are hand-written minimal diffs, so this measures the *representations*, not the models.",
);
