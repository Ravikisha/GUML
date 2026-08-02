/**
 * Tests for the runtime evaluator.
 *
 * These matter more than their size suggests: the evaluator is the one place where
 * the browser runtime re-implements semantics the Rust compiler also owns
 * (`crates/guml-codegen/src/expr.rs`). If the two disagree, a live preview shows
 * something the emitted code would not do — the single most misleading failure
 * this project could ship. Several cases below exist purely to pin that parity.
 *
 * Run with `node --test` (Node strips the types natively).
 */

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { describe, it } from "node:test";
// `./eval.ts`, not `./eval.js`: Node runs this file directly with type stripping and does not
// rewrite specifiers, so it needs the file that exists. The library source uses `.js` because
// that is what the *published* ESM has to say. Both resolve under TypeScript.
import { evaluate, interpolate, runAction, shouldRequest, truthy } from "./eval.ts";

const TASKS = [
  { id: "1", title: "Ship it", done: true },
  { id: "2", title: "Write tests", done: false },
  { id: "3", title: "Measure tokens", done: false },
];

describe("evaluate — paths", () => {
  it("reads state", () => {
    assert.equal(evaluate("count", { count: 3 }), 3);
    assert.equal(evaluate("draft", { draft: "hi" }), "hi");
  });

  it("reads nested fields", () => {
    assert.equal(evaluate("user.name", { user: { name: "Ada" } }), "Ada");
  });

  it("returns undefined for a missing path rather than throwing", () => {
    // The resolver reports the undeclared name as GUML0033 at compile time; the
    // runtime must not crash the page over it.
    assert.equal(evaluate("nope", {}), undefined);
    assert.equal(evaluate("user.missing.deep", { user: {} }), undefined);
  });
});

describe("evaluate — collection aggregates", () => {
  it("counts", () => {
    assert.equal(evaluate("tasks.count", { tasks: TASKS }), 3);
    assert.equal(evaluate("tasks.length", { tasks: TASKS }), 3);
  });

  it("filters open and done", () => {
    assert.equal(evaluate("tasks.open.count", { tasks: TASKS }), 2);
    assert.equal(evaluate("tasks.done.count", { tasks: TASKS }), 1);
  });

  it("matches what the Rust backend emits for the same expression", () => {
    // Rust lowers `tasks.open.count` to `tasks.filter((it) => !it.done).length`.
    // Evaluating both against the same data has to agree, or preview and emitted
    // code diverge.
    const viaRuntime = evaluate("tasks.open.count", { tasks: TASKS });
    const viaEmittedShape = TASKS.filter((it) => !it.done).length;
    assert.equal(viaRuntime, viaEmittedShape);
  });

  it("sums", () => {
    assert.equal(evaluate("nums.sum", { nums: [1, 2, 3] }), 6);
  });

  it("finds the state field by shape, not by the name `done`", () => {
    // Both this evaluator and the Rust backend used to hardcode `done`. They agreed with each other
    // and were wrong together for every other name, and nothing here tested a second one.
    const invoices = [
      { id: "1", amount: 10, paid: true },
      { id: "2", amount: 20, paid: false },
      { id: "3", amount: 30, paid: false },
    ];
    assert.equal(evaluate("invoices.open.count", { invoices }), 2);
    assert.equal(evaluate("invoices.done.count", { invoices }), 1);
  });

  it("agrees with the emitted filter for a field that is not `done`", () => {
    // The parity that matters: Rust now lowers `invoices.open.count` to
    // `invoices.filter((it) => !it.paid).length`. A preview that disagreed with shipped code would be
    // worse than no preview.
    const invoices = [
      { id: "1", paid: true },
      { id: "2", paid: false },
    ];
    assert.equal(
      evaluate("invoices.open.count", { invoices }),
      invoices.filter((it) => !it.paid).length,
    );
  });
});

describe("evaluate — string helpers", () => {
  it("trims, with or without call parentheses", () => {
    assert.equal(evaluate("draft.trim()", { draft: "  hi  " }), "hi");
    assert.equal(evaluate("draft.trim", { draft: "  hi  " }), "hi");
  });

  it("changes case and measures length", () => {
    assert.equal(evaluate("t.lower", { t: "ABC" }), "abc");
    assert.equal(evaluate("t.length", { t: "abc" }), 3);
  });

  it("negates a trimmed field, the disabled-button idiom", () => {
    assert.equal(evaluate("!draft.trim()", { draft: "   " }), true);
    assert.equal(evaluate("!draft.trim()", { draft: "x" }), false);
  });
});

describe("evaluate — operators", () => {
  it("compares strictly", () => {
    assert.equal(evaluate('filter == "open"', { filter: "open" }), true);
    assert.equal(evaluate('filter != "open"', { filter: "done" }), true);
    // GUML has no loose equality, so a string never equals a number.
    assert.equal(evaluate('n == "1"', { n: 1 }), false);
  });

  it("orders numbers", () => {
    assert.equal(evaluate("count > 0", { count: 1 }), true);
    assert.equal(evaluate("count <= 0", { count: 0 }), true);
  });

  it("does arithmetic with the usual precedence", () => {
    assert.equal(evaluate("1 + 2 * 3", {}), 7);
    assert.equal(evaluate("(1 + 2) * 3", {}), 9);
    assert.equal(evaluate("-count", { count: 5 }), -5);
  });

  it("concatenates when either side is a string", () => {
    assert.equal(evaluate('"a" + 1', {}), "a1");
  });

  it("short-circuits booleans", () => {
    assert.equal(evaluate("a && b", { a: true, b: "yes" }), "yes");
    assert.equal(evaluate("a || b", { a: "", b: "fallback" }), "fallback");
  });

  it("reads literals", () => {
    assert.equal(evaluate("true", {}), true);
    assert.equal(evaluate("42", {}), 42);
    assert.equal(evaluate('"text"', {}), "text");
  });

  it("throws on syntax outside the grammar instead of guessing", () => {
    // `eval` would happily run this. The point of a hand-written parser is that
    // it cannot: a GUML document may come from an untrusted agent.
    assert.throws(() => evaluate("globalThis.fetch@", {}));
  });
});

