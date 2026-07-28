import { useState } from "react";

type Period = "monthly" | "yearly";

/** Yearly is two months free, so a year costs ten months. */
const YEARLY_MONTHS = 10;

const TIERS = [
  {
    name: "Starter",
    monthly: 12,
    blurb: "For one developer shipping side projects.",
    features: ["3 projects", "Community support", "Deploy previews", "1 GB build cache"],
    cta: "Start free",
    popular: false,
  },
  {
    name: "Growth",
    monthly: 40,
    blurb: "For a small team with something in production.",
    features: ["Unlimited projects", "Custom domains", "Email support", "10 GB build cache"],
    cta: "Choose Growth",
    popular: true,
  },
  {
    name: "Scale",
    monthly: 120,
    blurb: "For teams with compliance and uptime obligations.",
    features: ["SSO and SCIM", "Audit log", "Priority support", "100 GB build cache"],
    cta: "Talk to sales",
    popular: false,
  },
];

const FAQ = [
  {
    q: "When am I charged?",
    a: "On the day you subscribe, and on the same day each period after that. Yearly plans are charged once, up front.",
  },
  {
    q: "Can I switch period later?",
    a: "Yes. Switching to yearly credits the unused part of the current month; switching back takes effect at the end of the year you paid for.",
  },
  {
    q: "What happens if I go over the cache limit?",
    a: "Builds keep working. The oldest cache entries are evicted first, so you lose build speed rather than availability.",
  },
];

export default function Pricing() {
  const [period, setPeriod] = useState<Period>("monthly");
  const [open, setOpen] = useState<number | null>(0);

  return (
    <main className="mx-auto max-w-5xl px-6 py-14">
      <h1 className="text-center text-4xl font-semibold tracking-tight text-slate-900">
        Pricing that follows your team, not your traffic
      </h1>
      <h2 className="mx-auto mt-4 max-w-xl text-center text-slate-600">
        Every plan includes the full compiler, unlimited builds and the same support response
        target. Pay yearly and two months are on us.
      </h2>

      <div
        role="group"
        aria-label="Billing period"
        className="mx-auto mt-8 flex w-fit gap-1 rounded-full border border-slate-200 p-1"
      >
        {(["monthly", "yearly"] as const).map((p) => (
          <button
            key={p}
            type="button"
            aria-pressed={period === p}
            onClick={() => setPeriod(p)}
            className={
              period === p
                ? "rounded-full bg-slate-900 px-4 py-1.5 text-sm text-white"
                : "rounded-full px-4 py-1.5 text-sm text-slate-600"
            }
          >
            {p === "monthly" ? "Monthly" : "Yearly · 2 months free"}
          </button>
        ))}
      </div>

      <div className="mt-10 grid gap-6 md:grid-cols-3">
        {TIERS.map((tier) => {
          const yearlyTotal = tier.monthly * YEARLY_MONTHS;
          const perMonth = period === "yearly" ? yearlyTotal / 12 : tier.monthly;
          return (
            <section
              key={tier.name}
              className={
                tier.popular
                  ? "relative rounded-2xl border-2 border-slate-900 p-6"
                  : "rounded-2xl border border-slate-200 p-6"
              }
            >
              {tier.popular && (
                <p className="absolute -top-3 left-6 rounded-full bg-slate-900 px-2.5 py-0.5 text-xs text-white">
                  Most popular
                </p>
              )}
              <h3 className="font-medium text-slate-900">{tier.name}</h3>
              <p className="mt-3">
                <span className="text-3xl font-semibold tabular-nums text-slate-900">
                  ${perMonth.toFixed(perMonth % 1 === 0 ? 0 : 2)}
                </span>
                <span className="text-sm text-slate-500">/mo</span>
              </p>
              <p className="mt-1 text-xs text-slate-500">
                {period === "yearly" ? `$${yearlyTotal} billed yearly` : "billed monthly"}
              </p>
              <p className="mt-3 text-sm text-slate-600">{tier.blurb}</p>
              <ul className="mt-5 space-y-2 text-sm text-slate-600">
                {tier.features.map((f) => (
                  <li key={f} className="flex gap-2">
                    <span aria-hidden className="mt-2 h-1 w-1 shrink-0 rounded-full bg-slate-400" />
                    {f}
                  </li>
                ))}
              </ul>
              <a
                href="/signup"
                className={
                  tier.popular
                    ? "mt-6 block rounded-full bg-slate-900 py-2.5 text-center text-sm text-white"
                    : "mt-6 block rounded-full border border-slate-300 py-2.5 text-center text-sm text-slate-900"
                }
              >
                {tier.cta}
              </a>
            </section>
          );
        })}
      </div>

      <section className="mt-16">
        <h2 className="text-xl font-semibold text-slate-900">Billing questions</h2>
        <div className="mt-4 divide-y divide-slate-200 border-y border-slate-200">
          {FAQ.map((item, i) => (
            <div key={item.q}>
              <h3>
                <button
                  type="button"
                  aria-expanded={open === i}
                  aria-controls={`faq-${i}`}
                  onClick={() => setOpen(open === i ? null : i)}
                  className="flex w-full items-center justify-between py-4 text-left text-sm text-slate-900"
                >
                  {item.q}
                  <span aria-hidden className="text-slate-400">
                    {open === i ? "−" : "+"}
                  </span>
                </button>
              </h3>
              {open === i && (
                <p id={`faq-${i}`} className="pb-4 text-sm leading-relaxed text-slate-600">
                  {item.a}
                </p>
              )}
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
