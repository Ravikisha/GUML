# GUML task runner. `just` is optional — every recipe is a one-line cargo command.
# Install: cargo install just

default: test

build:
    cargo build --workspace

test:
    cargo test --workspace

lint:
    cargo clippy --workspace --all-targets -- -D warnings

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

# The Rust half. Fast enough to run on every save.
ci-rust: fmt-check lint test fmt-guml-check

# **Everything CI runs.** One trusted command.
#
# `ci` used to be `fmt-check lint test fmt-guml-check` — the Rust half only. Every other gate the
# project relies on (emitted TypeScript typechecks, emitted components render, the docs highlighter
# agrees with the compiler span-for-span, the npm package's tests, the Phase 0 preflight) existed as a
# separate recipe that had to be remembered, and CI ran a superset of what any developer could
# reproduce locally. That is how a green local run and a red CI run become normal.
#
# Needs Node and pnpm as well as cargo. `ci-rust` is the subset that needs neither.
ci: ci-rust mutation validate-all capabilities-budget reference-budget typecheck-emitted render-emitted check-wc highlight-parity check-tags grammar-test package-test wasm-fresh split-packages-test widgets-test shadcn-test editor-test check-packages docs-build phase0-verify bench-verify

# Every `.guml` in the repository parses and analyses cleanly.
validate-all:
    cargo run -q -p guml-cli -- validate fixtures bench/phase0/examples bench/guml-bench/reference

# The escape-hatch budget. A rising rate is the early warning that the vocabulary is hitting an
# expressiveness cliff (report §12.1 risk 5) — which is only a signal if something fails on it.
#
# Three is `d.guml`'s count, and `d.guml` exists *to* exercise `js` and `raw`. The number is a ratchet:
# lower it when the vocabulary grows enough to make a hatch unnecessary, and never raise it without
# saying which construct could not be expressed.
capabilities-budget:
    cargo run -q -p guml-cli -- capabilities fixtures bench/phase0/examples --max-escapes 3

# The same budget for the GUML-Bench reference answers, held separately.
#
# **Per document, not a total.** `--max-escapes` compares each file's own count; an earlier version of this
# comment described it as a sum across the corpus, which would have made the number drift meaningless as the
# corpus grew.
#
# **One** is the ceiling, and it is a real constraint: six of the nine reference answers use exactly one `js`
# block and three use none. Each hatch is named in its file's header with the construct that could not be
# expressed — an aggregate over an expression (a cart subtotal), a lookup into a collection by two keys,
# counting rows by a *value* rather than by truthiness, the first row of a single-object endpoint, and a
# conjunction of more than one client-side filter.
#
# These are whole task answers, so the count measures how far the vocabulary reaches rather than being a
# number to drive to zero. A ratchet in the same direction as the fixture budget: lower it when the
# vocabulary grows, and never raise it without naming the construct that forced the hatch.
reference-budget:
    cargo run -q -p guml-cli -- capabilities bench/guml-bench/reference --max-escapes 1

# What a document will actually do, and a CSP for it.
capabilities file="fixtures/b.guml":
    cargo run -q -p guml-cli -- capabilities {{file}}

# The npm package: the wasm compiler plus the React runtime.
package-test:
    cd packages/guml && pnpm test && pnpm typecheck

# The two things that can be wrong about a committed build artifact. Different causes, so checked
# separately — and both were live when this recipe was written.
#
# **It must be in the tarball.** wasm-pack writes a `.gitignore` containing `*` into its out-dir, and npm
# honours a nested `.gitignore` over the `files` allowlist. So `npm publish` would have shipped a
# compiler package with no compiler in it — `exports["./wasm"]` resolving to a file that is not there,
# broken on install for every user. Invisible to every other gate here, because they all run against the
# working tree where the file plainly exists.
#
# **It must match its source.** `prepublishOnly` ran `build:ts` without `build:wasm`, so a publish shipped
# whatever binary happened to be committed. It had drifted by five days.
wasm-fresh:
    node scripts/check-wasm-fresh.mjs

# The two packages split out of the core so that a consumer who only formats or only highlights does not
# download a compiler.
#
# `@guml/fmt` is 178 KB against the full build's 787 KB, because `guml-fmt` sits below the parser — lexer,
# registry and diagnostics, no codegen. Its tests run under `node --test`, which is itself the check: the
# core package's wasm is built for the web target and cannot load in Node at all, and making the formatter
# work there is most of why it is a separate artifact.
#
# `@guml/highlight` has no wasm. Its correctness is `highlight-parity` above, which compares it against the
# Rust classifier span for span.
split-packages-test:
    cd packages/guml-fmt && pnpm typecheck && pnpm test
    cd packages/guml-highlight && pnpm typecheck

# The tree-sitter tag lists must match the registry. They were stale — 8 text tags against a registry
# of 16 — so half the vocabulary's prose lines lexed as piles of identifiers.
check-tags:
    cd editors/tree-sitter-guml && node scripts/gen-tags.mjs --check

