"""Python and the Rust CLI must compile every fixture to **byte-identical** output.

# Why this is the most important test in the package

This repository's most-repeated bug is two copies of one thing disagreeing about one document, and it
has landed three separate times:

* Three copies of the *element* mapping meant `nav`/`hero`/`footer` were `<div>` in the static-HTML
  backend where React emitted landmarks — so the no-JavaScript build shipped a page with none.
* Three copies of the *class* table let a theme change reach some backends and not others.
* The *expression* lowerer was reimplemented as a string rewrite in the Web Components backend, and got
  identifiers, string contents and lambda parameters wrong all at once.

Each was found by a test that compared two producers of the same artifact. There are now **three**
bindings over one compiler — the CLI, the wasm, and this — and a Python binding that quietly diverges
would be the fourth instance of exactly that bug.

Every other test here checks that Python returns *something reasonable*. This one checks that it
returns *the same thing*, which is a different and stronger claim: a wrong-but-plausible lowering
passes the first and fails this.

# What it does not cover

The wasm build is not compared here — Node is not a dependency of a Python test suite. `just ci` runs
both this and the JavaScript side against the same fixtures, so the three meet transitively through
the CLI. Skipped entirely when cargo is unavailable, so `pip install guml && pytest` still works for a
contributor with no Rust toolchain; CI has one, and CI is where this has to hold.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import pytest

import guml

REPO = Path(__file__).resolve().parents[3]
FIXTURES = sorted((REPO / "fixtures").glob("*.guml"))

pytestmark = pytest.mark.skipif(
    shutil.which("cargo") is None or not FIXTURES,
    reason="needs the Rust toolchain and the repository fixtures",
)


def cli(*args: str) -> str:
    """Run the real CLI and return stdout, normalising line endings.

    CRLF normalisation is not papering over a difference: the CLI writes through a Windows stdout that
    translates `\\n`, while PyO3 hands the string back untouched. That is a property of the pipe, not
    of the compiler, and comparing it would fail on Windows for a reason that has nothing to do with
    codegen.
    """
    result = subprocess.run(
        ["cargo", "run", "-q", "-p", "guml-cli", "--", *args],
        cwd=REPO,
        capture_output=True,
        text=True,
        # **Explicit UTF-8, not the locale default.** On Windows `text=True` decodes with cp1252, which
        # turns every em dash the compiler emits into U+FFFD. The first run of this test "failed" on
        # six fixtures for that reason alone — a diff that looks exactly like a real divergence and is
        # entirely an artefact of how the pipe was read.
        encoding="utf-8",
    )
    if result.returncode != 0:
        pytest.fail(f"cli {' '.join(args)} failed:\n{result.stderr}")
    return result.stdout.replace("\r\n", "\n")


@pytest.mark.parametrize("fixture", FIXTURES, ids=lambda p: p.name)
@pytest.mark.parametrize("backend", ["react", "html", "svelte", "json", "wc"])
def test_python_and_the_cli_emit_identical_bytes(fixture: Path, backend: str):
    source = fixture.read_text(encoding="utf-8")

    from_cli = cli("build", f"fixtures/{fixture.name}", "--backend", backend)
    from_python = guml.compile(source, backend).contents

    assert from_python == from_cli, (
        f"{fixture.name} via `{backend}` differs between the Python binding and the CLI.\n"
        "Two producers of one artifact have diverged — that is the bug class this test exists for."
    )


@pytest.mark.parametrize("fixture", FIXTURES, ids=lambda p: p.name)
def test_python_and_the_cli_agree_on_formatting(fixture: Path):
    source = fixture.read_text(encoding="utf-8")
    assert guml.format(source) == cli("fmt", f"fixtures/{fixture.name}")


@pytest.mark.parametrize("fixture", FIXTURES, ids=lambda p: p.name)
def test_python_and_the_cli_agree_on_canonical_form(fixture: Path):
    source = fixture.read_text(encoding="utf-8")
    assert guml.canonical(source) == cli("fmt", "--canonical", f"fixtures/{fixture.name}")


@pytest.mark.parametrize("fixture", FIXTURES, ids=lambda p: p.name)
def test_python_and_the_cli_agree_on_diagnostics(fixture: Path):
    """Codes and their order. The repair loop keys on codes, so a binding that reports a different set
    would send a model to fix something the CLI never complained about."""
    import json

    source = fixture.read_text(encoding="utf-8")
    raw = cli("check", f"fixtures/{fixture.name}", "--format", "json").strip()

    # A bare array, and `[]` for a clean document — it used to print nothing at all, which is not
    # valid JSON, so every consumer had to special-case the one case they most need to get right.
    # Parsed unconditionally now, which is itself the assertion that it stayed fixed.
    from_cli = [d["id"] for d in json.loads(raw)]
    from_python = [d.code for d in guml.check(source)]
    assert from_python == from_cli


def test_the_wheel_version_matches_the_workspace():
    """Three registries now publish from one repository. A wheel that says 0.1.0 while the crates say
    0.2.0 is the kind of thing nobody notices until someone files a bug against the wrong version."""
    cargo = (REPO / "Cargo.toml").read_text(encoding="utf-8")
    workspace_version = next(
        line.split('"')[1]
        for line in cargo.splitlines()
        if line.startswith("version = ")
    )
    assert guml.__version__ == workspace_version
