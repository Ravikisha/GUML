/**
 * guml — the GUML compiler, in the browser.
 *
 * This is the real Rust compiler built to wasm32, not a re-implementation, so
 * diagnostics and generated classes match the CLI exactly. The wasm module loads
 * lazily on first use and is cached for the lifetime of the page.
 */

import initWasm, {
  check as wasmCheck,
  compile as wasmCompile,
  fix as wasmFix,
  format as wasmFormat,
  highlight as wasmHighlight,
  registry as wasmRegistry,
  repair as wasmRepair,
  tree as wasmTree,
  version as wasmVersion,
} from "../wasm/guml.js";

// ---------------------------------------------------------------- types

export type Severity = "error" | "warning" | "note";

export type Span = {
  start: number;
  end: number;
  /** 1-based. */
  line: number;
  /** 1-based, absolute within the line. */
  col: number;
};

export type Diagnostic = {
  /** Stable code, e.g. `GUML0030`. Append-only across versions. */
  id: string;
  code: string;
  severity: Severity;
  message: string;
  span: Span;
  help?: string;
  /**
   * A literal replacement for `span`, present only when the fix is unambiguous.
   * Safe to apply mechanically — that is what it exists for.
   */
  suggestion?: string;
};

export type CheckResult = {
  ok: boolean;
  errorCount: number;
  diagnostics: Diagnostic[];
};

export type CompileResult = {
  ok: boolean;
  files: Array<{ path: string; contents: string }>;
  diagnostics: Diagnostic[];
  stats: {
    sourceBytes: number;
    sourceLines: number;
    emittedBytes: number;
    /** ~3.6 chars/token heuristic. An estimate — never a published figure. */
    approxSourceTokens: number;
    approxEmittedTokens: number;
  };
};

export type Prop = {
  name: string;
  value: unknown;
  /** When true, `value` is a binding expression to evaluate, not a literal. */
  bound: boolean;
};

export type UiNode = {
  tag: string;
  /** `null` for tags this compiler version cannot lower yet. */
  el: string | null;
  class: string;
  text: string | null;
  label: string | null;
  bind: string | null;
  props: Prop[];
  actions: string[];
  source: string | null;
  filter: string | null;
  ariaFrom: string | null;
  lines: string[];
  children: UiNode[];
};

export type UiTree = {
  page: string;
  state: Array<{ name: string; init: unknown; domain: string[] }>;
  resources: Array<{
    name: string;
    ty: string;
    method: string;
    url: string;
    mutations: Array<{
      name: string;
      method: string;
      url: string;
      body: string[];
      optimistic: string | null;
    }>;
  }>;
  /**
   * Declared `on` effects. Optional because a tree emitted before they existed has none, and a
   * consumer pinned to an older compiler should not fail to parse a newer document.
   *
   * Carried as data rather than dropped the way a `js` body is: the trigger is an expression and the
   * action is the same restricted language every button in this tree already uses, so nothing here
   * can reach `eval`.
   */
  effects?: Array<{
    /** `"mount"`, or the trigger expression as written. */
    on: string;
    actions: string[];
  }>;
  nodes: UiNode[];
};

export type TreeResult = { ok: boolean; tree: UiTree; diagnostics: Diagnostic[] };

export type RegistryResult = {
  components: Array<{
    name: string;
    kind: string;
    doc: string;
    requiresLabel: boolean;
    attrs: string[];
  }>;
  modifiers: string[];
  globalAttrs: string[];
};

export type Backend = "react" | "json";

export type FormatResult = { text: string; changed: boolean };

export type FixResult = {
  text: string;
  /** Diagnostic codes that were applied, one entry per edit. */
  codes: string[];
  rounds: number;
};

export type RepairResult = {
  text: string;
  /** True when the repaired document has no errors left. */
  ok: boolean;
  /** True when any layer changed anything. */
  changed: boolean;
  errorsBefore: number;
  errorsAfter: number;
  /** What was removed as packaging rather than document. */
  sanitize: {
    /** A ``` fence was unwrapped. */
    fence: boolean;
    /** Markdown horizontal rules removed. */
    rules: number;
    /** Trailing commentary lines dropped. */
    trailing: number;
  };
  reformatted: boolean;
  /** Diagnostic codes `fix` applied, one entry per edit. */
  applied: string[];
  rounds: number;
  /** One human-readable line per layer that did something. */
  report: string[];
};

/**
 * Syntax classes, produced by the compiler's own lexer and registry. A regex highlighter
 * cannot produce these: whether a line's remainder is structure or prose depends on the
 * tag, which only the registry knows.
 */
export type HighlightClass =
  | "tag"
  | "directive"
  | "modifier"
  | "binding"
  | "string"
  | "number"
  | "attr"
  | "action"
  | "prose"
  | "comment"
  | "route"
  | "anchor"
  | "punct"
  | "text";

export type HighlightSpan = {
  start: number;
  end: number;
  line: number;
  class: HighlightClass;
  /** The LSP `SemanticTokenType` this maps onto, for editor integrations. */
  lsp: string;
};

// ---------------------------------------------------------------- loading

let ready: Promise<void> | null = null;

/**
 * How the wasm binary can be supplied. A URL is the browser case; the rest exist because a URL is
 * *not enough* outside one.
 *
 * The build targets `web`, so with no argument the generated glue resolves the `.wasm` beside itself
 * and `fetch`es it. Under Node that fails outright — `fetch` on a `file:` URL is not implemented — so
 * the package could not be initialised off the browser at all, which rules out server-side rendering,
 * a CLI wrapper, and its own test suite. Accepting bytes fixes all three, and costs nothing: passing
 * a `BufferSource` is already what `wasm-bindgen`'s `module_or_path` supports.
 */
export type WasmSource = string | URL | BufferSource | WebAssembly.Module | Response;

