"""``guml`` on the command line, from ``pip install guml``.

**Not a port of the Rust CLI.** It is a small shim over the same compiler, covering what someone who
reached for pip actually wants: compile a file, check it, format it, ask what a document will do. The
full CLI — source maps, custom themes, registry validation, the token estimator — is
``cargo install guml-cli``, and this says so rather than pretending to be it.

The reason to ship it at all is that a Python developer wanting to compile a ``.guml`` file should not
need a Rust toolchain to do it.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import guml


def _read(path: str) -> str:
    if path == "-":
        return sys.stdin.read()
    try:
        return Path(path).read_text(encoding="utf-8")
    except OSError as e:
        sys.exit(f"guml: cannot read {path}: {e}")


def _report(diagnostics: list[guml.Diagnostic], *, source: str) -> int:
    """Print diagnostics the way a compiler should: the line, and a caret under the span."""
    lines = source.splitlines()
    for d in diagnostics:
        print(f"{d.severity}[{d.code}] {d.message}", file=sys.stderr)
        if 1 <= d.line <= len(lines):
            print(f"  {d.line:>4} | {lines[d.line - 1]}", file=sys.stderr)
            print(f"       | {' ' * max(0, d.column - 1)}^", file=sys.stderr)
        if d.help:
            print(f"       = help: {d.help}", file=sys.stderr)
    return 1 if any(d.is_error for d in diagnostics) else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="guml",
        description="Compile GUML. A subset of the full CLI — `cargo install guml-cli` for all of it.",
    )
    parser.add_argument("--version", action="version", version=f"guml {guml.__version__}")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_build = sub.add_parser("build", help="compile a document")
    p_build.add_argument("file", help="a .guml file, or - for stdin")
    p_build.add_argument("-b", "--backend", default="react", choices=list(guml.BACKENDS))
    p_build.add_argument(
        "--level",
        default="app",
        choices=["core", "app"],
        help="core = markup only: no state, data, actions or js. Use it for documents you did not write.",
    )
    p_build.add_argument("-o", "--out", help="write files here instead of stdout")

    p_check = sub.add_parser("check", help="parse and analyse, reporting every problem in one pass")
    p_check.add_argument("file")
    p_check.add_argument("--level", default="app", choices=["core", "app"])

    p_fmt = sub.add_parser("fmt", help="format a document")
    p_fmt.add_argument("file")
    p_fmt.add_argument("--write", action="store_true", help="rewrite the file in place")
    p_fmt.add_argument("--check", action="store_true", help="exit non-zero if it is not formatted")
    p_fmt.add_argument("--canonical", action="store_true", help="strip comments and sort directives")

    p_caps = sub.add_parser("capabilities", help="what a document will do, and a CSP for it")
    p_caps.add_argument("file")
    p_caps.add_argument("-b", "--backend", default="html")

    args = parser.parse_args(argv)
    source = _read(args.file)

    if args.cmd == "check":
        diagnostics = guml.check(source, level=args.level)
        if not diagnostics:
            print("no problems found")
            return 0
        return _report(diagnostics, source=source)

    if args.cmd == "fmt":
        out = guml.canonical(source) if args.canonical else guml.format(source)
        if args.check:
            if out != source:
                print(f"guml: {args.file} is not formatted", file=sys.stderr)
                return 1
            return 0
        if args.write:
            if args.file == "-":
                sys.exit("guml: --write needs a real file, not stdin")
            Path(args.file).write_text(out, encoding="utf-8")
            return 0
        sys.stdout.write(out)
        return 0

    if args.cmd == "capabilities":
        caps = guml.capabilities(source, args.backend)
        print(f"level          : {caps.level}")
        print(f"js / raw       : {caps.js_blocks} / {caps.raw_blocks}")
        print(f"network        : {', '.join(caps.network) or 'none'}")
        print(f"storage        : {'yes' if caps.storage else 'no'}")
        print(f"script         : {'yes' if caps.script else 'no'}")
        print(f"csp            : {caps.csp}")
        return 0

    # build
    result = guml.compile(source, args.backend, level=args.level)
    status = _report(result.diagnostics, source=source) if result.diagnostics else 0
    if status:
        return status

    if args.out:
        directory = Path(args.out)
        directory.mkdir(parents=True, exist_ok=True)
        for f in result.files:
            (directory / f.path).write_text(f.contents, encoding="utf-8")
            print(f"wrote {directory / f.path}")
        return 0

    for f in result.files:
        if len(result.files) > 1:
            print(f"// ---- {f.path} ----")
        sys.stdout.write(f.contents)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
