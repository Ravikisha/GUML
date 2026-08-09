"""The HTML this package produces must actually be HTML.

`test_api.py` checks that `render()` returns a string containing the words you expect. That is a
weaker claim than it looks: an unclosed tag, a duplicated `id`, a `<div>` inside a `<p>` or a stray
`</main>` all pass a substring assertion and all break a real page.

The JavaScript side has had `render-emitted.mjs` doing this for a while and it caught a dormant
assertion nothing else saw. This is the same idea against the Python path, using only the standard
library so the suite stays dependency-free.

Not a validator. `html.parser` is lenient by design and will happily accept things a browser merely
tolerates. What it does catch is structural nonsense, unbalanced tags, and the accessibility rules
that are cheap to state and expensive to get wrong — which is most of what a code generator gets
wrong.
"""

from __future__ import annotations

from html.parser import HTMLParser
from pathlib import Path

import pytest

import guml

REPO = Path(__file__).resolve().parents[3]
FIXTURES = sorted((REPO / "fixtures").glob("*.guml"))

# Elements with no closing tag. An unbalanced-tag check that does not know these reports every `<meta>`
# as an error.
VOID = {
    "area", "base", "br", "col", "embed", "hr", "img", "input",
    "link", "meta", "param", "source", "track", "wbr",
}


class Structure(HTMLParser):
    """Tracks tag balance, ids, landmarks and the attributes that carry accessibility."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.stack: list[str] = []
        self.errors: list[str] = []
        self.ids: list[str] = []
        self.tags: list[str] = []
        self.buttons_without_text: int = 0
        self.images_without_alt: int = 0
        self._open_button_has_text = True

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self.tags.append(tag)
        attr = dict(attrs)

        if "id" in attr and attr["id"]:
            self.ids.append(attr["id"])
        if tag == "img" and not attr.get("alt") and attr.get("alt") != "":
            self.images_without_alt += 1
        if tag == "button":
            self._open_button_has_text = bool(attr.get("aria-label"))

        if tag not in VOID:
            self.stack.append(tag)

    def handle_endtag(self, tag: str) -> None:
        if tag in VOID:
            return
        if tag == "button" and not self._open_button_has_text:
            self.buttons_without_text += 1
        if not self.stack:
            self.errors.append(f"</{tag}> with nothing open")
            return
        if self.stack[-1] != tag:
            self.errors.append(f"</{tag}> closes <{self.stack[-1]}>")
            return
        self.stack.pop()

    def handle_data(self, data: str) -> None:
        if data.strip() and self.stack and self.stack[-1] == "button":
            self._open_button_has_text = True


def parse(html: str) -> Structure:
    s = Structure()
    s.feed(html)
    s.close()
    return s


@pytest.mark.skipif(not FIXTURES, reason="needs the repository fixtures")
@pytest.mark.parametrize("fixture", FIXTURES, ids=lambda p: p.name)
def test_every_fixture_renders_to_balanced_html(fixture: Path):
    html = guml.render(fixture.read_text(encoding="utf-8"), level="app")
    s = parse(html)
    assert not s.errors, f"{fixture.name}: {s.errors[:3]}"
    assert not s.stack, f"{fixture.name}: never closed {s.stack}"


@pytest.mark.skipif(not FIXTURES, reason="needs the repository fixtures")
@pytest.mark.parametrize("fixture", FIXTURES, ids=lambda p: p.name)
def test_every_fixture_renders_a_well_formed_document(fixture: Path):
    html = guml.render(fixture.read_text(encoding="utf-8"), level="app")
    assert html.startswith("<!doctype html>")
    s = parse(html)
    for required in ("html", "head", "title", "body", "main"):
        assert required in s.tags, f"{fixture.name}: no <{required}>"
    # Exactly one `main` landmark. More than one is a real accessibility fault and the reason the
    # fragment backend emits none.
    assert s.tags.count("main") == 1, f"{fixture.name}: {s.tags.count('main')} <main> elements"
    assert "lang" in html[:120], "a document without `lang` makes a screen reader guess pronunciation"


@pytest.mark.skipif(not FIXTURES, reason="needs the repository fixtures")
@pytest.mark.parametrize("fixture", FIXTURES, ids=lambda p: p.name)
def test_ids_are_unique(fixture: Path):
    """Duplicate ids break `for`/`id` label association and every anchor link on the page, and a
    generator producing them per repeated element is the classic way it happens."""
    s = parse(guml.render(fixture.read_text(encoding="utf-8"), level="app"))
    duplicates = {i for i in s.ids if s.ids.count(i) > 1}
    assert not duplicates, f"{fixture.name}: duplicate ids {sorted(duplicates)}"


@pytest.mark.skipif(not FIXTURES, reason="needs the repository fixtures")
@pytest.mark.parametrize("fixture", FIXTURES, ids=lambda p: p.name)
def test_accessible_names_and_alt_text(fixture: Path):
    """The rules the compiler claims to own. It supplies `aria-label` and `alt` precisely so an author
    cannot forget them — so if either is ever missing, the compiler broke its own promise."""
    s = parse(guml.render(fixture.read_text(encoding="utf-8"), level="app"))
    assert s.buttons_without_text == 0, f"{fixture.name}: {s.buttons_without_text} unnamed button(s)"
    assert s.images_without_alt == 0, f"{fixture.name}: {s.images_without_alt} image(s) without alt"


@pytest.mark.skipif(not FIXTURES, reason="needs the repository fixtures")
@pytest.mark.parametrize("fixture", FIXTURES, ids=lambda p: p.name)
def test_fragments_are_balanced_and_carry_no_document_furniture(fixture: Path):
    frag = guml.render(fixture.read_text(encoding="utf-8"), level="app", fragment=True)
    s = parse(frag)
    assert not s.errors, f"{fixture.name}: {s.errors[:3]}"
    assert not s.stack, f"{fixture.name}: never closed {s.stack}"
    for forbidden in ("html", "head", "body", "main", "title"):
        assert forbidden not in s.tags, f"{fixture.name}: a fragment carries <{forbidden}>"


def test_the_static_build_ships_no_javascript():
    """The whole selling point for a Python server: nothing to serve alongside the page."""
    for fixture in FIXTURES:
        html = guml.render(fixture.read_text(encoding="utf-8"), level="app")
        assert "<script" not in html, f"{fixture.name} emitted a script tag"
