/* tslint:disable */
/* eslint-disable */

/**
 * Parse and analyse. Returns every diagnostic in one pass, which is what keeps
 * an editor or a repair loop to a single round trip.
 */
export function check(source: string): any;

/**
 * Compile to a backend: `"react"` for source text, `"json"` for a render tree.
 */
export function compile(source: string, backend?: string | null): any;

/**
 * Format source. `canonical` strips comments, blank lines and declaration order so that
 * two semantically identical documents produce identical bytes.
 */
export function format(source: string, canonical?: boolean | null): any;

/**
 * Syntax classification from the real lexer and registry, so a browser highlighter cannot
 * drift from the compiler. Returns one entry per coloured span, in source order.
 */
export function highlight(source: string): any;

/**
 * The component vocabulary. `tags` narrows it to a prompt-sized slice.
 */
export function registry(tags?: string | null): any;

/**
 * The render tree, for the runtime renderer. Diagnostics come along so a preview
 * can show the problem instead of rendering something misleading.
 */
export function tree(source: string): any;

/**
 * Compiler version, so a host can report which build produced a result.
 */
export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly check: (a: number, b: number) => [number, number, number];
    readonly compile: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly format: (a: number, b: number, c: number) => [number, number, number];
    readonly highlight: (a: number, b: number) => [number, number, number];
    readonly registry: (a: number, b: number) => [number, number, number];
    readonly tree: (a: number, b: number) => [number, number, number];
    readonly version: () => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
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
