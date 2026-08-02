/* tslint:disable */
/* eslint-disable */

/**
 * Canonical form: comments and blank lines stripped, directives hoisted and sorted, the shortest
 * spelling of every value preferred.
 *
 * Two documents that mean the same thing become byte-identical, which is what makes independent
 * generations of one interface comparable. Not what you want in an editor — this is a normaliser, and
 * it deletes commentary on purpose.
 */
export function canonical(src: string): string;

/**
 * Format a document. Idempotent: formatting formatted output returns it unchanged.
 */
export function format(src: string): string;

/**
 * Classify every byte for highlighting, as JSON:
 * `[{ "line": 1, "start": 0, "end": 4, "class": "tag" }]`.
 *
 * The same classifier `guml highlight` uses, so the class names are the compiler's own. For a page
 * that renders on a server, `@guml/highlight` does this in ~15 KB of TypeScript with no wasm at all,
 * held to this implementation by a parity test — prefer it unless you need exactness by construction
 * rather than by test.
 */
export function highlight(src: string): string;

/**
 * The crate version this wasm was built from.
 */
export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly canonical: (a: number, b: number) => [number, number];
    readonly format: (a: number, b: number) => [number, number];
    readonly highlight: (a: number, b: number) => [number, number];
    readonly version: () => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
