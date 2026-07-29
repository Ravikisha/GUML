/**
 * Docs samples that carry a live preview.
 *
 * # Why these live in one module
 *
 * Every sample here is compiled by `scripts/check-doc-previews.mjs` with the real `guml` CLI, and the
 * check fails on an error *or* a warning. A preview that renders a diagnostic panel instead of an
 * interface would undercut the page it sits on, and a sample can rot in two directions — the language
 * changes, or someone edits the snippet — so it is checked rather than eyeballed. Keeping the samples
 * importable is what makes that check possible without parsing TSX.
 *
 * # `scaffold`
 *
 * Most samples are fragments: `btn Add primary disabled={!draft.trim()}` is the sentence the
 * paragraph is about, and a `state draft=""` line above it would be noise on every sample. The
 * scaffold supplies exactly what the fragment references — no more, or the preview reports the
 * scaffold's own unused declarations. The reader is told the preview supplied it, and can display it.
 *
 * # What is deliberately *not* here
 *
 * Samples that cannot render honestly: annotated diagrams (the box-drawing callouts on the syntax
 * page), the EBNF grammar, shell transcripts, `PLANNED` directives, and the diagnostics pages' code —
 * which is broken on purpose, so a preview showing errors would be showing the point rather than
 * contradicting it. Declaration-only samples (`type Task {...}` on its own) are also excluded: they
 * compile, and they render an empty box.
 */

export type Sample = { code: string; scaffold?: string };

/** Referenced by several samples: the task resource the docs use as their running example. */
const TASKS = `type Task {id, title, done:bool}
data tasks:Task[] GET /api/tasks
  add  POST   /api/tasks      {title} optimistic:prepend
  save PATCH  /api/tasks/{id} {done}  optimistic
  drop DELETE /api/tasks/{id}         optimistic`;

export const SAMPLES: Record<string, Sample> = {
  /* ------------------------------------------------------------- elements */
  "elements.containers": {
    code: `card sm center
  h Clicks
  row center
    btn Decrement ghost >count--
    btn Increment primary >count++`,
    scaffold: `state count=0`,
  },

  "elements.title": {
    code: `card "Ship in minutes" | Describe the page, get a deployable build.`,
  },

  "elements.text": {
    code: `head Tasks — {tasks.open.count} open
metric {count}
empty Nothing here yet.`,
    scaffold: `state count=0\n${TASKS}`,
  },

  "elements.repeater": {
    code: `list tasks where={filter}
  check {done} >tasks.save
  text {title} strike={done}
  btn Delete quiet aria="Delete {title}" >tasks.drop
  empty Nothing here yet.`,
    scaffold: `state filter=all|open|done\n${TASKS}`,
  },

  "elements.content": {
    code: `tier Pro $24/mo "For working developers" cta="Go Pro" /signup featured
  Unlimited projects
  Custom domains
  Email support

faq open=1
  Can I export the code? | Yes. Every build is plain source.
  Do I need a card to try it? | No. The free tier needs no payment details.`,
  },

  /* ------------------------------------------------------------ modifiers */
  "modifiers.intent": {
    code: `btn Increment primary
// emits: rounded-md px-4 py-2 text-sm font-medium transition-colors
//        bg-slate-900 text-white hover:bg-slate-800 disabled:opacity-40`,
  },

  /**
   * The modifier-state sample is deliberately absent. Its lines are illustrative fragments rather
   * than an interface: `list tasks loading` has no item template on purpose (`GUML0072`) and
   * `input email required` has no accessible name on purpose (`GUML0051`). Previewing it would show
   * two diagnostics that are artefacts of the excerpt, not of the language.
   */

  /* --------------------------------------------------------------- syntax */
  "syntax.prose": {
    code: `p Press the buttons to change the value.
h1 Build the interface, skip the boilerplate
head Tasks — {tasks.open.count} open

card "Ship in minutes" | Describe the page, get a deployable build.`,
    scaffold: TASKS,
  },

  /**
   * The action sample on the syntax page and the binding-reads sample on the bindings page stay
   * plain blocks. Both show row-scoped bindings — `check {done}`, `text {title} strike={done}` —
   * outside the repeater that would put those fields in scope, because the subject of those
   * paragraphs is the *form* of a binding. Compiled standalone they are `GUML0033`: the correct
   * answer, and the wrong thing to show under that paragraph.
   */

  /* ------------------------------------------------------- user components */
  "defs.stat": {
    code: `def stat label value
  card sm center
    h {label}
    metric {value}
    p Measured this quarter.

stat "Revenue" {revenue}
stat "Signups" {signups}`,
    scaffold: `state revenue=0
state signups=0`,
  },

  /* ------------------------------------------------------------- bindings */
  "bindings.mutations": {
    code: `form >tasks.add{title:draft}; draft=""
  input draft placeholder="Add a task…" aria="New task"
  btn Add primary disabled={!draft.trim()} busy="Adding…"`,
    scaffold: `state draft=""\n${TASKS}`,
  },
};
