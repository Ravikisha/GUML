# tree-sitter-guml

A tree-sitter grammar for GUML, for editors that colour and fold with tree-sitter — Neovim, Helix, Zed,
GitHub — and for structural selection.

## Status: 14 of 14 corpus cases, and all 10 real documents parse clean

```sh
npm install            # tree-sitter-cli
npm run gen:tags       # tag list from `guml registry`
npm run build          # tree-sitter generate
npm test               # the corpus, then every real .guml in the repository
npm run check:fixtures # just the second half
```

### The corpus passed 12 of 12 with three bugs live

That is the thing worth taking from this directory. Seven bugs were found here in total, and the last four
were found by pointing the parser at `fixtures/` — not by the twelve documents someone had written to
demonstrate features. Two show why:

- **Two top-level siblings nested**, because at depth zero the scanner emitted an indent token for *any*
  line. Invisible to the corpus for a subtle reason: every case had a `page` directive before its first
  indent, and a directive has no body, so `valid_symbols[INDENT]` was false and the broken branch never
  ran.
- **`divider` had no possible parse.** It is a text-kind tag that normally carries no text, and the scanner
  returned nothing at all for an empty prose remainder rather than falling through to `NEWLINE`. No corpus
  case had a bare text tag. Nine `ERROR` nodes in `fixtures/e.guml`.

So `npm run check:fixtures` parses every `.guml` under `fixtures/` and `bench/phase0/examples/` and fails
on a single `ERROR` or `MISSING` node. It is the same argument as the `highlight-parity` check: agree with
the compiler on the input it is actually tested against, not on examples. The corpus still matters — zero
error nodes is not the same as the *right* tree — so `npm test` runs both, and CI runs `npm test`.

### The one that cost the most to find

**tree-sitter persists external-scanner state only for a call that returns a token.** A call that returns
`false` has its mutations thrown away. Three separate bugs here were the same mistake: the scanner tried to
remember something across lines — which body a `tier` header had opened, whether the current line's tag
was text-kind — and a header line produces no external token, so the flag was gone before it was read.

Two different fixes, and the difference between them is the design lesson:

- For *which body is this* — ask the grammar. After a `tier`/`faq` or `js`/`raw` header only
  `_verbatim_indent` is valid, so the scanner reads `valid_symbols` and emits whichever indent is wanted.
  The dependency runs the right way round and there is no state at all.
- For *is this line's remainder prose* — decide it while producing a token. The lookup happens on the
  `NEWLINE` that ends the *previous* line, peeking one line ahead. `NEWLINE` always returns a token, so the
  decision survives.

One known limitation follows from that second fix: a document whose *first* line is a text tag has no
previous line, so `p Hello world.` at the very top colours as words rather than one `prose` node. It is
only reachable in a document the compiler rejects with `GUML0041` (no `page` directive), and it degrades to
imperfect colour rather than to a parse error.

Nothing else in the project depends on this grammar — the language server provides colour via
`guml highlight`, and the VS Code extension ships a generated TextMate grammar.

## Two things a context-free grammar cannot express

Both live in the external scanner, which is the whole reason one is needed.

1. **Indentation is structure.** Children are the following lines with a *strictly greater* indent,
   applied recursively — so `4` then `5` is a parent and a child, not two ragged siblings. The scanner
   maintains the indent stack and emits `INDENT`/`DEDENT`, mirroring `guml-syntax`'s rule exactly. A
   grammar that guessed "those look like siblings" would produce a different tree from the compiler's.

2. **Prose versus structure depends on the tag.** `btn Decrement ghost` is a label plus a modifier;
   `p Press the buttons` is prose taken verbatim. Which one a line is depends on the tag's kind in the
   component registry, so the scanner carries a compiled-in list of the text-kind tags and looks up the
   line's first word.

That list is **generated** from `guml registry` by `scripts/gen-tags.mjs`. A hand-maintained copy would
be a second vocabulary that can drift from the compiler — which has already happened twice in this
project (the docs highlighter, and a duplicated expression grammar), both times caught only because
someone thought to compare them.

## What this grammar is not

It is not the definition of GUML. The normative definition is the executed conformance suite in
`spec/tests/*.txt`; `spec/grammar.ebnf` is the artifact fed to grammar-prompting and
grammar-constrained decoding. A tree-sitter grammar that disagreed with the compiler would be a
highlighting bug, not a language change — and the way to settle it is to add a case to the suite.

Two deliberate shallownesses:

- **Expressions** are matched as `[^}\n]*`. The compiler has a real precedence-climbing parser for
  them; re-deriving it here would be a third implementation. An editor needs to know where an
  expression *is*, not what it evaluates to.
- **Tags and modifiers** are plain identifiers rather than an enumeration. The tag set is not fixed at
  grammar-writing time — a host can load its own registry, and a document can declare components with
  `def` — so deciding whether a particular tag resolves belongs to the language server.

## Layout

```
grammar.js              the grammar
src/scanner.c           external scanner: INDENT/DEDENT/NEWLINE/PROSE/RAW_LINE
src/tags.h              GENERATED text-kind tag list
scripts/gen-tags.mjs    writes src/tags.h from `guml registry`
test/corpus/            tree-sitter corpus tests
queries/highlights.scm  capture names, kept in step with `guml highlight`
```
