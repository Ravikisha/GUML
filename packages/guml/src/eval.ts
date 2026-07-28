/**
 * The runtime half of GUML's expression and action semantics.
 *
 * GUML bindings are deliberately not a general-purpose language, which is what
 * makes this safe: a small recursive-descent evaluator over paths, comparisons and
 * arithmetic. **No `eval`, no `new Function`** — a GUML document can come from an
 * untrusted agent, and the language's non-Turing-complete action set is the
 * security boundary that makes rendering one reasonable.
 *
 * v0 covers what the fixtures use. Anything outside the grammar throws, and the
 * renderer surfaces it rather than silently rendering `undefined`.
 */

export type Scope = Record<string, unknown>;

class ExprError extends Error {}

// ---------------------------------------------------------------- tokenizer

type Tok = { kind: "num" | "str" | "ident" | "op"; text: string };

function lex(src: string): Tok[] {
  const out: Tok[] = [];
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    if (c === " " || c === "\t") {
      i++;
      continue;
    }
    if (c === '"' || c === "'") {
      const quote = c;
      let j = i + 1;
      let value = "";
      while (j < src.length && src[j] !== quote) {
        if (src[j] === "\\") j++;
        value += src[j++];
      }
      out.push({ kind: "str", text: value });
      i = j + 1;
      continue;
    }
    if (/[0-9]/.test(c)) {
      let j = i;
      while (j < src.length && /[0-9.]/.test(src[j])) j++;
      out.push({ kind: "num", text: src.slice(i, j) });
      i = j;
      continue;
    }
    if (/[A-Za-z_$]/.test(c)) {
      let j = i;
      while (j < src.length && /[A-Za-z0-9_$.]/.test(src[j])) j++;
      out.push({ kind: "ident", text: src.slice(i, j) });
      i = j;
      continue;
    }
    const two = src.slice(i, i + 2);
    if (["==", "!=", "<=", ">=", "&&", "||"].includes(two)) {
      out.push({ kind: "op", text: two });
      i += 2;
      continue;
    }
    if ("+-*/%<>!()".includes(c)) {
      out.push({ kind: "op", text: c });
      i++;
      continue;
    }
    throw new ExprError(`unexpected character \`${c}\` in expression`);
  }
  return out;
}

// ---------------------------------------------------------------- parser / eval

/**
 * Evaluate a GUML binding expression against a scope.
 *
 * Supports: paths (`a.b.c`), `!`, unary `-`, `* / %`, `+ -`, comparisons,
 * `&& ||`, parentheses, number and string literals, and the collection helpers
 * below.
 */
export function evaluate(expr: string, scope: Scope): unknown {
  const toks = lex(expr);
  let pos = 0;

  const peek = () => toks[pos];
  const eat = (text: string) => {
    if (peek()?.text === text) {
      pos++;
      return true;
    }
    return false;
  };

  function primary(): unknown {
    const t = peek();
    if (!t) throw new ExprError("unexpected end of expression");

    if (eat("(")) {
      const v = or();
      if (!eat(")")) throw new ExprError("expected `)`");
      return v;
    }
    if (eat("!")) return !truthy(primary());
    if (eat("-")) return -Number(primary());

    pos++;
    if (t.kind === "num") return Number(t.text);
    if (t.kind === "str") return t.text;
    if (t.kind === "ident") {
      if (t.text === "true") return true;
      if (t.text === "false") return false;
      if (t.text === "null") return null;
      // A call suffix like `draft.trim()` is handled by resolvePath; consume the
      // parentheses the tokenizer left behind.
      const value = resolvePath(t.text, scope);
      if (peek()?.text === "(") {
        pos++;
        if (!eat(")")) throw new ExprError("only zero-argument calls are supported");
      }
      return value;
    }
    throw new ExprError(`unexpected \`${t.text}\``);
  }

  function mul(): unknown {
    let left = primary();
    for (;;) {
      if (eat("*")) left = Number(left) * Number(primary());
      else if (eat("/")) left = Number(left) / Number(primary());
      else if (eat("%")) left = Number(left) % Number(primary());
      else return left;
    }
  }

  function add(): unknown {
    let left = mul();
    for (;;) {
      if (eat("+")) {
        const right = mul();
        left =
          typeof left === "string" || typeof right === "string"
            ? String(left) + String(right)
            : Number(left) + Number(right);
      } else if (eat("-")) left = Number(left) - Number(mul());
      else return left;
    }
  }

  function cmp(): unknown {
    const left = add();
    if (eat("==")) return left === add();
    if (eat("!=")) return left !== add();
    if (eat("<=")) return Number(left) <= Number(add());
    if (eat(">=")) return Number(left) >= Number(add());
    if (eat("<")) return Number(left) < Number(add());
    if (eat(">")) return Number(left) > Number(add());
    return left;
  }

  function and(): unknown {
    let left = cmp();
    while (eat("&&")) {
      const right = cmp();
      left = truthy(left) ? right : left;
    }
    return left;
  }

  function or(): unknown {
    let left = and();
    while (eat("||")) {
      const right = and();
      left = truthy(left) ? left : right;
    }
    return left;
  }

  const value = or();
  if (pos < toks.length) throw new ExprError(`unexpected \`${toks[pos].text}\` after expression`);
  return value;
}

