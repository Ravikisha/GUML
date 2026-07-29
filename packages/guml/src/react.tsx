"use client";

/**
 * React runtime for GUML.
 *
 * Renders the compiler's own render tree, so the markup and classes you see match
 * what `guml build` would have written — the preview cannot drift from the code.
 *
 * Nothing is evaluated with `eval`: bindings go through the small expression
 * evaluator in `./eval`, and actions lower to a fixed set of effects. That is what
 * makes it defensible to render a document produced by an untrusted agent.
 */

import {
  createElement,
  useCallback,
  useEffect,
  useMemo,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { tree as compileTree, type Diagnostic, type UiNode, type UiTree } from "./index.js";
import { evaluate, interpolate, runAction, truthy, type Scope } from "./eval.js";

export type GumlProps = {
  /** GUML source. Recompiled when it changes. */
  source: string;
  /**
   * Seed data for `data` resources, keyed by resource name. Without this a
   * resource is fetched from its declared URL.
   */
  data?: Record<string, unknown[]>;
  /** Prefix for resource URLs, e.g. an API origin. */
  baseUrl?: string;
  /** Override how a tag renders. Receives the node and its rendered children. */
  components?: Partial<Record<string, (node: UiNode, children: ReactNode) => ReactNode>>;
  className?: string;
  style?: CSSProperties;
  /** Called after each compile, with the diagnostics the compiler produced. */
  onDiagnostics?: (diagnostics: Diagnostic[]) => void;
  /** Rendered while the wasm compiler loads. */
  fallback?: ReactNode;
};

type Status = "loading" | "ready" | "invalid";

/**
 * Compile GUML and render it.
 *
 * ```tsx
 * <Guml source={"page Hi\nstate n=0\n\nbtn Add primary >n++"} />
 * ```
 */
export function Guml({
  source,
  data,
  baseUrl,
  components,
  className,
  style,
  onDiagnostics,
  fallback = null,
}: GumlProps) {
  const { tree, diagnostics, status } = useGumlTree(source);
  const view = useGumlRuntime(tree, { data, baseUrl });

  useEffect(() => {
    if (status !== "loading") onDiagnostics?.(diagnostics);
  }, [status, diagnostics, onDiagnostics]);

  if (status === "loading") return <>{fallback}</>;
  if (status === "invalid" || !tree) return null;

  return (
    <div className={className} style={style}>
      {tree.nodes.map((node, i) => (
        <Node key={i} node={node} view={view} components={components} />
      ))}
    </div>
  );
}

/** Compile a source string to a render tree, recompiling as it changes. */
export function useGumlTree(source: string) {
  const [state, setState] = useState<{
    tree: UiTree | null;
    diagnostics: Diagnostic[];
    status: Status;
  }>({ tree: null, diagnostics: [], status: "loading" });

  useEffect(() => {
    let cancelled = false;
    compileTree(source)
      .then((res) => {
        if (cancelled) return;
        setState({
          tree: res.tree,
          diagnostics: res.diagnostics,
          status: res.ok ? "ready" : "invalid",
        });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setState({
          tree: null,
          status: "invalid",
          diagnostics: [
            {
              id: "GUML9000",
              code: "host_error",
              severity: "error",
              message: err instanceof Error ? err.message : String(err),
              span: { start: 0, end: 0, line: 1, col: 1 },
            },
          ],
        });
      });
    return () => {
      cancelled = true;
    };
  }, [source]);

  return state;
}

type Runtime = {
  scope: Scope;
  dispatch: (action: string, local?: Scope) => void;
  set: (name: string, value: unknown) => void;
  pending: Set<string>;
};

/**
 * State, resources and action dispatch for a tree. Exposed so a host can build
 * its own renderer while keeping GUML's semantics.
 */
export function useGumlRuntime(
  tree: UiTree | null,
  opts: { data?: Record<string, unknown[]>; baseUrl?: string } = {},
): Runtime {
  const { data, baseUrl = "" } = opts;

  const initial = useMemo(() => {
    const scope: Scope = {};
    for (const s of tree?.state ?? []) scope[s.name] = s.init;
    for (const r of tree?.resources ?? []) scope[r.name] = data?.[r.name] ?? [];
    return scope;
  }, [tree, data]);

  const [scope, setScope] = useState<Scope>(initial);
  const [pending, setPending] = useState<Set<string>>(new Set());

  // Reset when the program itself changes — a new tree is a new program.
  useEffect(() => setScope(initial), [initial]);

  const set = useCallback((name: string, value: unknown) => {
    setScope((prev) => ({ ...prev, [name]: value }));
  }, []);

  const dispatch = useCallback(
    (action: string, local?: Scope) => {
      const merged = local ? { ...scope, ...local } : scope;
      let effects;
      try {
        effects = runAction(action, merged);
      } catch {
        return; // unsupported action: the compiler already reported it
      }

      for (const effect of effects) {
        if (effect.kind === "set") {
          set(effect.name, effect.value);
          continue;
        }

        const resource = tree?.resources.find((r) => r.name === effect.resource);
        const mutation = resource?.mutations.find((m) => m.name === effect.mutation);
        if (!resource || !mutation) continue;

        // Optimistic apply first, then the request — the rollback path is the
        // reason `optimistic:` exists in the language.
        const before = (scope[resource.name] as unknown[]) ?? [];
        const item = local?.__item as Record<string, unknown> | undefined;
        const next = applyOptimistic(before, mutation, effect.body, item);
        set(resource.name, next);

        if (!mutation.url || typeof fetch === "undefined") continue;

        const url = baseUrl + fillPath(mutation.url, item);
        const key = `${resource.name}.${mutation.name}`;
        setPending((p) => new Set(p).add(key));
        void fetch(url, {
          method: mutation.method,
          headers: { "Content-Type": "application/json" },
          body: mutation.method === "GET" ? undefined : JSON.stringify(effect.body),
        })
          .then((res) => {
            if (!res.ok) throw new Error(String(res.status));
          })
          .catch(() => set(resource.name, before))
          .finally(() =>
            setPending((p) => {
              const n = new Set(p);
              n.delete(key);
              return n;
            }),
          );
      }
    },
    [scope, set, tree, baseUrl],
  );

  // Resources with no seeded data fetch themselves on mount.
  useEffect(() => {
    if (!tree || typeof fetch === "undefined") return;
    let cancelled = false;
    for (const r of tree.resources) {
      if (data?.[r.name] || !r.url) continue;
      void fetch(baseUrl + r.url)
        .then((res) => (res.ok ? res.json() : []))
        .then((rows) => {
          if (!cancelled && Array.isArray(rows)) set(r.name, rows);
        })
        .catch(() => {});
    }
    return () => {
      cancelled = true;
    };
  }, [tree, data, baseUrl, set]);

  return { scope, dispatch, set, pending };
}

function applyOptimistic(
  rows: unknown[],
  mutation: { name: string; optimistic: string | null },
  body: Record<string, unknown>,
  item?: Record<string, unknown>,
): unknown[] {
  switch (mutation.optimistic) {
    case "prepend":
      return [{ id: `tmp-${Date.now()}`, ...body }, ...rows];
    case "append":
      return [...rows, { id: `tmp-${Date.now()}`, ...body }];
    case "replace":
      if (!item) return rows;
      // A body-less mutation on an item is a delete; otherwise merge the body.
      return Object.keys(body).length === 0 && mutation.name !== "save"
        ? rows.filter((r) => r !== item)
        : rows.map((r) => (r === item ? { ...item, ...invert(body, item) } : r));
    default:
      return rows;
  }
}

/** `{done}` with no value toggles the field, which is what a row checkbox means. */
function invert(body: Record<string, unknown>, item: Record<string, unknown>) {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(body)) {
    out[k] = v === undefined || v === k ? !item[k] : v;
  }
  if (Object.keys(out).length === 0) out.done = !item.done;
  return out;
}

