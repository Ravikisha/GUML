/**
 * Phase 0 task set — ten tasks across the six benchmark categories.
 *
 * Each task is the *same* prompt for both arms. The arm only changes the target
 * representation, never the requirements: if the React arm is asked for less, the
 * comparison is rigged, and that is the easiest way to fake this result.
 *
 * `checklist` is the scoring instrument, not documentation. One line = one point,
 * scored 1 (met) / 0.5 (partial) / 0 (absent) by a human who does not know which
 * arm produced the output. See ../rubric.md.
 *
 * `tags` drives the registry slice (`guml registry --tags …`). Slicing is itself a
 * variable: a slice that happens to contain exactly the tags the answer needs is a
 * hint. Record whether the slice was task-specific or full — `--full-registry`.
 */

/** @typedef {"structure"|"content"|"mixed"} Category */

export const TASKS = [
  {
    id: "t01-crud",
    title: "Task list with optimistic updates",
    category: "structure",
    // Reused from fixtures/ so the results connect to the report's token numbers.
    fixture: "b",
    reference: "t01-crud.tsx",
    tags: "head,form,input,btn,tabs,list,check,text,empty",
    prompt: `Build a single-page task list backed by a REST API.

The API: GET /api/tasks returns Task objects with id, title, done and createdAt.
POST /api/tasks creates one from a title. PATCH /api/tasks/{id} updates done.
DELETE /api/tasks/{id} removes one.

The page must let someone add a task from a text field, tick a task off, delete a
task, and filter between all / open / done. Show a count of open tasks in the
header. Creating, ticking and deleting should feel instant and must roll back if
the request fails.`,
    checklist: [
      "Fetches the list on mount",
      "Shows a loading state while the first fetch is in flight",
      "Shows an error state if the fetch fails",
      "Shows an empty state when the list is empty",
      "Add: posts the draft title and clears the field",
      "Add button is disabled while the draft is blank or whitespace",
      "Toggle: patches done for that task only",
      "Delete: removes that task only",
      "All three mutations apply optimistically",
      "All three mutations roll back the previous list on failure",
      "Filter switches between all / open / done",
      "Header shows a live count of open tasks",
      "Every control has an accessible name (delete identifies which task)",
      "List items are keyed by id, not index",
    ],
  },

  {
    id: "t02-dashboard",
    title: "Support dashboard",
    category: "structure",
    reference: "t02-dashboard.tsx",
    tags: "head,row,card,metric,tabs,table,text,empty",
    prompt: `Build a support dashboard page.

GET /api/tickets returns Ticket objects with id, subject, requester, priority
(one of low, normal, urgent), minutesOpen and resolved. PATCH
/api/tickets/{id} accepts { resolved }.

The page shows four KPI tiles across the top: open tickets, urgent tickets,
resolved today, and median minutes open. Below that, a table of tickets showing
subject, requester, priority and age, sorted oldest first, with a control to
resolve a ticket in place. A segmented control filters the table by priority.`,
    checklist: [
      "Fetches tickets on mount",
      "Loading, error and empty states all present",
      "Four KPI values are derived from the fetched data, not hard-coded",
      "Table rows show subject, requester, priority and age",
      "Table is sorted by age, oldest first",
      "Priority filter narrows the table",
      "Resolve control patches that ticket and updates the row",
      "Resolve is optimistic with rollback on failure",
      "KPI tiles update after a resolve",
      "Table has a header row associated with its columns",
      "Resolve control names which ticket it acts on",
      "Rows keyed by id",
    ],
  },

  {
    id: "t03-landing",
    title: "Product landing page",
    category: "content",
    fixture: "c",
    reference: "t03-landing.tsx",
    tags: "nav,hero,section,card,tier,faq,footer,btn,link",
    prompt: `Build a marketing landing page for Northwind, a tool that turns a short
description into a working, accessible web app the customer owns outright.

It needs: a top nav with links to Features, Pricing and FAQ plus a "Get started"
call to action; an above-the-fold headline with subcopy and two calls to action
(start free, watch demo); three feature cards; three pricing tiers (Hobby $0/mo,
Pro $24/mo highlighted, Team $96/mo) each with three perks and a call to action;
an FAQ with three question-and-answer pairs where the first is open; and a footer
with a copyright line and links to Privacy and Terms.

Write the copy yourself. It is a static page — no API.`,
    checklist: [
      "Nav with brand, three anchor links and a CTA",
      "Anchor links point at ids that exist on the page",
      "Hero has one h1, subcopy and two CTAs",
      "Three feature cards with distinct copy",
      "Three pricing tiers with the stated names and prices",
      "Pro tier is visually distinguished",
      "Each tier has three perks and its own CTA",
      "FAQ has three pairs and the first is open by default",
      "FAQ is keyboard-operable and announces expanded state",
      "Footer has the copyright line and both links",
      "Exactly one h1 on the page",
      "Copy is specific to the product, not lorem placeholder",
    ],
  },

  {
    id: "t04-docs",
    title: "Documentation article",
    category: "content",
    reference: "t04-docs.tsx",
    tags: "section,h,h2,p,card,link,footer",
    prompt: `Build a documentation page explaining HTTP caching headers to a working
web developer.

It needs an on-page title and one-paragraph introduction, then four sections:
Cache-Control, ETag and conditional requests, Vary, and a short troubleshooting
list. Each section needs a heading and at least two paragraphs of real
explanation. Include a callout warning that no-cache does not mean no store.
End with links to the two relevant RFCs.

Write the prose yourself — it must be technically correct, not placeholder.`,
    checklist: [
      "Title plus an introductory paragraph",
      "Four sections with headings in the stated order",
      "Each section has at least two paragraphs",
      "Cache-Control section names at least three real directives",
      "ETag section explains If-None-Match and 304",
      "Vary section explains why it affects cache keys",
      "Troubleshooting section is a list, not prose",
      "Callout distinguishes no-cache from no-store",
      "Two RFC links, both real URLs",
      "Heading levels are nested, not skipped",
      "Prose is technically accurate",
      "No placeholder or filler text",
    ],
  },

  {
    id: "t05-settings",
    title: "Account settings screen",
    category: "mixed",
    reference: "t05-settings.tsx",
    tags: "section,card,form,input,select,toggle,btn,p",
    prompt: `Build an account settings screen.

GET /api/settings returns display name, email, timezone (one of UTC, Europe/London,
America/New_York), a weekly-digest flag and a product-updates flag. PATCH
/api/settings accepts any subset of those.

Group it into two cards: Profile (name, email, timezone) and Notifications (the two
flags). Profile changes are saved with an explicit Save button that is disabled
until something changes; the notification switches save immediately. Show a
confirmation after a successful save and an error if it fails.`,
    checklist: [
      "Loads current settings on mount",
      "Loading and error states for the initial load",
      "Two grouped sections: profile and notifications",
      "Name and email are editable text fields",
      "Timezone is a select limited to the three values",
      "Save is disabled until a field actually changes",
      "Save patches only the profile fields",
      "Switches patch immediately, without a Save press",
      "Switch state reflects the server value after load",
      "Success confirmation shown after a save",
      "Failed save shows an error and does not silently keep the value",
      "Every field and switch has a label, not just a placeholder",
    ],
  },

  {
    id: "t06-checkout",
    title: "Checkout step: shipping details",
    category: "mixed",
    reference: "t06-checkout.tsx",
    tags: "section,card,form,input,select,check,btn,text,p",
    prompt: `Build the shipping step of a checkout flow.

The form collects full name, address line 1, address line 2 (optional), city,
postcode and country (one of GB, US, DE). A checkbox offers "billing address is the
same". A summary card on the side shows three fixed line items, a subtotal, the
shipping cost, and a total.

Shipping is £4.99 unless the subtotal is over £50, in which case it is free — and
the summary must say which applies. Continue submits to POST /api/checkout/shipping
and is disabled until every required field is filled.`,
    checklist: [
      "All six address fields present, with line 2 marked optional",
      "Country is a select limited to the three values",
      "Billing-same checkbox present and bound to state",
      "Summary lists the three line items with prices",
      "Subtotal is computed from the line items, not hard-coded",
      "Shipping is £4.99 below the threshold and free above it",
      "Summary states which shipping rule applied",
      "Total = subtotal + shipping, and updates with shipping",
      "Continue disabled until every required field is non-empty",
      "Submit posts the collected address",
      "Submit shows a pending state and cannot double-fire",
      "Every field has a label associated with it",
    ],
  },

  {
    id: "t07-filters",
    title: "Data-viz filter panel",
    category: "mixed",
    reference: "t07-filters.tsx",
    tags: "row,col,card,select,check,tabs,btn,metric,text,list,empty",
    prompt: `Build a filter panel beside a result count for an analytics view.

GET /api/events returns Event objects with id, name, channel (web, ios, android),
country (GB, US, DE, IN) and count. The panel filters by channel (a segmented
control), country (a select), and a "only above 100" checkbox. It shows the number
of matching events and their summed count, a list of the matches, and a Reset that
returns every filter to its default.

Filtering happens client-side over the fetched rows — one fetch, not one per
change.`,
    checklist: [
      "Fetches events once, on mount",
      "Filtering does not refetch",
      "Channel segmented control with the three values plus an all option",
      "Country select with the four values plus an all option",
      "Threshold checkbox filters to count > 100",
      "Filters compose — all three apply together",
      "Match count reflects the active filters",
      "Summed count reflects the active filters",
      "Empty state shown when nothing matches",
      "Reset returns all three filters to their defaults",
      "Each control has a visible or programmatic label",
      "Loading and error states for the fetch",
    ],
  },

  {
    id: "t08-team",
    title: "Team management",
    category: "mixed",
    reference: "t08-team.tsx",
    tags: "head,form,input,select,table,text,btn,empty",
    prompt: `Build a team management page.

GET /api/members returns Member objects with id, name, email and role (owner,
admin, member). POST /api/members invites one from an email and a role. PATCH
/api/members/{id} changes the role. DELETE /api/members/{id} removes one.

The page shows a table of members with their name, email and role, where role is
editable in place. Above it, an invite form takes an email and a role. Show the
member count in the header. An owner cannot be removed or demoted.`,
    checklist: [
      "Loads members on mount, with loading and error states",
      "Table shows name, email and role per member",
      "Role is editable in place and patches that member",
      "Invite form takes an email and a role",
      "Invite posts and adds the member to the table",
      "Invite is rejected or disabled for an empty or malformed email",
      "Remove deletes that member only",
      "Owner row cannot be removed",
      "Owner role cannot be changed",
      "Mutations are optimistic and roll back on failure",
      "Header shows the member count",
      "Remove and role controls name which member they act on",
    ],
  },

  {
    id: "t09-pricing",
    title: "Pricing page with billing period",
    category: "mixed",
    reference: "t09-pricing.tsx",
    tags: "section,h1,p,tabs,tier,faq,btn",
    prompt: `Build a pricing page with a monthly / yearly switch.

Three tiers: Starter $12/mo, Growth $40/mo highlighted as most popular, and Scale
$120/mo. Yearly billing is two months free, so the yearly price is ten times the
monthly one — show it as a per-month figure with the yearly total underneath. Each
tier lists four features and has a call to action. Below the tiers, three FAQ
entries about billing.

Write the copy yourself.`,
    checklist: [
      "Segmented monthly / yearly control",
      "Three tiers with the stated names and monthly prices",
      "Yearly prices are ten months' worth, shown per month",
      "Yearly total shown alongside the per-month figure",
      "Switching the period updates all three tiers",
      "Growth tier is marked most popular and visually distinguished",
      "Each tier lists four features",
      "Each tier has its own call to action",
      "Three billing FAQ entries",
      "The active period is programmatically indicated, not only coloured",
      "One h1 and a supporting subheading",
      "Copy is specific, not placeholder",
    ],
  },

  {
    id: "t10-wizard",
    title: "Three-step form wizard",
    category: "mixed",
    reference: "t10-wizard.tsx",
    tags: "section,card,tabs,form,input,select,toggle,btn,p,text",
    prompt: `Build a three-step onboarding wizard: Account, Workspace, Invite.

Step 1 collects name and email. Step 2 collects a workspace name and a size
(1-10, 11-50, 51+). Step 3 collects up to three colleague emails and a "send
invites now" switch. A progress indicator shows which of the three steps is
current. Back and Next move between steps; Next is blocked until the current step
is valid. The final step submits everything to POST /api/onboarding.

Answers must survive moving backwards and forwards.`,
    checklist: [
      "Three steps with the stated fields",
      "Progress indicator shows the current step of three",
      "Next is blocked while the current step is invalid",
      "Back returns to the previous step",
      "Values entered earlier survive going back and forward",
      "Step 1 requires a name and a plausible email",
      "Step 2 size is limited to the three values",
      "Step 3 accepts up to three emails",
      "Send-invites switch is bound to state",
      "Final step submits the whole payload once",
      "Submit shows a pending state and cannot double-fire",
      "Current step is announced programmatically, not only styled",
    ],
  },
];

/** Categories, for per-category reporting. A single mean across them is misleading. */
export const CATEGORIES = ["structure", "content", "mixed"];

export function taskById(id) {
  const task = TASKS.find((t) => t.id === id);
  if (!task) throw new Error(`unknown task: ${id}`);
  return task;
}
