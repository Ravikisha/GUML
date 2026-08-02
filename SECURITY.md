# Security policy

## Supported versions

GUML is `0.x`. Only the latest released version receives security fixes.

| Version | Supported |
|---|---|
| 0.1.x | yes |
| < 0.1 | no |

## Reporting a vulnerability

**Do not open a public issue.**

Report privately through
[GitHub Security Advisories](https://github.com/guml-lang/guml/security/advisories/new),
which lets us discuss and patch before anything is disclosed.

Please include the GUML source that triggers it, the emitted output, the backend
(`react`, `svelte`, `html`, `wc`, `json`, `a2ui`, `mcp-ui`), and the version.

You should get an acknowledgement within 3 working days and an assessment within
10. If a fix is warranted we will agree a disclosure date with you, and credit you
in the advisory unless you would rather we did not.

## What counts as a vulnerability here

GUML is a compiler. It takes source — frequently source *written by a language
model*, from a prompt that may itself contain text an attacker supplied — and emits
code that runs in someone's browser. That makes the interesting question not "can
GUML be crashed" but **"can a GUML document cause the compiler to emit something its
author did not authorise"**.

In scope, in rough order of how much we care:

- **Escaping the output.** A document whose text, attribute value or expression
  breaks out of the construct it was lowered into and becomes executable code — an
  injection into emitted JSX, a template literal, an HTML attribute, a CSS
  declaration or a JSON string. This is the one that matters most, because it turns
  untrusted *content* into code.
- **Escaping the action language.** GUML actions are deliberately not
  Turing-complete, and that boundary is the security boundary, not a design
  preference (see below). Anything that reaches arbitrary evaluation without going
  through `js` or `raw` is a vulnerability.
- **A capability that is not declared.** `guml capabilities` reports what a document
  will do and emits a matching Content-Security-Policy. A document that performs
  network access, storage or script evaluation the report does not mention defeats
  the control built on top of it.
- **Silent mis-lowering with a security consequence** — dropped escaping, a lost
  `rel="noopener"`, a sanitiser that does not run. Note that mis-lowering in general
  is a correctness invariant we treat as a bug of the highest severity even when it
  has no security impact.
- Denial of service in the compiler: non-termination, unbounded memory, or
  superlinear blowup on a small input. `check` runs on a keystroke in the LSP and
  between rounds of the repair loop, so a hostile document that hangs it is a real
  problem.
- Anything in the published artifacts: the wasm build, the CLI, the npm packages.

## What is not a vulnerability

- **`js` and `raw` blocks execute arbitrary code, by design.** They are the escape
  hatch and the security boundary at once: everything outside them is constrained,
  and everything inside them is the author's own code, compiled through unchanged.
  A `js` block that does something dangerous is the document doing it. If you are
  compiling documents you do not trust, `guml capabilities` will tell you a document
  uses an escape hatch, and refusing those documents is the intended control.
- Compiling a document you already do not trust and then running the output without
  reading it. The threat model is that GUML does not *add* an escape the source did
  not ask for.
- Vulnerabilities in the components a registry package points at. `@guml/shadcn`
  redistributes shadcn/ui, which is built on Radix and others; report those upstream,
  though we would like to know so we can bump the dependency.
- The hosted docs demo being rate-limited or unavailable.

## Security-relevant design, stated plainly

Three properties are load-bearing, and it is worth knowing them before you look:

1. **Actions are not Turing-complete.** The action language expresses state
   transitions and nothing else. Anything more expressive must be written in a `js`
   block, which is visible in the source, reported by `guml capabilities`, and
   counted against a per-document budget in CI.
2. **The compiler owns presentation.** GUML source contains no class strings, no
   colours, no ARIA plumbing. There is one element table, one class table and one
   expression lowering shared across all seven backends, so a fix to escaping in one
   place fixes it everywhere rather than in one backend out of seven.
3. **Unsupported constructs warn.** A construct the compiler cannot lower produces a
   warning and a `TODO` in the output, never silently-wrong code.
