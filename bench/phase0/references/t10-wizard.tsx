import { useState } from "react";

const SIZES = ["1-10", "11-50", "51+"] as const;
type Size = (typeof SIZES)[number];

const STEPS = ["Account", "Workspace", "Invite"] as const;

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

type Answers = {
  name: string;
  email: string;
  workspace: string;
  size: Size;
  invites: [string, string, string];
  sendNow: boolean;
};

export default function OnboardingWizard() {
  // One answers object across all three steps: per-step state is what loses input
  // when someone goes back.
  const [answers, setAnswers] = useState<Answers>({
    name: "",
    email: "",
    workspace: "",
    size: "1-10",
    invites: ["", "", ""],
    sendNow: true,
  });
  const [step, setStep] = useState(0);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  function set<K extends keyof Answers>(key: K, value: Answers[K]) {
    setAnswers((prev) => ({ ...prev, [key]: value }));
  }

  function setInvite(index: number, value: string) {
    setAnswers((prev) => {
      const next: [string, string, string] = [...prev.invites];
      next[index] = value;
      return { ...prev, invites: next };
    });
  }

  const stepValid = [
    answers.name.trim() !== "" && EMAIL.test(answers.email.trim()),
    answers.workspace.trim() !== "",
    answers.invites.every((v) => v.trim() === "" || EMAIL.test(v.trim())),
  ][step];

  async function submit() {
    if (pending) return;
    setPending(true);
    setError(null);
    try {
      const res = await fetch("/api/onboarding", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          ...answers,
          invites: answers.invites.map((v) => v.trim()).filter(Boolean),
        }),
      });
      if (!res.ok) throw new Error(`Request failed: ${res.status}`);
      setDone(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not finish setting up");
    } finally {
      setPending(false);
    }
  }

  if (done) {
    return (
      <main className="mx-auto max-w-lg px-6 py-16 text-center">
        <h1 className="text-2xl font-semibold text-slate-900">You are set up</h1>
        <p className="mt-3 text-slate-600">
          {answers.workspace} is ready
          {answers.sendNow ? " and the invitations are on their way." : "."}
        </p>
      </main>
    );
  }

  return (
    <main className="mx-auto max-w-lg px-6 py-12">
      <ol className="flex gap-2" aria-label="Progress">
        {STEPS.map((label, i) => (
          <li
            key={label}
            aria-current={i === step ? "step" : undefined}
            className={
              i === step
                ? "grow border-t-2 border-slate-900 pt-2 text-xs font-medium text-slate-900"
                : "grow border-t-2 border-slate-200 pt-2 text-xs text-slate-400"
            }
          >
            {label}
          </li>
        ))}
      </ol>
      <p className="mt-4 text-sm text-slate-500">
        Step {step + 1} of {STEPS.length}
      </p>

      {error && (
        <p role="alert" className="mt-4 rounded-lg bg-red-50 px-4 py-3 text-sm text-red-700">
          {error}
        </p>
      )}

      <form
        className="mt-6 rounded-xl border border-slate-200 p-6"
        onSubmit={(e) => {
          e.preventDefault();
          if (!stepValid) return;
          if (step < STEPS.length - 1) setStep(step + 1);
          else submit();
        }}
      >
        {step === 0 && (
          <div className="space-y-4">
            <h2 className="font-medium text-slate-900">Your account</h2>
            <div>
              <label htmlFor="name" className="block text-sm text-slate-700">
                Full name
              </label>
              <input
                id="name"
                value={answers.name}
                onChange={(e) => set("name", e.target.value)}
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
              />
            </div>
            <div>
              <label htmlFor="email" className="block text-sm text-slate-700">
                Email address
              </label>
              <input
                id="email"
                type="email"
                value={answers.email}
                onChange={(e) => set("email", e.target.value)}
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
              />
            </div>
          </div>
        )}

        {step === 1 && (
          <div className="space-y-4">
            <h2 className="font-medium text-slate-900">Your workspace</h2>
            <div>
              <label htmlFor="workspace" className="block text-sm text-slate-700">
                Workspace name
              </label>
              <input
                id="workspace"
                value={answers.workspace}
                onChange={(e) => set("workspace", e.target.value)}
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
              />
            </div>
            <div>
              <label htmlFor="size" className="block text-sm text-slate-700">
                Team size
              </label>
              <select
                id="size"
                value={answers.size}
                onChange={(e) => set("size", e.target.value as Size)}
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
              >
                {SIZES.map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </select>
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="space-y-4">
            <h2 className="font-medium text-slate-900">Invite colleagues</h2>
            <p className="text-sm text-slate-500">Up to three, and you can skip this.</p>
            {answers.invites.map((value, i) => (
              <div key={i}>
                <label htmlFor={`invite-${i}`} className="block text-sm text-slate-700">
                  Colleague {i + 1}
                </label>
                <input
                  id={`invite-${i}`}
                  type="email"
                  value={value}
                  onChange={(e) => setInvite(i, e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
                />
              </div>
            ))}
            <div className="flex items-center justify-between">
              <span id="sendNow-label" className="text-sm text-slate-700">
                Send invitations now
              </span>
              <button
                type="button"
                role="switch"
                aria-checked={answers.sendNow}
                aria-labelledby="sendNow-label"
                onClick={() => set("sendNow", !answers.sendNow)}
                className={
                  answers.sendNow
                    ? "h-6 w-11 rounded-full bg-slate-900 p-0.5 text-left"
                    : "h-6 w-11 rounded-full bg-slate-300 p-0.5 text-right"
                }
              >
                <span className="block h-5 w-5 rounded-full bg-white" />
              </button>
            </div>
          </div>
        )}

        <div className="mt-6 flex items-center justify-between">
          <button
            type="button"
            onClick={() => setStep(Math.max(0, step - 1))}
            disabled={step === 0}
            className="rounded-full px-4 py-2 text-sm text-slate-600 disabled:opacity-40"
          >
            Back
          </button>
          <button
            type="submit"
            disabled={!stepValid || pending}
            className="rounded-full bg-slate-900 px-5 py-2 text-sm text-white disabled:opacity-40"
          >
            {step < STEPS.length - 1 ? "Next" : pending ? "Finishing…" : "Finish"}
          </button>
        </div>
      </form>
    </main>
  );
}