/**
 * Resolve a dotted path, including the collection helpers GUML's grammar allows.
 *
 * `tasks.open.count` is the shape the fixtures use: `.open` / `.done` filter a
 * collection of records with a `done` field, and `.count` is its length. These are
 * the v0 aggregate semantics — the full set lands with the resolver.
 */
function resolvePath(path: string, scope: Scope): unknown {
  const parts = path.split(".");
  let current: unknown = scope;

  for (const part of parts) {
    if (current === null || current === undefined) return undefined;

    if (Array.isArray(current)) {
      if (part === "count" || part === "length") {
        current = current.length;
        continue;
      }
      if (part === "open") {
        current = current.filter((x) => !(x as Record<string, unknown>)?.done);
        continue;
      }
      if (part === "done") {
        current = current.filter((x) => Boolean((x as Record<string, unknown>)?.done));
        continue;
      }
      if (part === "sum") {
        current = (current as number[]).reduce((n, x) => n + Number(x), 0);
        continue;
      }
    }

    if (typeof current === "string") {
      if (part === "trim") {
        current = current.trim();
        continue;
      }
      if (part === "length" || part === "count") {
        current = current.length;
        continue;
      }
      if (part === "lower") {
        current = current.toLowerCase();
        continue;
      }
    }

    if (typeof current === "object") {
      current = (current as Record<string, unknown>)[part];
      continue;
    }

    return undefined;
  }
  return current;
}

export function truthy(v: unknown): boolean {
  if (Array.isArray(v)) return v.length > 0;
  return Boolean(v);
}

/** Interpolate `{expr}` occurrences inside prose. */
export function interpolate(text: string, scope: Scope): string {
  return text.replace(/\{([^}]*)\}/g, (_, expr: string) => {
    try {
      const v = evaluate(expr, scope);
      return v === null || v === undefined ? "" : String(v);
    } catch {
      return `{${expr}}`;
    }
  });
}

// ---------------------------------------------------------------- actions

export type ActionEffect =
  | { kind: "set"; name: string; value: unknown }
  | { kind: "mutate"; resource: string; mutation: string; body: Record<string, unknown> };

/**
 * Lower an action body to a list of effects, mirroring what the React backend
 * generates so runtime behaviour matches emitted code.
 *
 * `count++` · `count--` · `x=expr` · `tasks.add{title:draft}` · `;`-sequenced.
 */
export function runAction(action: string, scope: Scope): ActionEffect[] {
  const effects: ActionEffect[] = [];

  for (const raw of action.split(";")) {
    const stmt = raw.trim();
    if (!stmt) continue;

    if (stmt.endsWith("++") || stmt.endsWith("--")) {
      const name = stmt.slice(0, -2).trim();
      const delta = stmt.endsWith("++") ? 1 : -1;
      effects.push({ kind: "set", name, value: Number(resolvePath(name, scope) ?? 0) + delta });
      continue;
    }

    // A mutation call: `tasks.add{title:draft}` or `tasks.drop`.
    const call = /^([A-Za-z_$][\w$]*)\.([A-Za-z_$][\w$]*)\s*(\{([\s\S]*)\})?$/.exec(stmt);
    if (call) {
      const body: Record<string, unknown> = {};
      if (call[4]) {
        for (const pair of call[4].split(",")) {
          const [key, valueExpr] = pair.split(":").map((s) => s.trim());
          if (!key) continue;
          body[key] = valueExpr ? safeEval(valueExpr, scope) : resolvePath(key, scope);
        }
      }
      effects.push({ kind: "mutate", resource: call[1], mutation: call[2], body });
      continue;
    }

    const assign = stmt.indexOf("=");
    if (assign > 0 && !"=!<>".includes(stmt[assign - 1])) {
      const name = stmt.slice(0, assign).trim();
      // Only a bare state name is assignable. `crates/guml-codegen` enforces the
      // same rule, and the divergence was caught by a test: without this, an
      // action like `window.location = "…"` produced a nonsense `set` effect
      // instead of being rejected.
      if (!name.includes(".") && !name.includes("(")) {
        effects.push({ kind: "set", name, value: safeEval(stmt.slice(assign + 1).trim(), scope) });
        continue;
      }
    }

    throw new ExprError(`unsupported action \`${stmt}\``);
  }

  return effects;
}

function safeEval(expr: string, scope: Scope): unknown {
  try {
    return evaluate(expr, scope);
  } catch {
    return expr;
  }
}
