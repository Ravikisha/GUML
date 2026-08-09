"""GUML — an intermediate representation and compiler for LLM-generated user interfaces.

Two things this package is for, and they are different enough to describe separately.

**Generating UI with a model.** Put :data:`SPEC` and a :func:`registry` slice in a system prompt, get
GUML back, then :func:`check` it, :func:`repair` what is mechanically fixable without another model
call, and :func:`render` the result. The whole point of GUML is that a model writes ~80% fewer tokens
and the compiler supplies the class names, the ARIA plumbing and the loading states — so the
correctness surface is much smaller than "please emit correct React".

    >>> import guml
    >>> src = 'page "Hi"\\ncard\\n  h Hello\\n'
    >>> guml.check(src)
    []
    >>> "<h2" in guml.render(src)
    True

**Serving pages from Flask, FastAPI or Django.** :func:`render` turns GUML into HTML with no
JavaScript and no build step. See the README for per-framework snippets; they are three lines each.

Security
--------
``js`` and ``raw`` blocks compile through **unchanged** — that is GUML's documented escape hatch and
its security boundary. So :func:`render` defaults to ``level="core"``: markup only, no state, no
data, no actions, no ``js``. Rendering a model's output at ``level="app"`` executes whatever
JavaScript that model wrote, in your users' browsers.

This is a deliberate divergence from the CLI, whose default is ``app``. Different threat model:
``guml build`` compiles a file you wrote, and :func:`render` very often does not.
"""

from __future__ import annotations

import json as _json
from dataclasses import dataclass, field
from typing import Any, Literal

from . import _guml

__all__ = [
    "SPEC",
    "BACKENDS",
    "Diagnostic",
    "CompileResult",
    "Capabilities",
    "Repaired",
    "GumlError",
    "check",
    "compile",
    "render",
    "format",
    "canonical",
    "fix",
    "repair",
    "capabilities",
    "registry",
    "stylesheet",
    "__version__",
]

__version__: str = _guml.version()

#: The language specification — rules, not vocabulary — sized to sit in a system prompt.
#: Pair it with a :func:`registry` slice, which carries the tags.
SPEC: str = _guml.spec()

#: Every backend name the compiler can resolve, from its own registry of backends rather than a
#: second list here that could drift.
BACKENDS: tuple[str, ...] = tuple(_guml.backends())

Level = Literal["core", "app"]
Style = Literal["inline", "cdn", "none"]


class GumlError(Exception):
    """Raised when a document does not compile.

    Carries every diagnostic, not just the first — ``check`` collects them in one pass, and a repair
    loop that fixes one error per round pays for a full model generation each time.
    """

    def __init__(self, diagnostics: list[Diagnostic]) -> None:
        self.diagnostics = diagnostics
        errors = [d for d in diagnostics if d.severity == "error"]
        head = errors[0] if errors else diagnostics[0]
        extra = f" (+{len(errors) - 1} more)" if len(errors) > 1 else ""
        super().__init__(f"{head.code} line {head.line}: {head.message}{extra}")


@dataclass(frozen=True, slots=True)
class Diagnostic:
    """One problem with a document.

    ``code`` is stable forever: diagnostic codes are append-only and never renumbered, precisely so a
    repair loop can key on them.
    """

    code: str
    severity: Literal["error", "warning", "info"]
    message: str
    line: int
    column: int
    start: int
    end: int
    help: str | None = None
    #: Literal replacement text for ``start:end``. Present only when the fix is unambiguous, which is
    #: what lets :func:`fix` apply it with no model call.
    suggestion: str | None = None

    @property
    def is_error(self) -> bool:
        return self.severity == "error"

    def __str__(self) -> str:
        return f"{self.code} line {self.line}:{self.column} {self.message}"


@dataclass(frozen=True, slots=True)
class OutFile:
    path: str
    contents: str


@dataclass(frozen=True, slots=True)
class CompileResult:
    files: list[OutFile]
    diagnostics: list[Diagnostic]

    @property
    def ok(self) -> bool:
        return not any(d.is_error for d in self.diagnostics)

    @property
    def contents(self) -> str:
        """The first file's contents — the common case, where a document compiles to one file."""
        if not self.files:
            raise ValueError("nothing was emitted; check `diagnostics`")
        return self.files[0].contents


