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

# Everything CI runs.
ci: fmt-check lint test fmt-guml-check

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
    cargo run -q -p guml-cli -- fmt --write fixtures/*.guml bench/phase0/examples/*.guml

# Fail if any GUML source is not in formatted form. For CI.
fmt-guml-check:
    cargo run -q -p guml-cli -- fmt --check fixtures/*.guml bench/phase0/examples/*.guml

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
typecheck-emitted:
    bash scripts/typecheck-emitted.sh

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
