#!/usr/bin/env node
/**
 * Compile `example.guml` against this package's registry and typecheck the result **against the components**.
 *
 * # Why this is the check that matters for a registry package
 *
 * `guml registry --validate` says the document is well-formed. `pnpm typecheck` says the components compile.
 * Neither says the two agree — and that gap is where a package actually goes wrong: an entry declares
 * `attrs: ["rows", "of", "kind"]`, the compiler emits those as props, and nothing checks that `Chart` accepts
 * them or that their types line up.
 *
 * It found three real bugs the first time it ran, all in the compiler rather than in the package:
 *
 * * `of=revenue` and `kind=line` were **silently dropped**. The React backend's attribute loop encodes what
 *   each name means *for a builtin* — `of` belongs to a repeater, `kind` folds into an `<input>`'s `type` —
 *   and it applied that to a component it knows nothing about. Two declared props gone, no diagnostic, and a
 *   chart plotting nothing.
 * * The title positional never reached `aria-label`, so a component whose entry says `requires_label` was
 *   emitted with no accessible name. The compiler enforces the contract on the *document* and then dropped
 *   it on the way out.
 * * `date from` emitted the state *name* as children instead of `value`/`onChange`. Only `input` and `select`
 *   were wired for two-way binding, so a package's own `field` was decorative — the same shape as the
 *   `select` that once leaked its bound state name as element text.
 *
 * Run from this directory: `pnpm typecheck:example`.
 */
import { execFileSync } from "node:child_process";
import { mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const PKG = resolve(HERE, "..");
const ROOT = resolve(PKG, "..", "..");

// TypeScript resolves `import … from "@guml/shadcn"` by walking up from the *importing* file, so the emitted
// component is written inside this package. A temp directory would find no `node_modules` at all and every
// prop would degrade to an implicit `any` — 40 spurious errors that say nothing about the generated code.
const out = join(PKG, ".guml-emitted");

const { default: ts } = await import(
  pathToFileURL(join(PKG, "node_modules", "typescript", "lib", "typescript.js")).href
);

try {
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });

  execFileSync(
    "cargo",
    [
      "run",
      "-q",
      "-p",
      "guml-cli",
      "--",
      "build",
      "packages/guml-shadcn/example.guml",
      "--registry",
      "packages/guml-shadcn/guml.registry.json",
      "-o",
      out,
    ],
    { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "ignore", "inherit"] },
  );

  const emitted = readdirSync(out).filter((f) => f.endsWith(".tsx"));
  if (emitted.length === 0) throw new Error("the compiler emitted no .tsx files");

  // `@guml/shadcn` resolves to this package's own source through the workspace link, so the props checked
  // are the real ones.
  writeFileSync(
    join(out, "tsconfig.json"),
    JSON.stringify(
      {
        compilerOptions: {
          target: "ES2022",
          lib: ["ES2022", "DOM"],
          jsx: "react-jsx",
          module: "ESNext",
          moduleResolution: "bundler",
          strict: true,
          noEmit: true,
          skipLibCheck: true,
          // Both aliases. `@guml/shadcn` is what the *emitted document* imports; `@/*` is what the
          // components import each other by, and without it 57 "cannot find module '@/lib/utils'" errors
          // bury the handful of real prop mismatches this script exists to find.
          paths: { "@guml/shadcn": ["../src/index.ts"], "@/*": ["../src/*"] },
        },
        include: ["*.tsx"],
      },
      null,
      2,
    ),
  );

  const config = ts.parseJsonConfigFileContent(
    JSON.parse(
      // Read it back rather than reusing the object, so the file on disk is what is checked.
      execFileSync(process.execPath, ["-e", `process.stdout.write(require('node:fs').readFileSync(${JSON.stringify(join(out, "tsconfig.json"))},'utf8'))`], { encoding: "utf8" }),
    ),
    ts.sys,
    out,
  );

  console.log(`typechecking ${emitted.length} emitted component(s) against @guml/shadcn…`);
  const program = ts.createProgram(config.fileNames, config.options);
  const diagnostics = ts.getPreEmitDiagnostics(program);

  if (diagnostics.length > 0) {
    console.error(
      ts.formatDiagnosticsWithColorAndContext(diagnostics, {
        getCanonicalFileName: (f) => f,
        getCurrentDirectory: () => ROOT,
        getNewLine: () => "\n",
      }),
    );
    console.error(
      `${diagnostics.length} type error(s): the registry entries and the components disagree, or the ` +
        `compiler is emitting props the components do not accept.`,
    );
    process.exit(1);
  }
  console.log("the emitted component typechecks against the package's own components");
} finally {
  rmSync(out, { recursive: true, force: true });
}
