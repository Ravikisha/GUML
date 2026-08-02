/**
 * GUML-Bench tasks. **12 of the 150 the report specifies.**
 *
 * That number is stated first because it is the most important thing about this file. The report's dataset
 * is 6 categories × 25, and this is two per category — enough to exercise the harness end to end and to
 * catch a category where the compression story is different from expected, and *not* enough to publish a
 * per-category figure from. `guml-bench report` prints the coverage next to every number for that reason,
 * and `preflight.mjs` refuses to claim otherwise.
 *
 * # How a task is written
 *
 * One prompt, shared by every arm. If the React arm were asked for less than the GUML arm the comparison
 * would be rigged in a way no aggregate can reveal, so `schema.validateTask` rejects any task that tries
 * to vary the prompt per arm.
 *
 * The `checklist` is the scoring instrument. One line, one point, scored 1 / 0.5 / 0 by a human who does
 * not know which arm produced the output. It is written to be answerable by *looking at the running app* —
 * "creating a row rolls back if the request fails" is checkable; "uses good state management" is not.
 *
 * `tags` drives the registry slice for the GUML arms. Slicing is itself an independent variable: a slice
 * containing exactly the tags the answer needs is a hint the other arms do not get, which is why
 * `--full-registry` exists and why the run records which was used.
 *
 * # `reference` — an authored GUML answer for every task
 *
 * Every task has one, and `preflight` fails if the file is missing. Three point at repository fixtures so
 * their numbers connect to the published token counts; the rest live in `bench/guml-bench/reference/`.
 *
 * Two things they are for, and the second matters more.
 *
 * **They make `report` measurable.** Compile rate, emitted size, the accessibility guarantees, the
 * escape-hatch rate — all of it needs an artifact, and with three of twelve authored the report was
 * measuring a quarter of its own dataset and saying so.
 *
 * **Writing them is the expressiveness test.** Nine documents produced ten defects in the compiler, and
 * every one was a *silent* fault that fixtures had not reached: a two-step aggregate that emitted
 * `invoices.paid.amount.reduce(…)`, a state interpolated into a request URL that reached `fetch` with its
 * braces intact, `>channel = all` emitting `setChannel(all)`, a numeric field assigning a string, a
 * single-object resource typed as an array, `badge danger X` rendering the word "danger". A fixture is
 * written to exercise a feature someone already thought of; a *whole task answer* is written to be correct,
 * and it goes wherever the task goes.
 *
 * Each file's header names what it could not express, and those notes are the vocabulary evidence: a
 * repeater over a derived array, an aggregate over an expression, options projected from fetched rows.
 */

