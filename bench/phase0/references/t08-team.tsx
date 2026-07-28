import { useEffect, useState } from "react";

const ROLES = ["owner", "admin", "member"] as const;
type Role = (typeof ROLES)[number];

/** Owner is never offered in the pickers: it is transferred, not assigned. */
const ASSIGNABLE: Role[] = ["admin", "member"];

type Member = { id: string; name: string; email: string; role: Role };

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export default function TeamManagement() {
  const [members, setMembers] = useState<Member[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<Role>("member");
  const [inviting, setInviting] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await fetch("/api/members", { signal: controller.signal });
        if (!res.ok) throw new Error(`Request failed: ${res.status}`);
        setMembers((await res.json()) as Member[]);
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

  const validEmail = EMAIL.test(email.trim());

  async function invite() {
    if (!validEmail || inviting) return;
    setInviting(true);
    setError(null);
    const snapshot = members;
    const optimistic: Member = {
      id: `pending-${email.trim()}`,
      name: email.trim(),
      email: email.trim(),
      role,
    };
    setMembers((prev) => [...prev, optimistic]);
    setEmail("");
    try {
      const res = await fetch("/api/members", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: optimistic.email, role }),
      });
      if (!res.ok) throw new Error(`Request failed: ${res.status}`);
      const created = (await res.json()) as Member;
      setMembers((prev) => prev.map((m) => (m.id === optimistic.id ? created : m)));
    } catch (e) {
      setMembers(snapshot);
      setError(e instanceof Error ? e.message : "Could not send the invitation");
    } finally {
      setInviting(false);
    }
  }

  async function changeRole(member: Member, next: Role) {
    if (member.role === "owner") return;
    const snapshot = members;
    setMembers((prev) => prev.map((m) => (m.id === member.id ? { ...m, role: next } : m)));
    try {
      const res = await fetch(`/api/members/${member.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ role: next }),
      });
      if (!res.ok) throw new Error(`Request failed: ${res.status}`);
    } catch (e) {
      setMembers(snapshot);
      setError(e instanceof Error ? e.message : "Could not change that role");
    }
  }

  async function remove(member: Member) {
    if (member.role === "owner") return;
    const snapshot = members;
    setMembers((prev) => prev.filter((m) => m.id !== member.id));
    try {
      const res = await fetch(`/api/members/${member.id}`, { method: "DELETE" });
      if (!res.ok) throw new Error(`Request failed: ${res.status}`);
    } catch (e) {
      setMembers(snapshot);
      setError(e instanceof Error ? e.message : "Could not remove that member");
    }
  }

  return (
    <main className="mx-auto max-w-3xl px-6 py-10">
      <h1 className="text-2xl font-semibold text-slate-900">
        Team <span className="text-slate-400">· {loading ? "—" : members.length}</span>
      </h1>

      {error && (
        <p role="alert" className="mt-4 rounded-lg bg-red-50 px-4 py-3 text-sm text-red-700">
          {error}
        </p>
      )}

      <form
        className="mt-6 flex flex-wrap items-end gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          invite();
        }}
      >
        <div className="grow">
          <label htmlFor="invite-email" className="block text-sm text-slate-700">
            Invite by email
          </label>
          <input
            id="invite-email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="colleague@example.com"
            className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
          />
        </div>
        <div>
          <label htmlFor="invite-role" className="block text-sm text-slate-700">
            Role
          </label>
          <select
            id="invite-role"
            value={role}
            onChange={(e) => setRole(e.target.value as Role)}
            className="mt-1 rounded-lg border border-slate-300 px-3 py-2 text-sm"
          >
            {ASSIGNABLE.map((r) => (
              <option key={r} value={r}>
                {r}
              </option>
            ))}
          </select>
        </div>
        <button
          type="submit"
          disabled={!validEmail || inviting}
          className="rounded-full bg-slate-900 px-4 py-2 text-sm text-white disabled:opacity-40"
        >
          {inviting ? "Sending…" : "Send invite"}
        </button>
      </form>

      {loading ? (
        <div className="mt-8 space-y-2" aria-busy="true">
          {[0, 1, 2].map((i) => (
            <div key={i} className="h-12 animate-pulse rounded-lg bg-slate-100" />
          ))}
        </div>
      ) : members.length === 0 ? (
        <p className="mt-8 text-sm text-slate-500">No one here yet. Send the first invite.</p>
      ) : (
        <table className="mt-8 w-full text-left text-sm">
          <thead>
            <tr className="border-b border-slate-200 text-xs uppercase tracking-wide text-slate-500">
              <th scope="col" className="py-2 font-medium">
                Name
              </th>
              <th scope="col" className="py-2 font-medium">
                Email
              </th>
              <th scope="col" className="py-2 font-medium">
                Role
              </th>
              <th scope="col" className="py-2 font-medium">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {members.map((m) => (
              <tr key={m.id} className="border-b border-slate-100 last:border-0">
                <td className="py-3 text-slate-900">{m.name}</td>
                <td className="py-3 text-slate-600">{m.email}</td>
                <td className="py-3">
                  {m.role === "owner" ? (
                    <span className="text-slate-600">owner</span>
                  ) : (
                    <select
                      value={m.role}
                      onChange={(e) => changeRole(m, e.target.value as Role)}
                      aria-label={`Role for ${m.name}`}
                      className="rounded-lg border border-slate-300 px-2 py-1 text-sm"
                    >
                      {ASSIGNABLE.map((r) => (
                        <option key={r} value={r}>
                          {r}
                        </option>
                      ))}
                    </select>
                  )}
                </td>
                <td className="py-3 text-right">
                  {m.role === "owner" ? (
                    <span className="text-xs text-slate-400">—</span>
                  ) : (
                    <button
                      type="button"
                      onClick={() => remove(m)}
                      aria-label={`Remove ${m.name}`}
                      className="rounded-full px-3 py-1 text-xs text-slate-600 hover:bg-slate-100"
                    >
                      Remove
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
