/**
 * Applications to generate, and what counts as correct for each.
 *
 * This is not Phase 0. Phase 0 measures GUML against a React baseline to decide whether the
 * project continues. This is a product test: given the shipping prompt and a hosted model,
 * do we get documents that compile, validate, and actually contain the app that was asked
 * for?
 *
 * `must` predicates run against the parsed AST rather than the source text, so a check like
 * "there is a numeric input bound to state" cannot be satisfied by a comment that happens to
 * mention it. Each predicate returns true when the requirement is met.
 */

/** Every element in the tree, flattened. */
const els = (ast) => {
  const out = [];
  const walk = (list) => {
    for (const el of list ?? []) {
      out.push(el);
      walk(el.children);
    }
  };
  walk(ast.tree);
  return out;
};

const tags = (ast) => els(ast).map((e) => e.tag);
const has = (ast, tag) => tags(ast).includes(tag);
const count = (ast, tag) => tags(ast).filter((t) => t === tag).length;

/** Text of every positional, attribute, action and prose line — for binding checks. */
const allText = (ast) =>
  JSON.stringify(ast.tree ?? []) + JSON.stringify(ast.states ?? []) + JSON.stringify(ast.resources ?? []);

const stateNames = (ast) => (ast.states ?? []).map((s) => s.name);
const actionsOf = (ast) => els(ast).flatMap((e) => e.actions ?? []);
const bindingsOf = (ast) =>
  els(ast).flatMap((e) => [
    ...(e.positionals ?? []).filter((p) => p.Binding).map((p) => p.Binding),
    ...(e.attrs ?? []).filter((a) => a.value?.Binding).map((a) => a.value.Binding),
  ]);

/** A binding that does arithmetic or comparison rather than just naming a state. */
const hasComputedBinding = (ast) =>
  bindingsOf(ast).some((b) => /[+\-*/<>]|\b(count|sum)\b/.test(b)) ||
  els(ast).some((e) => /[{][^}]*[+\-*/][^}]*[}]/.test(e.content ?? ""));

