/**
 * GUML-Bench: the dataset schema, the arms, and the validation that keeps both honest.
 *
 * # What this file is for
 *
 * The report specifies GUML-Bench precisely (§8.1–8.3): 150 tasks across 6 categories, nine comparison
 * arms, three models. That specification is worth nothing until something *enforces* it, and the two ways
 * a benchmark like this goes wrong are both silent:
 *
 *   1. **The arms are not asked the same thing.** If the React arm's prompt is softer than the GUML arm's,
 *      the result is rigged, and it is rigged in a way no reviewer can see from the aggregate numbers. So
 *      a task has *one* prompt and one checklist, and `validate` refuses a task that tries to vary either
 *      per arm.
 *   2. **The dataset is thinner than the paper says.** 150 is the target; a partial set is fine and normal
 *      while it is being built, and quoting a category average over four tasks as though it were
 *      twenty-five is not. `coverage()` reports the real counts per category, and `guml-bench report`
 *      prints them next to every figure it produces.
 *
 * # Why per-category and never a single average
 *
 * The content floor makes an overall mean actively misleading. A landing page is mostly prose, and prose
 * is incompressible — so GUML's advantage there asymptotes at 2–3× while a CRUD app approaches 8×. Mixing
 * those into one number produces a figure that describes neither and moves with the *category mix* rather
 * than with anything about the language. `report` will not print an overall mean; it is not an oversight.
 */

/** The six categories, in the report's order (§8.1). */
export const CATEGORIES = [
  {
    id: "landing",
    title: "Landing / marketing pages",
    // Recorded per category because it is the single most important thing to know when reading a result,
    // and because writing it down in advance is what stops a disappointing number from being reframed
    // after the fact.
    expectation: "content-heavy — expect low compression, 2–3×",
    target: 25,
  },
  {
    id: "dashboard",
    title: "Dashboards",
    expectation: "chart + stat + table composition",
    target: 25,
  },
  {
    id: "crud",
    title: "CRUD applications",
    expectation: "state-heavy — expect high compression, approaching 8×",
    target: 25,
  },
  {
    id: "ecommerce",
    title: "E-commerce flows",
    expectation: "cart, variants, checkout — deep state, moderate prose",
    target: 25,
  },
  {
    id: "saas",
    title: "SaaS app screens",
    expectation: "settings, teams, billing, auth-gated routes",
    target: 25,
  },
  {
    id: "dataviz",
    title: "Data-visualization apps",
    expectation: "filter → query → chart interaction",
    target: 25,
  },
];

/**
 * The nine arms (§8.2).
 *
 * `available` is honest rather than aspirational. Two arms cannot run from this repository today and
 * saying so here means the report generator omits them with a reason instead of silently producing a
 * seven-arm table labelled nine.
 */
export const ARMS = [
  {
    id: "B1",
    title: "React + TS + Tailwind, direct",
    target: "react",
    available: true,
    note: "The baseline that matters. Everything else is measured against this.",
  },
  {
    id: "B2",
    title: "HTML/CSS/JS, direct",
    target: "html",
    available: true,
    note: "Separates 'React is verbose' from 'code is verbose'.",
  },
  {
    id: "B3",
    title: "JSON UI IR (A2UI-shaped)",
    target: "a2ui",
    available: true,
    note: "The first objection any reviewer raises: why not just emit JSON. The compiler emits this arm's format itself, so the comparison is against a real renderer target rather than a straw man.",
  },
  {
    id: "B4",
    title: "TOON-encoded IR",
    target: "toon",
    available: true,
    note: "The sharpest objection to the whole project: maybe the win is the *serialisation* and not the language. So B4 encodes the **same IR as B3** and nothing else — a hand-tuned structure for this arm would measure the tuning. `toon.mjs` also ships a decoder, and `selftest.mjs` asserts the encoding round-trips on every payload the report measures, because 'TOON is 29% smaller' and 'we deleted 29% of the characters' are otherwise the same claim.",
  },
  {
    id: "B5",
    title: "v0",
    target: "v0",
    available: false,
    unavailable: "Needs v0 API access, which this harness does not have.",
  },
  {
    id: "B6",
    title: "Human expert React",
    target: "human",
    available: true,
    note: "The quality ceiling, not a generation arm: these are the authored references in `references/`. Without it a reviewer cannot tell 'the model did well' from 'the task was easy'.",
  },
  {
    id: "T1",
    title: "GUML → compiler",
    target: "guml",
    available: true,
    note: "The treatment.",
  },
  {
    id: "T2",
    title: "T1 + grammar-constrained decoding",
    target: "guml-constrained",
    available: false,
    unavailable:
      "Hosted APIs expose no client-side CFG masking, so this arm needs a local/open model with llguidance. The report says so; recording it here keeps the table from implying it was run.",
  },
  {
    id: "T3",
    title: "T2 + compiler-feedback repair (≤3 rounds)",
    target: "guml-repair",
    available: true,
    note: "Runs on top of T1 rather than T2, since T2 is unavailable — which makes it 'T1 + repair'. Reported under that name, because calling it T3 would imply constrained decoding was involved.",
  },
];

