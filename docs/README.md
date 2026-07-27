# GUML documentation site

The docs for the GUML compiler, which lives in the parent directory.

```sh
pnpm install
pnpm dev            # http://localhost:3000
pnpm build          # regenerates fixtures, then next build
pnpm gen:fixtures   # refresh code samples from ../fixtures and the compiler
```

## How content stays honest

Two things are generated rather than written, because both are load-bearing claims:

- **Code samples** come from `../fixtures/*.guml`, `../fixtures/*.react.tsx`, and the actual
  output of `cargo run -p guml-cli -- build`. A stale sample is impossible; a fixture that stops
  compiling shows up here.
- **The component vocabulary** on `/docs/language/registry` is read from `guml registry`, so the
  table cannot drift from what the parser accepts.

Both land in `lib/fixtures.generated.ts` via `scripts/gen-fixtures.mjs`, which runs before every
build. It degrades gracefully: without a Rust toolchain the compiled-output panes are omitted and
the rest of the site still builds.

Token counts shown on the site are the measured figures from `../GUML-Research-Report.md` §1.5
(cl100k_base, hand-authored fixtures) — not the compiler's `guml tokens` estimate, which is
labelled as an estimate wherever it appears.

## Design notes

- **Direction — "the meter".** GUML's thesis is a falling number, so the site is organised around
  counting. Ember marks the verbose source, iris the compact representation, and mint is reserved
  for one job: what was saved.
- **Signature.** The hero is a chart, not an ornament: one cell is 8 real tokens of the task
  fixture, 158 of 180 burn off, and the headline's variable width axis narrows from 100 to 76 on the
  same GSAP timeline. Reduced motion jumps to the end state.
- **Type.** Bricolage Grotesque for display (chosen for that width axis), Geist for body, Geist Mono
  for code and for every structural label.
- **Syntax highlighting** is a port of the compiler's own lexer rules (`lib/highlight.ts`) rather
  than a highlighting library — same tokenization as the parser, and no dependency.
- Dark-only, deliberately. One committed surface keeps the accent pair legible everywhere.

## Stack

Next.js 16 (App Router, Turbopack) · Tailwind v4 · motion · GSAP · Radix primitives · cmdk ·
lucide-react. UI primitives are owned in-repo in `components/ui.tsx` rather than pulled from a
component library, so they inherit this site's tokens instead of a default theme.
