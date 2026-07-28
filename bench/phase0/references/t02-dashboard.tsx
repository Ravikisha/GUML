import { useEffect, useMemo, useState } from "react";

type Priority = "low" | "normal" | "urgent";

type Ticket = {
  id: string;
  subject: string;
  requester: string;
  priority: Priority;
  minutesOpen: number;
  resolved: boolean;
};

const PRIORITIES: Array<Priority | "all"> = ["all", "urgent", "normal", "low"];

function median(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid];
}

function age(minutes: number): string {
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return hours < 24 ? `${hours}h` : `${Math.floor(hours / 24)}d`;
}

export default function SupportDashboard() {
  const [tickets, setTickets] = useState<Ticket[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [priority, setPriority] = useState<Priority | "all">("all");

  useEffect(() => {
    const controller = new AbortController();
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await fetch("/api/tickets", { signal: controller.signal });
        if (!res.ok) throw new Error(`Request failed: ${res.status}`);
        setTickets((await res.json()) as Ticket[]);
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

  const open = useMemo(() => tickets.filter((t) => !t.resolved), [tickets]);
  const urgent = useMemo(() => open.filter((t) => t.priority === "urgent"), [open]);
  const resolved = useMemo(() => tickets.filter((t) => t.resolved), [tickets]);
  const medianAge = useMemo(() => median(open.map((t) => t.minutesOpen)), [open]);

  const rows = useMemo(() => {
    const scoped = priority === "all" ? tickets : tickets.filter((t) => t.priority === priority);
    return [...scoped].sort((a, b) => b.minutesOpen - a.minutesOpen);
  }, [tickets, priority]);

  async function resolve(ticket: Ticket) {
    const snapshot = tickets;
    setTickets((prev) => prev.map((t) => (t.id === ticket.id ? { ...t, resolved: true } : t)));
    try {
      const res = await fetch(`/api/tickets/${ticket.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ resolved: true }),
      });
      if (!res.ok) throw new Error(`Request failed: ${res.status}`);
    } catch (e) {
      setTickets(snapshot);
      setError(e instanceof Error ? e.message : "Could not resolve the ticket");
    }
  }

  const kpis = [
    { label: "Open", value: open.length },
    { label: "Urgent", value: urgent.length },
    { label: "Resolved", value: resolved.length },
    { label: "Median age", value: age(Math.round(medianAge)) },
  ];

  return (
    <main className="mx-auto max-w-5xl px-6 py-10">
      <h1 className="text-2xl font-semibold text-slate-900">Support</h1>

      {error && (
        <p role="alert" className="mt-4 rounded-lg bg-red-50 px-4 py-3 text-sm text-red-700">
          {error}
        </p>
      )}

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {kpis.map((kpi) => (
          <div key={kpi.label} className="rounded-xl border border-slate-200 p-4">
            <p className="text-xs uppercase tracking-wide text-slate-500">{kpi.label}</p>
            <p className="mt-1 text-2xl font-semibold tabular-nums text-slate-900">
              {loading ? "—" : kpi.value}
            </p>
          </div>
        ))}
      </div>

      <div className="mt-8 flex gap-1" role="group" aria-label="Filter by priority">
        {PRIORITIES.map((p) => (
          <button
            key={p}
            type="button"
            aria-pressed={priority === p}
            onClick={() => setPriority(p)}
            className={
              priority === p
                ? "rounded-full bg-slate-900 px-3 py-1.5 text-sm text-white"
                : "rounded-full px-3 py-1.5 text-sm text-slate-600 hover:bg-slate-100"
            }
          >
            {p}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="mt-6 space-y-2" aria-busy="true">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="h-11 animate-pulse rounded-lg bg-slate-100" />
          ))}
        </div>
      ) : rows.length === 0 ? (
        <p className="mt-6 text-sm text-slate-500">No tickets match this filter.</p>
      ) : (
        <table className="mt-6 w-full text-left text-sm">
          <thead>
            <tr className="border-b border-slate-200 text-xs uppercase tracking-wide text-slate-500">
              <th scope="col" className="py-2 font-medium">
                Subject
              </th>
              <th scope="col" className="py-2 font-medium">
                Requester
              </th>
              <th scope="col" className="py-2 font-medium">
                Priority
              </th>
              <th scope="col" className="py-2 font-medium">
                Age
              </th>
              <th scope="col" className="py-2 font-medium">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {rows.map((t) => (
              <tr key={t.id} className="border-b border-slate-100 last:border-0">
                <td className="py-3 text-slate-900">{t.subject}</td>
                <td className="py-3 text-slate-600">{t.requester}</td>
                <td className="py-3 text-slate-600">{t.priority}</td>
                <td className="py-3 tabular-nums text-slate-600">{age(t.minutesOpen)}</td>
                <td className="py-3 text-right">
                  {t.resolved ? (
                    <span className="text-xs text-slate-400">resolved</span>
                  ) : (
                    <button
                      type="button"
                      onClick={() => resolve(t)}
                      aria-label={`Resolve ${t.subject}`}
                      className="rounded-full px-3 py-1 text-xs text-slate-600 hover:bg-slate-100"
                    >
                      Resolve
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </main>
  );
}