function fillPath(url: string, item?: Record<string, unknown>) {
  return url.replace(/\{(\w+)\}/g, (_, k: string) => String(item?.[k] ?? ""));
}

// ---------------------------------------------------------------- node renderer

function Node({
  node,
  view,
  components,
  local,
}: {
  node: UiNode;
  view: Runtime;
  components?: GumlProps["components"];
  local?: Scope;
}) {
  const scope = local ? { ...view.scope, ...local } : view.scope;

  // Repeater: render its children once per row.
  if (node.source) {
    const rows = (scope[node.source] as unknown[]) ?? [];
    const filtered = filterRows(rows, node.filter, scope);
    const empty = node.children.find((c) => c.tag === "empty");
    const template = node.children.filter((c) => c.tag !== "empty");

    if (filtered.length === 0 && empty) {
      return <Node node={empty} view={view} components={components} local={local} />;
    }

    return (
      <ul className="divide-y divide-slate-200 rounded-md border border-slate-200">
        {filtered.map((row, i) => (
          <li key={i} className="flex items-center gap-3 px-3 py-3">
            {template.map((child, j) => (
              <Node
                key={j}
                node={child}
                view={view}
                components={components}
                local={{ ...(row as Scope), __item: row }}
              />
            ))}
          </li>
        ))}
      </ul>
    );
  }

  const children: ReactNode[] = node.children.map((child, i) => (
    <Node key={i} node={child} view={view} components={components} local={local} />
  ));

  const override = components?.[node.tag];
  if (override) return <>{override(node, children)}</>;

  // A tag the compiler cannot lower yet: say so rather than render nothing.
  if (!node.el) {
    return (
      <div className="rounded-md border border-dashed border-amber-400/50 bg-amber-400/5 px-3 py-2 font-mono text-xs text-amber-600">
        {node.tag} — not lowered by this compiler version
      </div>
    );
  }

  const props: Record<string, unknown> = { className: node.class || undefined };

  for (const p of node.props) {
    const value = p.bound ? safe(() => evaluate(String(p.value), scope)) : p.value;
    if (p.name === "placeholder" || p.name === "aria-label" || p.name === "href" || p.name === "id" || p.name === "type") {
      props[p.name] = typeof value === "string" ? interpolate(value, scope) : value;
    } else if (p.name === "disabled" || p.name === "checked" || p.name === "required" || p.name === "readOnly") {
      props[p.name] = truthy(value);
    } else if (p.name === "strike") {
      props.className = `${node.class} ${truthy(value) ? "line-through text-slate-400" : ""}`.trim();
    } else if (p.name !== "busy" && p.name !== "where" && p.name !== "cta" && p.name !== "open") {
      props[p.name] = value;
    }
  }

  // Accessible name inherited from the row, matching what the analyser accepted.
  if (node.ariaFrom && !props["aria-label"]) {
    props["aria-label"] = String(scope[node.ariaFrom] ?? "");
  }

  if (node.actions.length) {
    const handler =
      node.tag === "check" || node.tag === "toggle"
        ? "onChange"
        : node.tag === "form"
          ? "onSubmit"
          : "onClick";
    props[handler] = (e: { preventDefault?: () => void }) => {
      e.preventDefault?.();
      view.dispatch(node.actions[0], local);
    };
  }

  // Two-way binding for fields.
  if ((node.tag === "input" || node.tag === "select") && node.bind) {
    props.value = String(scope[node.bind] ?? "");
    props.onChange = (e: { target: { value: string } }) => view.set(node.bind!, e.target.value);
  }
  if (node.tag === "check" && node.bind) {
    props.checked = truthy(scope[node.bind]);
  }

  if (node.el === "input") return createElement("input", props);

  const text = node.text ?? (node.bind && !node.children.length ? `{${node.bind}}` : node.label);
  const content: ReactNode[] = [];
  if (text) content.push(interpolate(text, scope));
  if (node.lines.length) {
    content.push(
      ...node.lines.map((line, i) => (
        <div key={`l${i}`} className="text-sm text-slate-600">
          {interpolate(line.split("|")[0].trim(), scope)}
        </div>
      )),
    );
  }
  content.push(...children);

  return createElement(node.el, props, ...content);
}

function filterRows(rows: unknown[], filter: string | null, scope: Scope): unknown[] {
  if (!filter) return rows;
  const value = String(scope[filter] ?? filter);
  if (value === "open") return rows.filter((r) => !(r as Scope)?.done);
  if (value === "done") return rows.filter((r) => Boolean((r as Scope)?.done));
  return rows;
}

function safe<T>(fn: () => T): T | undefined {
  try {
    return fn();
  } catch {
    return undefined;
  }
}