export const APPS = [
  {
    id: "todo",
    title: "Todo app",
    prompt: `Build a todo list page backed by a REST API at /api/todos. GET returns Todo objects
with id, title and done. POST creates one from a title. PATCH /api/todos/{id} updates done.
DELETE /api/todos/{id} removes one. Someone should be able to add a todo from a text field,
tick it off, delete it, and filter between all, open and done. Show how many are still open.
Adding, ticking and deleting should feel instant and roll back if the request fails.`,
    must: [
      ["declares a resource with mutations", (a) => (a.resources ?? []).some((r) => r.mutations.length >= 2)],
      ["renders the rows with a repeater", (a) => has(a, "list") || has(a, "table")],
      ["has a text field bound to state", (a) => has(a, "input") && stateNames(a).length > 0],
      ["an action creates a row", (a) => actionsOf(a).some((x) => /\.(add|create|post)/i.test(x))],
      ["an action deletes a row", (a) => actionsOf(a).some((x) => /\.(drop|delete|remove)/i.test(x))],
      ["a control toggles done", (a) => has(a, "check") || has(a, "toggle")],
      ["mutations declare optimistic behaviour", (a) => (a.resources ?? []).some((r) => r.mutations.some((m) => m.optimistic))],
      ["filters with an enumerated state", (a) => (a.states ?? []).some((s) => (s.domain ?? []).length >= 2)],
    ],
  },

  {
    id: "bmi",
    title: "BMI calculator",
    prompt: `Build a BMI calculator page. Two number fields: height in centimetres and weight in
kilograms. Show the resulting BMI as a single large number, and below it the category —
underweight, normal, overweight or obese. It is entirely client-side; there is no API. Include
a reset button.`,
    must: [
      ["declares numeric state for both inputs", (a) => (a.states ?? []).filter((s) => typeof s.init?.Num === "number").length >= 2],
      ["has two fields", (a) => count(a, "input") >= 2],
      ["shows a computed result", (a) => hasComputedBinding(a)],
      ["displays the number prominently", (a) => has(a, "metric") || has(a, "h1") || has(a, "h")],
      ["has a reset action", (a) => actionsOf(a).some((x) => /=\s*0|reset/i.test(x))],
      ["declares no API resource", (a) => (a.resources ?? []).length === 0],
    ],
  },

  {
    id: "tip",
    title: "Tip splitter",
    prompt: `Build a tip splitter. Fields for the bill total, the tip percentage, and the number
of people. Show the tip amount, the total with tip, and the amount each person owes. Client-side
only, no API.`,
    must: [
      ["three numeric states", (a) => (a.states ?? []).filter((s) => typeof s.init?.Num === "number").length >= 3],
      ["three fields", (a) => count(a, "input") >= 3],
      ["computes from the inputs", (a) => hasComputedBinding(a)],
      ["shows more than one derived figure", (a) => bindingsOf(a).filter((b) => /[+\-*/]/.test(b)).length >= 2],
      ["declares no API resource", (a) => (a.resources ?? []).length === 0],
    ],
  },

  {
    id: "expenses",
    title: "Expense tracker",
    prompt: `Build an expense tracker backed by /api/expenses. GET returns Expense objects with
id, label, amount and category (food, travel or other). POST creates one. DELETE
/api/expenses/{id} removes one. Show a running total of all expenses, a form to add one, a
table of them, and a way to filter by category.`,
    must: [
      ["declares the resource and its type", (a) => (a.resources ?? []).length === 1 && (a.types ?? []).length >= 1],
      ["renders a table or list", (a) => has(a, "table") || has(a, "list")],
      ["shows an aggregate total", (a) => /\.(sum|total)|\bsum\b/.test(allText(a))],
      ["has an add form", (a) => has(a, "form") && has(a, "input")],
      ["deletes an expense", (a) => actionsOf(a).some((x) => /\.(drop|delete|remove)/i.test(x))],
      ["filters by an enumerated state", (a) => (a.states ?? []).some((s) => (s.domain ?? []).length >= 3)],
    ],
  },

  {
    id: "signup",
    title: "Sign-up form",
    prompt: `Build a sign-up form: full name, email, password, a country dropdown limited to GB,
US and DE, and a checkbox to accept the terms. The submit button must be disabled until the
name and email are filled in and the terms are accepted. Submit posts to /api/signup.`,
    must: [
      ["has a form", (a) => has(a, "form")],
      ["three or more text fields", (a) => count(a, "input") >= 3],
      ["a dropdown over an enumerated state", (a) => has(a, "select") && (a.states ?? []).some((s) => (s.domain ?? []).length === 3)],
      ["a checkbox or toggle for the terms", (a) => has(a, "check") || has(a, "toggle")],
      ["the submit button is conditionally disabled", (a) => els(a).some((e) => (e.attrs ?? []).some((x) => x.name === "disabled" && x.value?.Binding))],
      ["submits somewhere", (a) => actionsOf(a).length > 0],
    ],
  },

  {
    id: "dashboard",
    title: "Sales dashboard",
    prompt: `Build a sales dashboard backed by /api/orders. GET returns Order objects with id,
customer, amount and status (new, shipped or refunded). Across the top show four KPI tiles:
order count, total revenue, how many are new, and how many were refunded. Below them a table of
orders. A segmented control filters the table by status.`,
    must: [
      ["declares the resource and type", (a) => (a.resources ?? []).length === 1 && (a.types ?? []).length >= 1],
      ["four KPI tiles", (a) => count(a, "metric") >= 4],
      ["KPIs are derived, not hard-coded", (a) => bindingsOf(a).filter((b) => /\.(count|sum)\b/.test(b)).length >= 2],
      ["renders a table", (a) => has(a, "table") || has(a, "list")],
      ["filters with a segmented control", (a) => has(a, "tabs")],
      ["the filter state is enumerated", (a) => (a.states ?? []).some((s) => (s.domain ?? []).length >= 3)],
    ],
  },
];