# The tree-sitter grammar: the corpus, then a parse of every real `.guml` in the repository. The second
# half is the one to trust — three scanner bugs were live while the corpus passed 12 of 12.
grammar-test:
    cd editors/tree-sitter-guml && npm test

# The example registry package: audit it, typecheck its components, and typecheck the *emitted* component
# against them.
#
# The third one is the check that matters, and it found three compiler bugs the first time it ran. A package
# declares `attrs`, the compiler emits them as props, and until this existed nothing verified the component
# accepted them: `of=revenue` and `kind=line` were silently dropped, the title never reached `aria-label`,
# and a `field`-kind component got its state name as children instead of a two-way binding.
widgets-test:
    cargo run -q -p guml-cli -- registry --validate packages/guml-widgets
    cd packages/guml-widgets && pnpm typecheck && pnpm typecheck:example

# The same three checks against `@guml/shadcn` — 26 tags over the real shadcn/ui components, which are not a
# reimplementation: `shadcn add --all` wrote all 61 files and they are updated in place by the same command.
#
# It earns a second run of the same recipe because the components are *someone else's*, so the emitted props
# are checked against an API this repo does not control. On its first run it found four things: `DatePicker`
# declared in the registry but absent upstream (shadcn ships the date picker as a recipe, not a component),
# `onChange` emitted as a value callback where a raw `<textarea>` wants a React event, `value={n}` where Radix
# wants `number[]`, and a `radio` emitted with no options at all — bound correctly to a state and offering the
# reader no way to change it.
shadcn-test:
    cargo run -q -p guml-cli -- registry --validate packages/guml-shadcn
    cd packages/guml-shadcn && pnpm typecheck && pnpm typecheck:example

# Every package the docs name must exist on npm, and every size they quote must be the size npm reports.
#
# Both failures are invisible to a build: a typo'd name produces a page that compiles, renders, and sends
# readers to a registry 404; a stale size keeps asserting an old number after a republish. The second one
# matters more than it sounds, because the whole argument for splitting `@guml/fmt` and `@guml/highlight`
# out of the core *is* a size comparison.
#
# Network-dependent, so it skips rather than fails when the registry is unreachable.
check-packages:
    cd docs && pnpm check:packages

# The docs site has to build. It is a deployed artifact with three API routes and a playground that runs
# the compiler, and until this was in `ci` a build break only surfaced at deploy time — which is the one
# moment you cannot afford to discover it.
docs-build:
    cd docs && pnpm typecheck && pnpm build

# The VS Code extension. It sat outside the pnpm workspace, so it was the one piece of first-party
# TypeScript with no typecheck — and it is the piece users install.
editor-test:
    cd editors/vscode && pnpm typecheck

# Everything the release workflow does, up to but not including an upload.
#
# Publishing is the one operation here that cannot be undone — a crates.io version can be yanked but
# never replaced, an npm version deprecated but never re-uploaded. So the dry run has to be something a
# person can execute locally before tagging, not a thing that only exists inside CI.
#
# `cargo publish --workspace --dry-run` is the load-bearing one. It computes the topological order
# itself (worth having: the obvious hand-written guess put `guml-syntax` first, and it depends on
# `guml-diagnostics`), and it rejects a path-only internal dependency, which is what crates.io would
# have rejected on the first real upload.
release-dry-run: ci
    cargo publish --workspace --dry-run
    pnpm --filter @guml/core build
    cd packages/guml && npm publish --dry-run --access public
    cd packages/guml-widgets && npm publish --dry-run --access public
    cd packages/guml-shadcn && npm publish --dry-run --access public
    @echo ""
    @echo "Dry run clean. To release: bump the workspace version, add a CHANGELOG entry, then"
    @echo "  git tag v$(cargo metadata --no-deps --format-version 1 | python -c \"import json,sys; print(json.load(sys.stdin)['packages'][0]['version'])\") && git push --tags"

# Compile a fixture and print the emitted React.
demo file="fixtures/a.guml":
    cargo run -q -p guml-cli -- build {{file}}

check file="fixtures/a.guml":
    cargo run -q -p guml-cli -- check {{file}}

# Machine-readable diagnostics — the shape the LLM repair loop consumes.
diagnose file="fixtures/b.guml":
    cargo run -q -p guml-cli -- check {{file}} --format json

ast file="fixtures/a.guml":
    cargo run -q -p guml-cli -- ast {{file}}

lex file="fixtures/a.guml":
    cargo run -q -p guml-cli -- lex {{file}}

# Rough token accounting. Estimates only — see .claude/skills/guml-measure.
tokens:
    cargo run -q -p guml-cli -- tokens fixtures/a.guml fixtures/b.guml fixtures/c.guml

# Full component vocabulary, or a prompt-sized slice: just registry-slice btn,card,list
registry:
    cargo run -q -p guml-cli -- registry

registry-slice tags:
    cargo run -q -p guml-cli -- registry --tags {{tags}}

# Compile every fixture into ./out
build-all:
    cargo run -q -p guml-cli -- build fixtures/a.guml -o out

release:
    cargo build --workspace --release

wasm:
    cargo build -p guml-compiler --target wasm32-unknown-unknown --release