/** The model grid (§8.2). Capability is a first-class variable (H6), not a footnote. */
export const MODELS = [
  { id: "claude-haiku-4-5-20251001", label: "Haiku 4.5" },
  { id: "claude-sonnet-5", label: "Sonnet 5" },
  { id: "claude-opus-5", label: "Opus 5" },
];

/**
 * Validate one task.
 *
 * Returns a list of problems. Every rule here exists because breaking it would make a *result* wrong
 * rather than merely making the file untidy.
 */
export function validateTask(task, seen = new Set()) {
  const problems = [];
  const need = (field) => {
    if (!task[field] || (Array.isArray(task[field]) && task[field].length === 0)) {
      problems.push(`missing \`${field}\``);
    }
  };
  need("id");
  need("title");
  need("category");
  need("prompt");
  need("checklist");

  if (task.id && seen.has(task.id)) problems.push(`duplicate id \`${task.id}\``);
  if (task.category && !CATEGORIES.some((c) => c.id === task.category)) {
    problems.push(`unknown category \`${task.category}\``);
  }

  // The checklist is the scoring instrument, not documentation. Too short and it cannot discriminate
  // between a working app and a screenshot of one.
  if (Array.isArray(task.checklist) && task.checklist.length < 8) {
    problems.push(
      `checklist has ${task.checklist.length} items; fewer than 8 cannot discriminate a working app from a plausible-looking one`,
    );
  }

  // The rigging rule. One prompt, one checklist, for every arm.
  for (const forbidden of ["prompts", "checklists", "reactPrompt", "gumlPrompt"]) {
    if (task[forbidden]) {
      problems.push(
        `\`${forbidden}\` would let the arms be asked different things, which rigs the comparison`,
      );
    }
  }

  return problems;
}

/**
 * Advisory observations about a task. Never fatal.
 *
 * Split from `validateTask` after its first run: the accessibility heuristic below produced two false
 * positives immediately, because "a quantity control" and "three plans to switch to" are controls and the
 * pattern did not know those words. A heuristic that fails a build on its own guess about English is worse
 * than one that mentions it and moves on — so these are notes, and `validateTask` keeps only the rules that
 * would make a *result* wrong.
 */
export function reviewTask(task) {
  const notes = [];
  const CONTROL_WORDS =
    /button|input|select|checkbox|form|filter|toggle|control|switch|selector|field|dialog|search|add|remove|edit|change|click|tick/i;
  if (
    Array.isArray(task.checklist) &&
    task.checklist.some((c) => /label|aria|keyboard/i.test(c)) &&
    !CONTROL_WORDS.test(task.prompt ?? "")
  ) {
    notes.push("checklist scores accessibility but the prompt seems to ask for no controls");
  }
  // A task with no `tags` gets the full registry, which is a different independent variable. Worth knowing
  // rather than discovering when the numbers do not line up.
  if (!task.tags) {
    notes.push("no `tags`, so the GUML arms will receive the full registry rather than a slice");
  }
  return notes;
}

/** Per-category counts against the report's target. */
export function coverage(tasks) {
  return CATEGORIES.map((c) => {
    const have = tasks.filter((t) => t.category === c.id).length;
    return { ...c, have, short: Math.max(0, c.target - have) };
  });
}

/** Arms that can actually run, and the reason each of the others cannot. */
export function armStatus() {
  return {
    available: ARMS.filter((a) => a.available),
    unavailable: ARMS.filter((a) => !a.available),
  };
}