describe("truthy", () => {
  it("treats an empty collection as false, unlike JavaScript", () => {
    // `[]` is truthy in JS, which would render an empty list as if it had rows.
    assert.equal(truthy([]), false);
    assert.equal(truthy([1]), true);
    assert.equal(truthy(""), false);
    assert.equal(truthy(0), false);
  });
});

describe("interpolate", () => {
  it("substitutes bindings inside prose", () => {
    assert.equal(
      interpolate("Tasks — {tasks.open.count} open", { tasks: TASKS }),
      "Tasks — 2 open",
    );
  });

  it("leaves prose without bindings alone", () => {
    assert.equal(interpolate("plain text", {}), "plain text");
  });

  it("renders null and undefined as nothing rather than as words", () => {
    assert.equal(interpolate("a{missing}b", {}), "ab");
  });

  it("leaves an unparseable binding visible instead of blanking the UI", () => {
    assert.equal(interpolate("{bad@syntax}", {}), "{bad@syntax}");
  });

  it("matches what the React backend emits for a bound attribute", () => {
    // `crates/guml-codegen` lowers `aria="Delete {title}"` to the template literal
    // `` {`Delete ${item.title}`} ``. This is the same value at runtime. The Rust side
    // used to emit `aria-label="Delete {item.title}"` — quoted, so the braces reached
    // the DOM and the accessible name read literally. The runtime was right and the
    // compiler was wrong; this test pins the agreement in the direction that failed.
    assert.equal(interpolate("Delete {title}", { title: "Write the spec" }), "Delete Write the spec");
  });
});

describe("runAction", () => {
  it("increments and decrements", () => {
    assert.deepEqual(runAction("count++", { count: 1 }), [
      { kind: "set", name: "count", value: 2 },
    ]);
    assert.deepEqual(runAction("count--", { count: 1 }), [
      { kind: "set", name: "count", value: 0 },
    ]);
  });

  it("treats a missing counter as zero", () => {
    assert.deepEqual(runAction("count++", {}), [{ kind: "set", name: "count", value: 1 }]);
  });

  it("assigns an evaluated expression", () => {
    assert.deepEqual(runAction("count=0", { count: 9 }), [
      { kind: "set", name: "count", value: 0 },
    ]);
    assert.deepEqual(runAction('draft=""', { draft: "x" }), [
      { kind: "set", name: "draft", value: "" },
    ]);
  });

  it("sequences statements with `;` in order", () => {
    const effects = runAction('tasks.add{title:draft}; draft=""', { draft: "New" });
    assert.equal(effects.length, 2);
    assert.deepEqual(effects[0], {
      kind: "mutate",
      resource: "tasks",
      mutation: "add",
      body: { title: "New" },
    });
    assert.deepEqual(effects[1], { kind: "set", name: "draft", value: "" });
  });

  it("reads a body-less mutation as acting on the item in scope", () => {
    assert.deepEqual(runAction("tasks.drop", {}), [
      { kind: "mutate", resource: "tasks", mutation: "drop", body: {} },
    ]);
  });

  it("does not mistake a comparison for an assignment", () => {
    // `>=` and friends contain `=`; a naive split would turn them into a set.
    assert.throws(() => runAction("count >= 1", { count: 1 }));
  });

  it("rejects an unsupported action rather than inventing one", () => {
    assert.throws(() => runAction("window.location = 'x'", {}));
  });
});

describe("shouldRequest", () => {
  it("issues a declared request by default", () => {
    assert.equal(shouldRequest({ url: "/api/tasks" }), true);
  });

  it("issues nothing when offline", () => {
    // The docs previews run this way: no server, so a mutation would 404 and the optimistic rollback
    // would undo it. A row that appears and vanishes reads as a language bug rather than a missing
    // endpoint, and no diagnostic fires because the rollback is behaving exactly as specified.
    assert.equal(shouldRequest({ offline: true, url: "/api/tasks" }), false);
  });

  it("issues nothing without a url", () => {
    assert.equal(shouldRequest({ url: null }), false);
    assert.equal(shouldRequest({ url: "" }), false);
    assert.equal(shouldRequest({}), false);
  });
});

describe("the offline gate covers every request site", () => {
  it("guards as many request sites as it has", async () => {
    // The original bug was a *missing* copy of this check: the mount fetch skipped seeded resources
    // while mutations went to the network regardless. Counting is what catches the next request site
    // added without a gate — a behavioural test of today's three would pass right through it.
    const src = await readFile(new URL("./react.tsx", import.meta.url), "utf8");
    // Actual invocations, not the `typeof fetch === "undefined"` capability probes.
    const invocations = src.match(/(?:void|await) fetch\(/g) ?? [];
    const gates = src.match(/shouldRequest\(/g) ?? [];
    assert.equal(
      invocations.length,
      gates.length,
      `${invocations.length} request sites but ${gates.length} gates — one issues a request unguarded`,
    );
    // Mount fetch, `.list` refetch, and each mutation.
    assert.equal(invocations.length, 3);
  });
});