# Format every fixture and example in place.
fmt-guml:
    cargo run -q -p guml-cli -- fmt --write fixtures/*.guml bench/phase0/examples/*.guml bench/guml-bench/reference/*.guml

# Fail if any GUML source is not in formatted form. For CI.
fmt-guml-check:
    cargo run -q -p guml-cli -- fmt --check fixtures/*.guml bench/phase0/examples/*.guml bench/guml-bench/reference/*.guml

# Canonical form: what the benchmark compares. Strips comments, blanks and declaration order.
canonical file="fixtures/b.guml":
    cargo run -q -p guml-cli -- fmt --canonical {{file}}

# Syntax classification from the real lexer and registry.
highlight file="fixtures/b.guml":
    cargo run -q -p guml-cli -- highlight {{file}} --format human

# The site's highlighter must agree with the compiler's, span for span.
highlight-parity:
    cd docs && pnpm check:highlight

# Typecheck the compiler's own emitted TSX under --strict.
# Node rather than bash: the shell version needed `bash`, `TMPDIR` and `wc`, so it simply did not run
# on a Windows checkout — leaving the strongest check on the code generator silently absent there.
typecheck-emitted:
    node scripts/typecheck-emitted.mjs

# Render the emitted components and assert on the HTML — structure, roles, accessible names.
render-emitted:
    node scripts/render-emitted.mjs

# Run the Web Components output. No DOM library: the generated code touches a small, known surface,
# and a shim of it executes `connectedCallback`, every binding and every action for real.
check-wc:
    node scripts/check-wc.mjs

# --- Robustness -------------------------------------------------------------

# The Phase 2 gate: 1M generated documents, no panic, every span inside the source.
#
# Minutes rather than seconds, so it is `#[ignore]`d and not in `ci`. The fast pass in `tests/fuzz.rs`
# runs 26,200 iterations on every push, which is what catches a regression the same day.
#
# This is the *seeded* generator: deterministic and reproducible from its printed seed, but not
# coverage-guided. `just fuzz-guided` is the libFuzzer run, and the two are different claims.
fuzz-long:
    cargo test -p guml-compiler --test fuzz --release -- --ignored --nocapture

# The mutation-recovery gate: definitionally-invalid single-token edits injected into every known-good
# document, measured for detection and for parse desync.
mutation:
    cargo test -p guml-compiler --test mutation -- --nocapture

# Coverage-guided fuzzing. Needs a nightly toolchain and libFuzzer, which is why `fuzz/` sits outside
# the workspace — see its Cargo.toml.
#
#   cargo install cargo-fuzz
fuzz-guided target="parse" seconds="300":
    cd fuzz && cargo +nightly fuzz run {{target}} -- -max_total_time={{seconds}}

# Explain a diagnostic code, or list them all.
explain code="":
    cargo run -q -p guml-cli -- explain {{code}}

# Which GUML line produced a line of emitted code.
where file line:
    cargo run -q -p guml-cli -- where {{file}} {{line}}

# Every token counter side by side. The authoritative column needs ANTHROPIC_API_KEY.
count-tokens:
    node scripts/count-tokens.mjs

# --- Phase 0: the kill-or-continue spike (bench/phase0) ---------------------

# Harness integrity: task set, examples, registry slices, prompt budget, references.
phase0-preflight:
    cd bench/phase0 && node preflight.mjs

# Scoring correctness against synthetic generations. No API key needed.
phase0-selftest:
    cd bench/phase0 && node selftest.mjs

# Assemble every prompt to bench/phase0/results/prompts without calling a model.
phase0-prompts:
    cd bench/phase0 && node run.mjs --dry-run

# The sweep. Needs ANTHROPIC_API_KEY; resumable, one file per run.
phase0-run *ARGS:
    cd bench/phase0 && node run.mjs {{ARGS}}

# Mechanical report, gate check, and the blind human scoresheet.
phase0-score *ARGS:
    cd bench/phase0 && node score.mjs {{ARGS}}

# Everything that can be verified without an API key.
phase0-verify: phase0-preflight phase0-selftest phase0-prompts

# --- GUML-Bench: the Phase 6 benchmark (bench/guml-bench) -------------------

# Is the dataset well-formed, and how big is it really? Fails on a malformed task; prints coverage.
bench-preflight:
    node bench/guml-bench/preflight.mjs

# What can be measured with no API key: authored GUML against emitted output, per category.
bench-report:
    node bench/guml-bench/report.mjs

# The cost of a *change*, against diff-based React editing rather than regeneration.
bench-edits:
    node bench/guml-bench/edit-locality.mjs

# The TOON encoder for arm B4, round-tripped against every payload the report measures.
#
# The rival's arm is the one that most needs a test, because a bug that makes GUML look better is a bug
# nobody reports. Encode, decode, deep-compare — otherwise "TOON is 30% smaller" and "we deleted 30% of
# the characters" are the same claim.
bench-selftest:
    node bench/guml-bench/selftest.mjs

# The part CI can check: the dataset is well-formed, every arm names a backend that exists, and the
# encoder for the competing arm is lossless.
bench-verify: bench-preflight bench-selftest