export const TASKS = [
  /* ------------------------------------------------------------------ landing */
  {
    id: "l01-saas-landing",
    reference: "fixtures/c.guml",
    title: "SaaS landing page with pricing",
    category: "landing",
    // Reused from `fixtures/c.guml` so this result connects to the report's authored token numbers.
    fixture: "c",
    tags: "nav,hero,section,card,tier,faq,footer,btn,link,h1,h2,p,grid",
    prompt: `Build a marketing page for a developer tool called Northwind.

It needs: a top navigation bar with the product name and links to Features, Pricing
and FAQ plus a "Get started" call to action; an above-the-fold headline with one
paragraph of subcopy and two calls to action; a features section with three cards;
a pricing section with three tiers where the middle one is highlighted and each
lists its perks and price; an FAQ of five question-and-answer pairs where the first
is open; and a footer with a copyright line.

Write real marketing copy, not placeholder text.`,
    checklist: [
      "Navigation bar with the product name and three section links",
      "Each navigation link scrolls to a section that exists on the page",
      "Above-the-fold headline, subcopy and two calls to action",
      "Exactly one h1 on the page",
      "Three feature cards, each with a heading and a description",
      "Three pricing tiers, each with a name, a price and at least two perks",
      "The middle tier is visually distinguished",
      "Each tier has its own call to action",
      "Five FAQ pairs, expandable, the first one open",
      "Footer with a copyright line",
      "Copy is specific to the product rather than lorem ipsum",
      "Renders with no JavaScript at all",
    ],
  },
  {
    id: "l02-changelog",
    reference: "bench/guml-bench/reference/l02-changelog.guml",
    title: "Changelog page",
    category: "landing",
    tags: "nav,section,card,h1,h2,p,badge,note,divider,footer,link",
    prompt: `Build a changelog page for a product called Meridian.

Six releases, newest first. Each entry shows a version number, a date, a one-line
summary, a short paragraph of detail, and a badge saying whether it is a feature, a
fix or a breaking change. Group them under month headings. Include a header with
the product name and a link back to the docs, and a footer.

Write plausible release notes for a database tool, not placeholders.`,
    checklist: [
      "Six release entries",
      "Newest release appears first",
      "Each entry has a version, a date and a summary",
      "Each entry has a badge classifying the change",
      "Breaking-change badges are visually distinct from feature badges",
      "Entries are grouped under month headings",
      "Header with the product name and a docs link",
      "Footer present",
      "Exactly one h1",
      "Release notes are specific to a database product",
      "Renders with no JavaScript at all",
    ],
  },

  /* ---------------------------------------------------------------- dashboard */
  {
    id: "d01-ops-console",
    reference: "fixtures/e.guml",
    title: "Operations console",
    category: "dashboard",
    fixture: "e",
    tags: "toolbar,grid,stat,alert,table,tabs,badge,metric,breadcrumb,btn,select,text,empty",
    prompt: `Build an operations console for a job-running service.

GET /api/jobs returns Job objects with id, name, owner and failed. POST
/api/jobs/{id}/retry retries one.

The page shows: a breadcrumb trail; a toolbar with a refresh button and a live
badge; a banner when any job has failed; three KPI tiles for total jobs, failing
jobs and jobs retried today; a table of jobs with the owner and a retry button per
row; and a filter that narrows the table by severity.

Retrying should feel instant and roll back if the request fails.`,
    checklist: [
      "Fetches jobs on mount",
      "Loading state while the first fetch is in flight",
      "Error state if the fetch fails",
      "Three KPI tiles, each with a label and a value",
      "At least one KPI is derived from the fetched rows rather than hardcoded",
      "Table with one row per job showing name and owner",
      "Retry button on each row",
      "Retry applies optimistically and rolls back on failure",
      "Retry targets the row it was clicked on",
      "Severity filter narrows the visible rows",
      "Empty state when no jobs match",
      "Banner shown only when a job has failed",
      "Every control has a visible or programmatic label",
      "Refresh re-fetches without a full page reload",
    ],
  },
  {
    id: "d02-revenue",
    reference: "bench/guml-bench/reference/d02-revenue.guml",
    title: "Revenue dashboard",
    category: "dashboard",
    tags: "grid,stat,card,metric,table,tabs,text,empty,h1,note,progress",
    prompt: `Build a revenue dashboard.

GET /api/invoices returns Invoice objects with id, client, amount, paid and due.

Show: four KPI tiles for total billed, total collected, outstanding, and collection
rate as a percentage; a period selector for this month / this quarter / this year; a
table of invoices with client, amount, due date and paid status; and a progress bar
for the collection rate. Every number must be derived from the fetched rows.`,
    checklist: [
      "Fetches invoices on mount",
      "Loading and error states for the fetch",
      "Four KPI tiles",
      "Total billed is the sum of all amounts",
      "Total collected is the sum of paid amounts",
      "Outstanding is billed minus collected",
      "Collection rate is a percentage, not a raw count",
      "Progress bar reflects the collection rate",
      "Period selector with three options",
      "Table with client, amount, due date and paid status per row",
      "Empty state when there are no invoices",
      "No number on the page is hardcoded",
      "Every control has a label",
    ],
  },

  /* --------------------------------------------------------------------- crud */
  {
    id: "c01-tasks",
    reference: "fixtures/b.guml",
    title: "Task list with optimistic updates",
    category: "crud",
    fixture: "b",
    tags: "head,form,input,btn,tabs,list,check,text,empty",
    prompt: `Build a single-page task list backed by a REST API.

GET /api/tasks returns Task objects with id, title, done and createdAt. POST
/api/tasks creates one from a title. PATCH /api/tasks/{id} updates done. DELETE
/api/tasks/{id} removes one.

Someone must be able to add a task from a text field, tick it off, delete it, and
filter between all / open / done. Show a count of open tasks in the header.
Creating, ticking and deleting should feel instant and must roll back if the
request fails.`,
    checklist: [
      "Fetches the list on mount",
      "Loading state while the first fetch is in flight",
      "Error state if the fetch fails",
      "Add from a text field",
      "Add button disabled while the field is empty",
      "New task appears before the request completes",
      "A failed create removes the optimistic row",
      "Tick a task off, applied optimistically",
      "Delete a task, applied optimistically",
      "Filter between all / open / done",
      "Open count in the header, derived from the rows",
      "Empty state when nothing matches the filter",
      "Every control has a visible or programmatic label",
      "The text field clears after a successful add",
    ],
  },
  {
    id: "c02-contacts",
    reference: "bench/guml-bench/reference/c02-contacts.guml",
    title: "Contacts with search and edit",
    category: "crud",
    tags: "head,toolbar,input,btn,table,text,empty,modal,form,select,badge",
    prompt: `Build a contacts manager.

GET /api/contacts returns Contact objects with id, name, email, company and tier
(free, pro, enterprise). PATCH /api/contacts/{id} updates one. DELETE
/api/contacts/{id} removes one.

Show a searchable table of contacts with a tier badge per row. Searching filters
client-side over the fetched rows. Clicking a row opens a dialog to edit the name
and email, with Save and Cancel. Deleting asks for nothing but must roll back if it
fails.`,
    checklist: [
      "Fetches contacts on mount",
      "Loading and error states for the fetch",
      "Table with name, email, company and tier per row",
      "Tier rendered as a badge, visually distinct per tier",
      "Search field filters the visible rows",
      "Searching does not refetch",
      "Empty state when the search matches nothing",
      "A dialog opens for editing and is not present until opened",
      "The dialog has an accessible name",
      "Save applies optimistically and rolls back on failure",
      "Cancel closes the dialog without saving",
      "Delete applies optimistically and rolls back on failure",
      "Every control has a label",
    ],
  },

  /* ---------------------------------------------------------------- ecommerce */
  {
    id: "e01-cart",
    reference: "bench/guml-bench/reference/e01-cart.guml",
    title: "Shopping cart",
    category: "ecommerce",
    tags: "head,table,text,btn,input,metric,empty,stat,note,divider,alert",
    prompt: `Build a shopping cart page.

GET /api/cart returns CartLine objects with id, product, variant, unitPrice and
quantity. PATCH /api/cart/{id} updates quantity. DELETE /api/cart/{id} removes a
line.

Show each line with its product, variant, unit price, a quantity control and a line
total. Below: a subtotal, a fixed £4.95 shipping charge waived above £50, and an
order total. Changing a quantity or removing a line must update every total
immediately and roll back if the request fails. Show an empty-cart state.`,
    checklist: [
      "Fetches the cart on mount",
      "Loading and error states for the fetch",
      "Each line shows product, variant, unit price and quantity",
      "Line total is unit price times quantity",
      "Subtotal is the sum of the line totals",
      "Shipping is £4.95 below £50 and waived at or above it",
      "Order total is subtotal plus shipping",
      "Changing a quantity updates every total",
      "Quantity change applies optimistically and rolls back on failure",
      "Removing a line applies optimistically and rolls back on failure",
      "Empty-cart state when there are no lines",
      "Quantity controls have accessible names identifying their line",
    ],
  },
  {
    id: "e02-product",
    reference: "bench/guml-bench/reference/e02-product.guml",
    title: "Product page with variants",
    category: "ecommerce",
    tags: "section,h1,p,img,tabs,select,btn,badge,stat,faq,note,alert",
    prompt: `Build a product page for a pair of headphones.

GET /api/product/hp-1 returns a Product with id, name, description, price, and a
list of variants each with a colour, a size and a stock count. POST /api/cart adds
a variant to the basket.

Show the product image, name, price and description; a colour selector; a size
selector; a stock indicator that reflects the selected variant; an Add to basket
button disabled when the selection is out of stock; and an FAQ of three pairs about
shipping and returns. Write real product copy.`,
    checklist: [
      "Fetches the product on mount",
      "Loading and error states for the fetch",
      "Product image with alternative text",
      "Name, price and description shown",
      "Colour selector with the available colours",
      "Size selector with the available sizes",
      "Stock indicator reflects the selected colour and size together",
      "Add to basket disabled when the selection is out of stock",
      "Add to basket posts the selected variant, not the first one",
      "Three FAQ pairs",
      "Product copy is specific to headphones",
      "Every control has a label",
    ],
  },

  /* --------------------------------------------------------------------- saas */
  {
    id: "s01-team-settings",
    reference: "bench/guml-bench/reference/s01-team-settings.guml",
    title: "Team settings with roles",
    category: "saas",
    tags: "sidebar,section,h1,h2,table,select,btn,badge,avatar,toggle,note,modal,alert,empty",
    prompt: `Build a team settings screen.

GET /api/members returns Member objects with id, name, email, role (owner, admin,
member) and active. PATCH /api/members/{id} updates role or active. DELETE
/api/members/{id} removes a member.

Show a settings sidebar; a members table with an avatar, name, email, a role
selector and an active toggle per row; a Remove action per row that asks for
confirmation in a dialog; and a banner when the team has no admin. An owner's role
must not be editable.`,
    checklist: [
      "Fetches members on mount",
      "Loading and error states for the fetch",
      "Sidebar with settings navigation",
      "Table with avatar, name, email, role and active state per row",
      "Role selector offers the three roles",
      "An owner's role selector is disabled",
      "Active toggle applies optimistically and rolls back on failure",
      "Role change applies optimistically and rolls back on failure",
      "Remove opens a confirmation dialog",
      "The dialog has an accessible name",
      "Removing applies only after confirmation",
      "Banner shown only when no member has the admin role",
      "Every control has an accessible name identifying its row",
    ],
  },
  {
    id: "s02-billing",
    reference: "bench/guml-bench/reference/s02-billing.guml",
    title: "Billing and plan",
    category: "saas",
    tags: "section,h1,h2,grid,tier,stat,progress,table,btn,badge,note,alert,divider",
    prompt: `Build a billing screen.

GET /api/subscription returns a Subscription with plan, seats, seatsUsed,
renewsOn and amount. GET /api/invoices returns past invoices with id, date, amount
and status. POST /api/subscription/plan changes the plan.

Show the current plan with its price and renewal date; seat usage as a progress bar
with a warning above 90%; three plans to switch to with the current one marked; and
a table of past invoices with a status badge. Changing plan should feel instant.`,
    checklist: [
      "Fetches the subscription and the invoices",
      "Loading and error states for both fetches",
      "Current plan, price and renewal date shown",
      "Seat usage shown as a progress bar",
      "Seat usage warning appears only above 90%",
      "Three plans offered",
      "The current plan is visually marked and not offered as a switch target",
      "Changing plan applies optimistically and rolls back on failure",
      "Invoice table with date, amount and status",
      "Status rendered as a badge, visually distinct per status",
      "Empty state when there are no past invoices",
      "Every control has a label",
    ],
  },

  /* ------------------------------------------------------------------ dataviz */
  {
    id: "v01-event-filters",
    reference: "bench/guml-bench/reference/v01-event-filters.guml",
    title: "Event filter panel",
    category: "dataviz",
    tags: "row,col,card,select,check,tabs,btn,metric,stat,text,list,empty",
    prompt: `Build a filter panel beside a result count for an analytics view.

GET /api/events returns Event objects with id, name, channel (web, ios, android),
country (GB, US, DE, IN) and count.

The panel filters by channel as a segmented control, country as a select, and an
"only above 100" checkbox. Show the number of matching events, their summed count,
a list of the matches, and a Reset that returns every filter to its default.

Filtering happens client-side over the fetched rows — one fetch, not one per
change.`,
    checklist: [
      "Fetches events once, on mount",
      "Filtering does not refetch",
      "Channel segmented control with the three values plus an all option",
      "Country select with the four values plus an all option",
      "Threshold checkbox filters to count above 100",
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
    id: "v02-cohort",
    reference: "bench/guml-bench/reference/v02-cohort.guml",
    title: "Cohort explorer",
    category: "dataviz",
    tags: "toolbar,select,tabs,grid,stat,table,text,empty,metric,note,skeleton",
    prompt: `Build a cohort explorer.

GET /api/signups returns Signup objects with id, cohort (a month), plan (free,
pro), country and retained (a boolean).

Show a toolbar with a cohort selector and a plan segmented control; four stat tiles
for cohort size, retained, churned and retention rate; and a table of the selected
cohort's signups. Every figure is derived from the fetched rows and recomputed when
a filter changes, without refetching.`,
    checklist: [
      "Fetches signups once, on mount",
      "Changing a filter does not refetch",
      "Cohort selector populated from the fetched rows",
      "Plan segmented control with both plans plus an all option",
      "Four stat tiles",
      "Cohort size counts only the selected cohort",
      "Retained and churned sum to the cohort size",
      "Retention rate is a percentage, not a count",
      "Table shows only the selected cohort's rows",
      "Empty state when a cohort has no rows",
      "Loading state while the first fetch is in flight",
      "Every control has a label",
    ],
  },
];