/**
 * Load the compiler. Optional in a browser — every API call awaits it — but useful to warm the module
 * before a user starts typing, and **required** under Node, where there is nothing to `fetch`:
 *
 * ```ts
 * import { readFile } from "node:fs/promises";
 * await init(await readFile(new URL("../wasm/guml_bg.wasm", import.meta.url)));
 * ```
 */
export function init(wasm?: WasmSource): Promise<void> {
  ready ??= initWasm(wasm ? { module_or_path: wasm } : undefined).then(() => undefined);
  return ready;
}

/** True once the wasm module has finished loading. */
export function isReady(): boolean {
  return loaded;
}

let loaded = false;
async function load() {
  await init();
  loaded = true;
}

// ---------------------------------------------------------------- api

/** Parse and analyse. Reports every problem in one pass. */
export async function check(source: string): Promise<CheckResult> {
  await load();
  return wasmCheck(source) as CheckResult;
}

/** Compile to framework source (`react`) or to a render tree (`json`). */
export async function compile(
  source: string,
  backend: Backend = "react",
): Promise<CompileResult> {
  await load();
  return wasmCompile(source, backend) as CompileResult;
}

/** The render tree the React runtime consumes. */
export async function tree(source: string): Promise<TreeResult> {
  await load();
  return wasmTree(source) as TreeResult;
}

/**
 * The component vocabulary. Pass tag names to get a prompt-sized slice — the same
 * retrieval path the CLI's `guml registry --tags` exposes.
 */
export async function registry(tags?: string[]): Promise<RegistryResult> {
  await load();
  return wasmRegistry(tags?.join(",")) as RegistryResult;
}

/**
 * Format source.
 *
 * `canonical` strips comments, blank lines and declaration order, so two documents that
 * mean the same thing produce the same bytes — what dedup and inter-run comparison need.
 * Formatting never changes the AST; that is enforced by a test in the compiler.
 */
export async function format(source: string, canonical = false): Promise<FormatResult> {
  await load();
  return wasmFormat(source, canonical) as FormatResult;
}

/**
 * Apply every unambiguous diagnostic suggestion. No model call.
 *
 * The free layer of the repair loop: renaming `crad` to `card` is an edit the compiler already
 * described precisely, and spending a generation on it is the most expensive way to fix a typo.
 * Suggestions that are *templates* (`aria="…"`) are left for a human.
 */
export async function fix(source: string, rounds = 3): Promise<FixResult> {
  await load();
  return wasmFix(source, rounds) as FixResult;
}

/**
 * The whole free repair pipeline: sanitise, format, fix. No model call.
 *
 * This is what to run on raw model output. `fix` only applies edits the compiler described, so it
 * still fails on the packaging a model wraps around a document — a ```` ```guml ```` fence, a
 * markdown rule, a closing "This page counts clicks." sentence. Those layers existed only in the
 * benchmark harness, which meant the measured pipeline could repair things the shipped package
 * could not.
 *
 * Every layer is guarded: one that would raise the error count is discarded rather than kept, and
 * `report` names the layers that did the work — so "the repair loop helped" is a statement with
 * evidence rather than an assumption.
 */
export async function repair(source: string, rounds = 3): Promise<RepairResult> {
  await load();
  return wasmRepair(source, rounds) as RepairResult;
}

/** Classify every byte for highlighting, using the compiler's lexer and registry. */
export async function highlight(source: string): Promise<HighlightSpan[]> {
  await load();
  return wasmHighlight(source) as HighlightSpan[];
}

/** Version of the compiler that produced a result. */
export async function version(): Promise<string> {
  await load();
  return wasmVersion();
}

/** Render a diagnostic the way the CLI does, for terminals and `<pre>` blocks. */
export function formatDiagnostic(d: Diagnostic, source?: string): string {
  const lines: string[] = [`${d.severity}[${d.id}]: ${d.message}`];
  if (source) {
    const line = source.split("\n")[d.span.line - 1];
    if (line !== undefined) {
      const pad = " ".repeat(Math.max(0, d.span.col - 1));
      const width = Math.max(1, d.span.end - d.span.start);
      lines.push(`  ${d.span.line} | ${line}`, `  ${" ".repeat(String(d.span.line).length)} | ${pad}${"^".repeat(width)}`);
    }
  }
  if (d.help) lines.push(`  = help: ${d.help}`);
  if (d.suggestion) lines.push(`  = suggestion: ${d.suggestion}`);
  return lines.join("\n");
}

/**
 * Apply a diagnostic's suggestion to the source. Only acts when the diagnostic
 * carries an unambiguous replacement, so this is safe to run unattended — it is
 * the no-model-call half of the repair loop.
 */
export function applySuggestion(source: string, d: Diagnostic): string {
  if (!d.suggestion || !isApplicable(d)) return source;
  return source.slice(0, d.span.start) + d.suggestion + source.slice(d.span.end);
}

/**
 * Whether a suggestion is a replacement or a *template*.
 *
 * Accessibility diagnostics suggest shapes like `toggle aria="…"`, where the ellipsis is a
 * placeholder for a human. Splicing that in literally puts an ellipsis in the accessible
 * name, which is worse than the original problem — the code said "unambiguous" and did not
 * check.
 */
export function isApplicable(d: Diagnostic): boolean {
  return Boolean(d.suggestion) && !d.suggestion!.includes("…");
}

/** Apply every unambiguous suggestion, right to left so offsets stay valid. */
export function applyAllSuggestions(source: string, diagnostics: Diagnostic[]): string {
  return [...diagnostics]
    .filter(isApplicable)
    .sort((a, b) => b.span.start - a.span.start)
    .reduce(applySuggestion, source);
}

export { evaluate, runAction } from "./eval.ts";
