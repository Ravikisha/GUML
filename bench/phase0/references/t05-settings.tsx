import { useEffect, useState } from "react";

const ZONES = ["UTC", "Europe/London", "America/New_York"] as const;
type Zone = (typeof ZONES)[number];

type Settings = {
  displayName: string;
  email: string;
  timezone: Zone;
  weeklyDigest: boolean;
  productUpdates: boolean;
};

type Profile = Pick<Settings, "displayName" | "email" | "timezone">;

async function patch(body: Partial<Settings>) {
  const res = await fetch("/api/settings", {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`Request failed: ${res.status}`);
}

export default function AccountSettings() {
  const [server, setServer] = useState<Settings | null>(null);
  const [draft, setDraft] = useState<Profile | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await fetch("/api/settings", { signal: controller.signal });
        if (!res.ok) throw new Error(`Request failed: ${res.status}`);
        const data = (await res.json()) as Settings;
        setServer(data);
        setDraft({ displayName: data.displayName, email: data.email, timezone: data.timezone });
      } catch (e) {
        if (!controller.signal.aborted) {
          setError(e instanceof Error ? e.message : "Unknown error");
        }
      } finally {
        if (!controller.signal.aborted) setLoading(false);
      }
    }
    load();
    return () => controller.abort();
  }, []);

  // Dirty is derived, not tracked: a tracked flag drifts the moment a field is
  // edited back to its original value.
  const dirty =
    server !== null &&
    draft !== null &&
    (draft.displayName !== server.displayName ||
      draft.email !== server.email ||
      draft.timezone !== server.timezone);

  async function saveProfile() {
    if (!draft || !dirty) return;
    setSaving(true);
    setError(null);
    setSaved(false);
    try {
      await patch(draft);
      setServer((prev) => (prev ? { ...prev, ...draft } : prev));
      setSaved(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not save your profile");
    } finally {
      setSaving(false);
    }
  }

  async function toggleFlag(key: "weeklyDigest" | "productUpdates") {
    if (!server) return;
    const next = !server[key];
    const snapshot = server;
    setServer({ ...server, [key]: next });
    setError(null);
    try {
      await patch({ [key]: next });
    } catch (e) {
      setServer(snapshot);
      setError(e instanceof Error ? e.message : "Could not update that preference");
    }
  }

  if (loading) {
    return (
      <main className="mx-auto max-w-2xl px-6 py-10" aria-busy="true">
        <div className="h-8 w-40 animate-pulse rounded bg-slate-100" />
        <div className="mt-6 h-48 animate-pulse rounded-xl bg-slate-100" />
      </main>
    );
  }

  if (error && !server) {
    return (
      <main className="mx-auto max-w-2xl px-6 py-10">
        <p role="alert" className="rounded-lg bg-red-50 px-4 py-3 text-sm text-red-700">
          {error}
        </p>
      </main>
    );
  }

  if (!server || !draft) return null;

  return (
    <main className="mx-auto max-w-2xl px-6 py-10">
      <h1 className="text-2xl font-semibold text-slate-900">Settings</h1>

      {error && (
        <p role="alert" className="mt-4 rounded-lg bg-red-50 px-4 py-3 text-sm text-red-700">
          {error}
        </p>
      )}

      <section className="mt-6 rounded-xl border border-slate-200 p-6">
        <h2 className="font-medium text-slate-900">Profile</h2>
        <form
          className="mt-4 space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            saveProfile();
          }}
        >
          <div>
            <label htmlFor="displayName" className="block text-sm text-slate-700">
              Display name
            </label>
            <input
              id="displayName"
              value={draft.displayName}
              onChange={(e) => setDraft({ ...draft, displayName: e.target.value })}
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
              value={draft.email}
              onChange={(e) => setDraft({ ...draft, email: e.target.value })}
              className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
            />
          </div>
          <div>
            <label htmlFor="timezone" className="block text-sm text-slate-700">
              Timezone
            </label>
            <select
              id="timezone"
              value={draft.timezone}
              onChange={(e) => setDraft({ ...draft, timezone: e.target.value as Zone })}
              className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
            >
              {ZONES.map((zone) => (
                <option key={zone} value={zone}>
                  {zone}
                </option>
              ))}
            </select>
          </div>
          <div className="flex items-center gap-3">
            <button
              type="submit"
              disabled={!dirty || saving}
              className="rounded-full bg-slate-900 px-4 py-2 text-sm text-white disabled:opacity-40"
            >
              {saving ? "Saving…" : "Save changes"}
            </button>
            {saved && !dirty && (
              <p role="status" className="text-sm text-emerald-700">
                Profile saved.
              </p>
            )}
          </div>
        </form>
      </section>

      <section className="mt-6 rounded-xl border border-slate-200 p-6">
        <h2 className="font-medium text-slate-900">Notifications</h2>
        <p className="mt-1 text-sm text-slate-500">These save as soon as you change them.</p>
        <div className="mt-4 space-y-3">
          {(
            [
              ["weeklyDigest", "Weekly digest"],
              ["productUpdates", "Product updates"],
            ] as const
          ).map(([key, label]) => (
            <div key={key} className="flex items-center justify-between">
              <span id={`${key}-label`} className="text-sm text-slate-700">
                {label}
              </span>
              <button
                type="button"
                role="switch"
                aria-checked={server[key]}
                aria-labelledby={`${key}-label`}
                onClick={() => toggleFlag(key)}
                className={
                  server[key]
                    ? "h-6 w-11 rounded-full bg-slate-900 p-0.5 text-left"
                    : "h-6 w-11 rounded-full bg-slate-300 p-0.5 text-right"
                }
              >
                <span className="block h-5 w-5 rounded-full bg-white" />
              </button>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