@dataclass(frozen=True, slots=True)
class Capabilities:
    """What a document will actually do, and a Content-Security-Policy that permits exactly that."""

    #: A policy allowing what this document needs and nothing else. Put it in a response header.
    csp: str
    #: Conformance level the document actually requires: ``"core"`` or ``"app"``.
    level: str = "core"
    #: Origins it will contact. Empty means it makes no requests at all.
    network: list[str] = field(default_factory=list)
    #: Every request it will issue, with method and URL.
    requests: list[dict[str, Any]] = field(default_factory=list)
    #: Whether it touches local/session storage.
    storage: bool = False
    #: Whether it needs script execution at all.
    script: bool = False
    #: Whether it emits unescaped markup via ``raw``.
    raw_markup: bool = False
    #: ``js`` block count.
    js_blocks: int = 0
    #: ``raw`` block count.
    raw_blocks: int = 0
    #: Share of the document's lines inside an escape hatch. A rising number across a corpus is the
    #: early warning that the vocabulary is hitting an expressiveness limit.
    escape_share: float = 0.0
    #: The whole manifest, for anything not surfaced above.
    raw: dict[str, Any] = field(default_factory=dict)

    @property
    def uses_escape_hatch(self) -> bool:
        """True if the document contains a ``js`` or ``raw`` block.

        The one predicate worth branching on before rendering something a model wrote at
        ``level="app"``, because those blocks compile through unchanged.
        """
        return self.js_blocks > 0 or self.raw_blocks > 0


@dataclass(frozen=True, slots=True)
class Repaired:
    text: str
    applied: list[str]
    rounds: int
    reformatted: bool = False

    @property
    def changed(self) -> bool:
        return bool(self.applied) or self.reformatted


def _diags(raw: str) -> list[Diagnostic]:
    return [Diagnostic(**d) for d in _json.loads(raw)]


def check(source: str, *, level: Level = "app") -> list[Diagnostic]:
    """Parse and analyse. Returns **every** problem, never just the first.

    Non-raising by design, because the common caller is a repair loop that wants the list. Use
    :func:`raise_for_errors` — or just ``if any(d.is_error for d in ...)`` — when you want an
    exception instead.
    """
    return _diags(_guml.check(source, level))


def raise_for_errors(source: str, *, level: Level = "app") -> None:
    """:func:`check`, but raise :class:`GumlError` if anything is an error."""
    diagnostics = check(source, level=level)
    if any(d.is_error for d in diagnostics):
        raise GumlError(diagnostics)


def compile(source: str, backend: str = "react", *, level: Level = "app") -> CompileResult:  # noqa: A001
    """Compile to one of :data:`BACKENDS`.

    Shadows the builtin ``compile``, deliberately: ``guml.compile`` reads correctly at the call site,
    and the builtin is not something a caller of this module is reaching for. Import it as
    ``from guml import compile as guml_compile`` if that bothers you.
    """
    raw = _json.loads(_guml.compile(source, backend, level))
    return CompileResult(
        files=[OutFile(**f) for f in raw["files"]],
        diagnostics=[Diagnostic(**d) for d in raw["diagnostics"]],
    )


def stylesheet() -> str:
    """The active theme's stylesheet.

    Fragments deliberately carry no styles — a site with fifty fragments wants one copy of the CSS in
    its layout, not fifty beside the content. This is how you get that copy:

        # once, at build time or on startup
        Path("static/guml.css").write_text(guml.stylesheet())
    """
    return _guml.stylesheet()


