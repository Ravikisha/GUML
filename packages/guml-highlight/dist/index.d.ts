/**
 * Syntax highlighting for GUML.
 *
 * **No WebAssembly, and that is the entire point of this package.** The compiler ships its own
 * classifier — `guml_fmt::highlight`, reachable from `@guml/core` — and it is the authoritative one.
 * But reaching it means loading 787 KB of compiler wasm, asynchronously, in a browser. Highlighting a
 * snippet in a static page needs none of that: it has to run *synchronously during server rendering*,
 * and it has to work in Node, where the wasm build cannot load at all.
 *
 * So this is a hand-written tokeniser, ~15 KB, zero dependencies. What makes that safe rather than a
 * second source of truth is the parity gate: `pnpm check:highlight` runs this and the compiler's own
 * classifier over every fixture and fails on any disagreement. 936 spans across 10 documents currently
 * agree. A hand-maintained highlighter drifts silently, and this one already had — it listed `h3`,
 * which the registry does not define.
 *
 * The vocabulary is generated from `guml registry` into `vocabulary.generated.ts`, never retyped. Both
 * halves matter: the parity check catches tokenising that disagrees, and the generated vocabulary means
 * a tag added in Rust reaches this package without a second edit.
 *
 * TSX / bash / JSON are ordinary regex grammars. Nothing in the compiler describes them, so there is
 * nothing for them to drift from.
 */
/**
 * `cls` is the compiler's class name (`guml_fmt::highlight::Class::name`), not a CSS class.
 * Mapping to colour happens in `CLASS_STYLE` so the parity check can compare names.
 */
export type Tok = {
    text: string;
    cls: string;
};
export type Lang = "guml" | "tsx" | "python" | "bash" | "json" | "text";
/** The only place a class name becomes a colour. */
export declare const CLASS_STYLE: Record<string, string>;
/** Tokenize source into lines of coloured spans. */
export declare function highlight(code: string, lang: Lang): Tok[][];
/**
 * Rough token estimate, matching the compiler's own `guml tokens` heuristic
 * (~3.6 chars/token). Estimates only — the real figures in the research report
 * were measured with a real tokenizer.
 */
export declare function approxTokens(code: string): number;
//# sourceMappingURL=index.d.ts.map