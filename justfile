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
ci: fmt-check lint test

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
