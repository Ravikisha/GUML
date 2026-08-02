/**
 * The GUML formatter and syntax classifier, as WebAssembly.
 *
 * This is the compiler's own `guml-fmt` crate — the same code `guml fmt` runs — not a
 * re-implementation. Formatting here and formatting on the command line produce identical bytes.
 *
 * **178 KB rather than 787 KB.** `guml-fmt` sits below the parser: it needs the lexer, the registry and
 * the diagnostic codes, and nothing else. No parser, no semantic analysis, no code generation, no
 * backends. A tool that formats GUML has no reason to download the code generator for seven of them.
 * `@guml/core` still exposes `format` and `highlight` from these same Rust functions, so this is a
 * smaller door to the same room rather than a fork — reach for core when you also need to *compile*.
 *
 * **Works in Node as well as the browser**, which `@guml/core` does not. That is deliberate and it is
 * most of the point: a formatter is wanted in pre-commit hooks, CI and editor tooling, all of which are
 * Node. See `load()` for how, and why it is not simply a `fetch`.
 */
var __rewriteRelativeImportExtension = (this && this.__rewriteRelativeImportExtension) || function (path, preserveJsx) {
    if (typeof path === "string" && /^\.\.?\//.test(path)) {
        return path.replace(/\.(tsx)$|((?:\.d)?)((?:\.[^./]+?)?)\.([cm]?)ts$/i, function (m, tsx, d, ext, cm) {
            return tsx ? preserveJsx ? ".jsx" : ".js" : d && (!ext || !cm) ? m : (d + ext + "." + cm.toLowerCase() + "js");
        });
    }
    return path;
};
import initWasm, { canonical as wasmCanonical, format as wasmFormat, highlight as wasmHighlight, version as wasmVersion, } from "../wasm/guml_fmt.js";
let ready;
const isNode = () => typeof process !== "undefined" && process.versions?.node !== undefined;
/**
 * Load the wasm. Called automatically by every function below; you rarely need it.
 *
 * **Why this is not just `initWasm()`.** `wasm-pack --target web` generates a loader that fetches the
 * `.wasm` beside itself. In a browser or through a bundler that is correct and needs no help. In Node
 * it fails: `fetch` does not support `file://` URLs, and the error surfaces as an undici stack trace
 * that never mentions WebAssembly, so it reads as a broken install rather than a wrong environment.
 *
 * Reading the bytes ourselves on Node avoids all of that. The module specifier is held in a variable so
 * that bundlers targeting the browser do not try to resolve `node:fs/promises` — the branch is
 * unreachable there anyway, but a static import would still be analysed and warned about.
 */
export function init(wasm) {
    ready ??= (async () => {
        if (wasm) {
            await initWasm({ module_or_path: wasm });
            return;
        }
        if (isNode()) {
            const fs = "node:fs/promises";
            const { readFile } = await import(__rewriteRelativeImportExtension(/* @vite-ignore */ /* webpackIgnore: true */ fs));
            const bytes = await readFile(new URL("../wasm/guml_fmt_bg.wasm", import.meta.url));
            await initWasm({ module_or_path: bytes });
            return;
        }
        await initWasm();
    })();
    return ready;
}
/**
 * Format a document.
 *
 * Idempotent — formatting formatted output returns it unchanged, which is what makes a `--check` mode
 * meaningful. Comments and blank lines are preserved; see {@link canonical} if you want them gone.
 */
export async function format(source) {
    await init();
    return wasmFormat(source);
}
/**
 * Canonical form: comments and blank lines stripped, directives hoisted and sorted, the shortest
 * spelling of every value preferred.
 *
 * Two documents that mean the same thing become byte-identical. That is what makes two independent
 * generations of one interface comparable, and it is why this is separate from {@link format} rather
 * than an option on it — **it deletes commentary on purpose**, so it is a normaliser for comparison,
 * never a formatter for an editor.
 */
export async function canonical(source) {
    await init();
    return wasmCanonical(source);
}
/**
 * Classify every byte for highlighting, using the compiler's own lexer and registry.
 *
 * For a page that renders on a server, `@guml/highlight` does this in ~15 KB of TypeScript with no wasm
 * and no `await`, held to this implementation by a parity test over every fixture. Prefer that unless
 * you need exactness by construction rather than by test.
 */
export async function highlight(source) {
    await init();
    return JSON.parse(wasmHighlight(source));
}
/** The version of the compiler this wasm was built from. */
export async function version() {
    await init();
    return wasmVersion();
}
/**
 * True if `source` is already formatted — the `--check` predicate, without a second round trip.
 *
 * Idempotence is what makes this sound: `format(format(x)) === format(x)`, so comparing one pass
 * against the input answers the question completely.
 */
export async function isFormatted(source) {
    return (await format(source)) === source;
}
//# sourceMappingURL=index.js.map