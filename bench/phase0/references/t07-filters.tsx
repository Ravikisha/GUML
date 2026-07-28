import { useEffect, useMemo, useState } from "react";

const CHANNELS = ["all", "web", "ios", "android"] as const;
const COUNTRIES = ["all", "GB", "US", "DE", "IN"] as const;

type Channel = (typeof CHANNELS)[number];
type Country = (typeof COUNTRIES)[number];

type Event = {
  id: string;
  name: string;
  channel: Exclude<Channel, "all">;
  country: Exclude<Country, "all">;
  count: number;
};

const THRESHOLD = 100;

export default function EventFilters() {
  const [events, setEvents] = useState<Event[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [channel, setChannel] = useState<Channel>("all");
  const [country, setCountry] = useState<Country>("all");
  const [aboveThreshold, setAboveThreshold] = useState(false);

  // One fetch. Filtering is a client-side derivation, so changing a filter costs
  // nothing and cannot race.
  useEffect(() => {
    const controller = new AbortController();
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await fetch("/api/events", { signal: controller.signal });
        if (!res.ok) throw new Error(`Request failed: ${res.status}`);
        setEvents((await res.json()) as Event[]);
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

  const matches = useMemo(
    () =>
      events.filter(
        (e) =>
          (channel === "all" || e.channel === channel) &&
          (country === "all" || e.country === country) &&
          (!aboveThreshold || e.count > THRESHOLD),
      ),
    [events, channel, country, aboveThreshold],
  );

  const totalCount = useMemo(() => matches.reduce((sum, e) => sum + e.count, 0), [matches]);

  function reset() {
    setChannel("all");
    setCountry("all");
    setAboveThreshold(false);
  }

  return (
    <main className="mx-auto grid max-w-4xl gap-8 px-6 py-10 md:grid-cols-[16rem_1fr]">
      <aside className="h-fit rounded-xl border border-slate-200 p-5">
        <h2 className="font-medium text-slate-900">Filters</h2>

        <div className="mt-4">
          <p id="channel-label" className="text-xs uppercase tracking-wide text-slate-500">
            Channel
          </p>
          <div className="mt-2 flex flex-wrap gap-1" role="group" aria-labelledby="channel-label">
            {CHANNELS.map((c) => (
              <button
                key={c}
                type="button"
                aria-pressed={channel === c}
                onClick={() => setChannel(c)}
                className={
                  channel === c
                    ? "rounded-full bg-slate-900 px-3 py-1 text-xs text-white"
                    : "rounded-full px-3 py-1 text-xs text-slate-600 hover:bg-slate-100"
                }
              >
                {c}
              </button>
            ))}
          </div>
        </div>

        <div className="mt-5">
          <label htmlFor="country" className="text-xs uppercase tracking-wide text-slate-500">
            Country
          </label>
          <select
            id="country"
            value={country}
            onChange={(e) => setCountry(e.target.value as Country)}
            className="mt-2 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
          >
            {COUNTRIES.map((c) => (
              <option key={c} value={c}>
                {c === "all" ? "All countries" : c}
              </option>
            ))}
          </select>
        </div>

        <div className="mt-5 flex items-center gap-2">
          <input
            id="threshold"
            type="checkbox"
            checked={aboveThreshold}
            onChange={(e) => setAboveThreshold(e.target.checked)}
            className="size-4 rounded border-slate-300"
          />
          <label htmlFor="threshold" className="text-sm text-slate-700">
            Only above {THRESHOLD}
          </label>
        </div>

        <button
          type="button"
          onClick={reset}
          className="mt-6 rounded-full px-3 py-1.5 text-xs text-slate-600 hover:bg-slate-100"
        >
          Reset filters
        </button>
      </aside>

      <section>
        <div className="flex gap-8">
          <div>
            <p className="text-xs uppercase tracking-wide text-slate-500">Events</p>
            <p className="mt-1 text-3xl font-semibold tabular-nums text-slate-900">
              {loading ? "—" : matches.length}
            </p>
          </div>
          <div>
            <p className="text-xs uppercase tracking-wide text-slate-500">Total count</p>
            <p className="mt-1 text-3xl font-semibold tabular-nums text-slate-900">
              {loading ? "—" : totalCount.toLocaleString()}
            </p>
          </div>
        </div>

        {error && (
          <p role="alert" className="mt-6 rounded-lg bg-red-50 px-4 py-3 text-sm text-red-700">
            {error}
          </p>
        )}

        {loading ? (
          <div className="mt-6 space-y-2" aria-busy="true">
            {[0, 1, 2, 3, 4].map((i) => (
              <div key={i} className="h-10 animate-pulse rounded-lg bg-slate-100" />
            ))}
          </div>
        ) : matches.length === 0 ? (
          <p className="mt-6 text-sm text-slate-500">No events match these filters.</p>
        ) : (
          <ul className="mt-6 divide-y divide-slate-100">
            {matches.map((e) => (
              <li key={e.id} className="flex items-center justify-between py-2.5 text-sm">
                <span className="text-slate-900">{e.name}</span>
                <span className="text-slate-500">
                  {e.channel} · {e.country} ·{" "}
                  <span className="tabular-nums text-slate-700">{e.count}</span>
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