def render(
    source: str,
    *,
    level: Level = "core",
    style: Style | None = None,
    fragment: bool = False,
) -> str:
    """Render GUML to HTML. No JavaScript, no build step, nothing to serve alongside it.

    :param level: ``"core"`` by default — markup only, no ``state``, ``data``, actions or ``js``.
        **This differs from the CLI**, which defaults to ``app``, and the reason is that the usual
        caller here is a web server rendering a document a model wrote. Pass ``level="app"`` when the
        document is yours; understand that it will execute any ``js`` the author included.
    :param style: ``"inline"`` embeds the theme stylesheet, giving a self-contained document.
        ``"none"`` emits classes only, for a host that already runs Tailwind. ``"cdn"`` is a preview
        convenience and a runtime dependency on a third party.
    :param fragment: content only — no ``<!doctype>``, no ``<head>``, and no ``<main>``. For a Jinja
        include or an htmx swap target. The missing ``<main>`` is the point rather than an oversight:
        a document may hold exactly one ``main`` landmark, so a fragment carrying its own would create
        a second the moment it were embedded.

    ``style`` defaults to what the shape implies: ``"inline"`` for a whole document, which should be
    self-contained, and ``"none"`` for a fragment, which cannot be. Asking for ``style="inline"`` on a
    fragment is a contradiction and raises — a fragment has no ``<head>`` to put a stylesheet in, and
    returning unstyled markup as though the request had been honoured is the failure mode this
    compiler refuses everywhere else. Use :func:`stylesheet` and put it in your layout once.
    """
    if style is None:
        style = "none" if fragment else "inline"

    if fragment and style != "none":
        raise ValueError(
            f"a fragment cannot carry {style!r} styling: it has no <head> to put it in. "
            "Use guml.stylesheet() and include it once in the surrounding page, or drop "
            "fragment=True for a self-contained document."
        )

    backend = {
        (False, "inline"): "html",
        (False, "cdn"): "html-cdn",
        (False, "none"): "html-bare",
        (True, "none"): "html-fragment",
    }.get((fragment, style))

    if backend is None:
        raise ValueError(f'style must be "inline", "cdn" or "none", not {style!r}')

    result = compile(source, backend, level=level)
    errors = [d for d in result.diagnostics if d.is_error]
    if errors:
        raise GumlError(result.diagnostics)
    return result.contents


def format(source: str) -> str:  # noqa: A001
    """Format. Idempotent, and comments and blank lines survive."""
    return _guml.format(source)


def canonical(source: str) -> str:
    """Canonical form: comments and blank lines stripped, directives hoisted and sorted.

    Two documents that *mean* the same thing come out byte-identical, which is what makes two
    independent generations of one interface comparable. It deletes commentary on purpose, so it is a
    normaliser and never a formatter for an editor.
    """
    return _guml.canonical(source)


def fix(source: str, *, max_rounds: int = 3) -> Repaired:
    """Apply every unambiguous suggestion, re-checking until nothing changes. No model call."""
    raw = _json.loads(_guml.fix(source, max_rounds))
    return Repaired(text=raw["text"], applied=raw["codes"], rounds=raw["rounds"])


def repair(source: str, *, max_rounds: int = 3) -> Repaired:
    """Everything a repair loop can do without asking a model again.

    Sanitise (strip a markdown fence a model wrapped the document in, and similar), format, then
    :func:`fix`. Run this *before* spending a round on the model — it is free, and it resolves a
    surprising share of what a first generation gets wrong.
    """
    raw = _json.loads(_guml.repair(source, max_rounds))
    return Repaired(
        text=raw["text"],
        applied=raw["applied"],
        rounds=raw["rounds"],
        reformatted=raw["reformatted"],
    )


def capabilities(source: str, backend: str = "html") -> Capabilities:
    """What the document will do, and a CSP permitting exactly that and nothing else.

    ``backend`` matters: the policy is a property of the *output*. The static-HTML backend inlines the
    theme stylesheet, so its policy needs ``style-src 'unsafe-inline'`` and says why.
    """
    raw = _json.loads(_guml.capabilities(source, backend))
    escapes = raw.get("escapes") or {}
    return Capabilities(
        csp=raw["csp"],
        level=raw.get("level", "core"),
        network=raw.get("network") or [],
        requests=raw.get("requests") or [],
        storage=bool(raw.get("storage")),
        script=bool(raw.get("script")),
        raw_markup=bool(raw.get("rawMarkup")),
        js_blocks=int(escapes.get("js", 0)),
        raw_blocks=int(escapes.get("raw", 0)),
        escape_share=float(escapes.get("shareOfLines", 0.0)),
        raw=raw,
    )


def registry(tags: list[str] | None = None) -> str:
    """The component vocabulary, or a prompt-sized slice of it.

    Pass the tags a task plausibly needs. The slice is the mechanism that keeps an assembled prompt
    under budget: a document uses a dozen tags, not the whole registry.

        >>> prompt = guml.SPEC + "\\n\\n" + guml.registry(["card", "btn", "list"])
    """
    return _guml.registry(tags)
