import { useState } from "react";

const features = [
  {
    title: "Ship in minutes",
    body: "Describe the page, get a deployable build. No boilerplate to review.",
  },
  {
    title: "Own your output",
    body: "Every project exports as plain framework code you can keep editing.",
  },
  {
    title: "Accessible by default",
    body: "Focus states, labels and contrast are handled before you ask.",
  },
];

const tiers = [
  {
    name: "Hobby",
    price: "$0",
    blurb: "For side projects",
    perks: ["3 projects", "Community support", "Deploy previews"],
    cta: "Start free",
    featured: false,
  },
  {
    name: "Pro",
    price: "$24",
    blurb: "For working developers",
    perks: ["Unlimited projects", "Custom domains", "Email support"],
    cta: "Go Pro",
    featured: true,
  },
  {
    name: "Team",
    price: "$96",
    blurb: "For small teams",
    perks: ["Shared workspaces", "Roles and permissions", "Priority support"],
    cta: "Contact sales",
    featured: false,
  },
];

const faqs = [
  {
    q: "Can I export the code?",
    a: "Yes. Every build is plain source you can download or push to a repository.",
  },
  {
    q: "Do I need a credit card to try it?",
    a: "No. The Hobby tier is free forever and needs no payment details.",
  },
  {
    q: "What frameworks are supported?",
    a: "React, Svelte and standards-based web components today, with more planned.",
  },
];

export default function Landing() {
  const [open, setOpen] = useState<number | null>(0);

  return (
    <div className="min-h-screen bg-white text-slate-900">
      <nav className="mx-auto flex max-w-6xl items-center justify-between px-6 py-5">
        <span className="text-lg font-semibold">Northwind</span>
        <div className="hidden gap-8 text-sm text-slate-600 md:flex">
          <a href="#features" className="hover:text-slate-900">Features</a>
          <a href="#pricing" className="hover:text-slate-900">Pricing</a>
          <a href="#faq" className="hover:text-slate-900">FAQ</a>
        </div>
        <a
          href="/signup"
          className="rounded-md bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800"
        >
          Get started
        </a>
      </nav>

      <header className="mx-auto max-w-3xl px-6 py-24 text-center">
        <h1 className="text-4xl font-semibold tracking-tight sm:text-5xl">
          Build the interface, skip the boilerplate
        </h1>
        <p className="mx-auto mt-5 max-w-xl text-lg text-slate-600">
          Northwind turns a short description into a working, accessible web app you
          actually own.
        </p>
        <div className="mt-8 flex items-center justify-center gap-3">
          <a
            href="/signup"
            className="rounded-md bg-slate-900 px-5 py-2.5 text-sm font-medium text-white hover:bg-slate-800"
          >
            Start free
          </a>
          <a
            href="/demo"
            className="rounded-md border border-slate-300 px-5 py-2.5 text-sm font-medium text-slate-700 hover:bg-slate-50"
          >
            Watch demo
          </a>
        </div>
      </header>

      <section id="features" className="mx-auto max-w-6xl px-6 py-16">
        <div className="grid gap-6 md:grid-cols-3">
          {features.map((f) => (
            <div key={f.title} className="rounded-xl border border-slate-200 p-6">
              <h3 className="font-medium">{f.title}</h3>
              <p className="mt-2 text-sm text-slate-600">{f.body}</p>
            </div>
          ))}
        </div>
      </section>

      <section id="pricing" className="mx-auto max-w-6xl px-6 py-16">
        <h2 className="text-center text-2xl font-semibold">Pricing</h2>
        <div className="mt-10 grid gap-6 md:grid-cols-3">
          {tiers.map((t) => (
            <div
              key={t.name}
              className={
                t.featured
                  ? "rounded-xl border-2 border-slate-900 p-6 shadow-sm"
                  : "rounded-xl border border-slate-200 p-6"
              }
            >
              <h3 className="font-medium">{t.name}</h3>
              <p className="mt-1 text-sm text-slate-500">{t.blurb}</p>
              <p className="mt-4 text-3xl font-semibold">
                {t.price}
                <span className="text-sm font-normal text-slate-500">/mo</span>
              </p>
              <ul className="mt-6 space-y-2 text-sm text-slate-600">
                {t.perks.map((p) => (
                  <li key={p}>• {p}</li>
                ))}
              </ul>
              <a
                href="/signup"
                className={
                  t.featured
                    ? "mt-6 block rounded-md bg-slate-900 px-4 py-2 text-center text-sm font-medium text-white"
                    : "mt-6 block rounded-md border border-slate-300 px-4 py-2 text-center text-sm font-medium text-slate-700"
                }
              >
                {t.cta}
              </a>
            </div>
          ))}
        </div>
      </section>

      <section id="faq" className="mx-auto max-w-3xl px-6 py-16">
        <h2 className="text-2xl font-semibold">Questions</h2>
        <div className="mt-8 divide-y divide-slate-200 border-y border-slate-200">
          {faqs.map((f, i) => (
            <div key={f.q}>
              <button
                onClick={() => setOpen(open === i ? null : i)}
                aria-expanded={open === i}
                className="flex w-full items-center justify-between py-4 text-left text-sm font-medium"
              >
                {f.q}
                <span className="text-slate-400">{open === i ? "−" : "+"}</span>
              </button>
              {open === i && <p className="pb-4 text-sm text-slate-600">{f.a}</p>}
            </div>
          ))}
        </div>
      </section>

      <footer className="border-t border-slate-200">
        <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-3 px-6 py-8 text-sm text-slate-500 sm:flex-row">
          <span>© 2026 Northwind Labs</span>
          <div className="flex gap-6">
            <a href="/privacy" className="hover:text-slate-900">Privacy</a>
            <a href="/terms" className="hover:text-slate-900">Terms</a>
          </div>
        </div>
      </footer>
    </div>
  );
}
