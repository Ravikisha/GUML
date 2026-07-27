import { useEffect, useMemo, useState } from "react";

type Task = {
  id: string;
  title: string;
  done: boolean;
  createdAt: string;
};

type Filter = "all" | "open" | "done";

export default function TaskList() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await fetch("/api/tasks");
        if (!res.ok) throw new Error(`Request failed: ${res.status}`);
        const data: Task[] = await res.json();
        if (!cancelled) setTasks(data);
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Unknown error");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  const visible = useMemo(() => {
    if (filter === "open") return tasks.filter((t) => !t.done);
    if (filter === "done") return tasks.filter((t) => t.done);
    return tasks;
  }, [tasks, filter]);

  const remaining = tasks.filter((t) => !t.done).length;

  async function addTask(e: React.FormEvent) {
    e.preventDefault();
    const title = draft.trim();
    if (!title) return;
    setSaving(true);
    setError(null);
    try {
      const res = await fetch("/api/tasks", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title }),
      });
      if (!res.ok) throw new Error("Could not add task");
      const created: Task = await res.json();
      setTasks((prev) => [created, ...prev]);
      setDraft("");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setSaving(false);
    }
  }

  async function toggle(task: Task) {
    setTasks((prev) =>
      prev.map((t) => (t.id === task.id ? { ...t, done: !t.done } : t))
    );
    try {
      await fetch(`/api/tasks/${task.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ done: !task.done }),
      });
    } catch {
      setTasks((prev) =>
        prev.map((t) => (t.id === task.id ? { ...t, done: task.done } : t))
      );
      setError("Could not save change");
    }
  }

  async function remove(id: string) {
    const snapshot = tasks;
    setTasks((prev) => prev.filter((t) => t.id !== id));
    try {
      await fetch(`/api/tasks/${id}`, { method: "DELETE" });
    } catch {
      setTasks(snapshot);
      setError("Could not delete task");
    }
  }

  return (
    <main className="mx-auto max-w-2xl px-4 py-10">
      <header className="flex items-baseline justify-between">
        <h1 className="text-2xl font-semibold text-slate-900">Tasks</h1>
        <span className="text-sm text-slate-500">{remaining} open</span>
      </header>

      <form onSubmit={addTask} className="mt-6 flex gap-2">
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="Add a task…"
          className="flex-1 rounded-md border border-slate-300 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-slate-900"
        />
        <button
          type="submit"
          disabled={saving || !draft.trim()}
          className="rounded-md bg-slate-900 px-4 py-2 text-sm font-medium text-white disabled:opacity-40"
        >
          {saving ? "Adding…" : "Add"}
        </button>
      </form>

      <div className="mt-4 flex gap-2">
        {(["all", "open", "done"] as Filter[]).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={
              filter === f
                ? "rounded-full bg-slate-900 px-3 py-1 text-xs font-medium text-white"
                : "rounded-full border border-slate-300 px-3 py-1 text-xs text-slate-600"
            }
          >
            {f}
          </button>
        ))}
      </div>

      {error && (
        <p role="alert" className="mt-4 rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">
          {error}
        </p>
      )}

      {loading ? (
        <ul className="mt-6 space-y-2">
          {[0, 1, 2].map((i) => (
            <li key={i} className="h-12 animate-pulse rounded-md bg-slate-100" />
          ))}
        </ul>
      ) : visible.length === 0 ? (
        <p className="mt-10 text-center text-sm text-slate-500">Nothing here yet.</p>
      ) : (
        <ul className="mt-6 divide-y divide-slate-200 rounded-md border border-slate-200">
          {visible.map((task) => (
            <li key={task.id} className="flex items-center gap-3 px-3 py-3">
              <input
                type="checkbox"
                checked={task.done}
                onChange={() => toggle(task)}
                className="h-4 w-4 rounded border-slate-300"
              />
              <span
                className={
                  task.done
                    ? "flex-1 text-sm text-slate-400 line-through"
                    : "flex-1 text-sm text-slate-900"
                }
              >
                {task.title}
              </span>
              <button
                onClick={() => remove(task.id)}
                aria-label={`Delete ${task.title}`}
                className="text-xs text-slate-400 hover:text-red-600"
              >
                Delete
              </button>
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
