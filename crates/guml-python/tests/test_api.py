"""The Python API surface.

The cross-language agreement test lives in `test_agreement.py`, and it is the one that matters most —
see that file for why.
"""

from __future__ import annotations

import pytest

import guml

# Already in formatted form. The blank line after the `page` directive is what the formatter emits, and
# `test_repair_leaves_a_clean_document_alone` only means anything if the input is genuinely clean.
CLEAN = 'page "Demo"\n\ncard\n  h Revenue\n  p Up 12%\n'
APP_LEVEL = 'page "Demo"\nstate n: 0\ncard\n  h Hi\n'
BROKEN = 'page "X"\ncrad Hi\n'


# --------------------------------------------------------------------------- check


def test_check_returns_nothing_for_a_clean_document():
    assert guml.check(CLEAN) == []


def test_check_reports_every_problem_in_one_pass():
    # Not "reports a problem" — *every* problem. A repair loop pays a full model generation per round,
    # so reporting one error at a time multiplies the cost of fixing a document by its mistake count.
    src = 'page "X"\ncrad Hi\nbtn Go\nfoo Bar\n'
    codes = [d.code for d in guml.check(src)]
    assert len(codes) >= 2, codes


def test_a_diagnostic_is_a_dataclass_not_a_dict():
    d = guml.check(BROKEN)[0]
    assert d.code.startswith("GUML")
    assert d.line == 2
    assert d.is_error
    assert "crad" in d.message
    # Unambiguous fixes carry replacement text — that is what lets `fix` work with no model call.
    assert d.suggestion == "card"
    with pytest.raises(AttributeError):
        d.nonexistent


def test_raise_for_errors_raises_and_carries_everything():
    with pytest.raises(guml.GumlError) as excinfo:
        guml.raise_for_errors(BROKEN)
    assert excinfo.value.diagnostics
    assert "GUML" in str(excinfo.value)


def test_check_is_not_raising():
    """The common caller is a repair loop that wants the list, not an exception."""
    assert guml.check(BROKEN)  # no raise


# --------------------------------------------------------------------------- levels


def test_core_level_refuses_app_constructs():
    assert any(d.is_error for d in guml.check(APP_LEVEL, level="core"))
    assert not any(d.is_error for d in guml.check(APP_LEVEL, level="app"))


def test_render_defaults_to_core():
    """The security default, and the one deliberate divergence from the CLI.

    `js` and `raw` compile through unchanged, so a server rendering a model's output must not default
    to executing it. If this ever flips, a Flask app rendering LLM output starts running that model's
    JavaScript in its users' browsers, and nothing else here would notice.
    """
    with pytest.raises(guml.GumlError):
        guml.render(APP_LEVEL)
    assert guml.render(APP_LEVEL, level="app")


def test_an_unknown_level_is_rejected_with_a_useful_message():
    with pytest.raises(ValueError, match="core"):
        guml.check(CLEAN, level="strict")


# --------------------------------------------------------------------------- render


def test_render_produces_a_whole_document():
    html = guml.render(CLEAN)
    assert html.startswith("<!doctype html>")
    assert "<title>" in html
    assert "Revenue" in html


def test_render_needs_no_javascript():
    """The selling point for a Python server: nothing to serve alongside it."""
    assert "<script" not in guml.render(CLEAN)


def test_a_fragment_carries_no_document_furniture():
    frag = guml.render(CLEAN, fragment=True)
    for forbidden in ("<!doctype", "<html", "<head", "<body", "<main"):
        assert forbidden not in frag, f"a fragment must not carry `{forbidden}`"
    assert "Revenue" in frag
    # `<main>` is absent on purpose: a document may hold exactly one, so a fragment carrying its own
    # would create a second the moment a template embedded it.


def test_style_none_emits_classes_without_a_stylesheet():
    out = guml.render(CLEAN, style="none")
    assert "<style>" not in out
    assert "rounded-xl" in out  # the classes are still there for the host's pipeline


def test_an_unknown_style_is_rejected():
    with pytest.raises(ValueError, match="style"):
        guml.render(CLEAN, style="tailwind")


def test_render_raises_rather_than_returning_broken_html():
    with pytest.raises(guml.GumlError):
        guml.render(BROKEN)


# --------------------------------------------------------------------------- compile


def test_compile_reaches_every_advertised_backend():
    for backend in guml.BACKENDS:
        result = guml.compile(CLEAN, backend)
        assert result.files, f"{backend} emitted nothing"
        assert result.contents.strip()


def test_an_unknown_backend_names_the_real_ones():
    with pytest.raises(ValueError) as excinfo:
        guml.compile(CLEAN, "vue")
    assert "react" in str(excinfo.value)


def test_compile_result_reports_ok():
    assert guml.compile(CLEAN, "react").ok
    assert not guml.compile(BROKEN, "react").ok


# --------------------------------------------------------------------------- format


def test_format_is_idempotent():
    once = guml.format('page   "X"\n\n\ncard\n     h  Hi\n')
    assert guml.format(once) == once


def test_canonical_makes_equivalent_documents_identical():
    a = 'page "X"\ncard\n  h Hi\n'
    b = '// a comment\npage   "X"\n\n\ncard\n     h  Hi\n'
    assert guml.canonical(a) == guml.canonical(b)


def test_format_keeps_what_canonical_strips():
    src = '// keep me\npage "X"\ncard\n  h Hi\n'
    assert "// keep me" in guml.format(src)
    assert "// keep me" not in guml.canonical(src)


# --------------------------------------------------------------------------- repair


def test_repair_strips_a_markdown_fence():
    """Models wrap output in fences. Handling that costs nothing and saves a whole round."""
    fenced = '```guml\npage "X"\ncard\n  h Hi\n```\n'
    out = guml.repair(fenced)
    assert "```" not in out.text
    assert out.changed


def test_fix_applies_unambiguous_suggestions_with_no_model_call():
    out = guml.fix(BROKEN)
    assert "card" in out.text
    assert "crad" not in out.text
    assert out.applied


def test_repair_leaves_a_clean_document_alone():
    assert guml.repair(CLEAN).text == CLEAN


# --------------------------------------------------------------------------- capabilities


def test_capabilities_reports_no_behaviour_for_static_markup():
    caps = guml.capabilities(CLEAN)
    assert not caps.uses_escape_hatch
    assert not caps.script
    assert caps.network == []
    assert "script-src 'none'" in caps.csp


def test_capabilities_sees_an_escape_hatch():
    caps = guml.capabilities('page "X"\njs\n  console.log(1)\n')
    assert caps.uses_escape_hatch
    assert caps.js_blocks == 1
    assert caps.script


def test_the_csp_reflects_the_backend_not_just_the_source():
    """The policy is a property of the output. The html backend inlines the theme, so it needs
    `style-src 'unsafe-inline'` and says why; a backend that does not, does not."""
    assert "unsafe-inline" in guml.capabilities(CLEAN, "html").csp


# --------------------------------------------------------------------------- registry & spec


def test_registry_with_no_arguments_returns_the_whole_vocabulary():
    """Regression: this returned an empty string, because the slice helper iterates the names it is
    given and was being handed none. A caller building a system prompt would have described no tags at
    all — a well-formed prompt that teaches the model nothing."""
    full = guml.registry()
    assert len(full.splitlines()) > 40
    assert "card" in full


def test_registry_slices_to_the_tags_asked_for():
    sliced = guml.registry(["card", "btn"])
    assert len(sliced.splitlines()) == 2
    assert len(sliced) < len(guml.registry())


def test_spec_is_embedded_and_prompt_sized():
    assert len(guml.SPEC) > 1000, "the spec must travel inside the wheel"
    # The budget is ≤3,000 tokens; ~3.6 chars/token puts the ceiling near 11k characters. This is a
    # smoke test that the file is the spec and not something else, not a token measurement.
    assert len(guml.SPEC) < 20_000


def test_version_is_reported():
    assert guml.__version__.count(".") == 2


# --------------------------------------------------------------------------- fragments and styling


def test_a_fragment_refuses_inline_styling_rather_than_dropping_it():
    """Regression. `render(fragment=True, style="inline")` used to return an unstyled fragment: the
    style argument was mapped to a backend that ignores it, so the request was silently not honoured.

    Silently-wrong output is the one thing this compiler refuses everywhere else, and a caller who
    asked for styling and got none has no way to tell from the return value."""
    with pytest.raises(ValueError, match="no <head>"):
        guml.render(CLEAN, fragment=True, style="inline")


def test_style_defaults_to_what_the_shape_implies():
    # A document should be self-contained; a fragment cannot be.
    assert "<style>" in guml.render(CLEAN)
    assert "<style>" not in guml.render(CLEAN, fragment=True)


def test_stylesheet_is_reachable_for_the_layout_to_include():
    css = guml.stylesheet()
    assert len(css) > 1000
    # Not `--background`: that is a shadcn token, and asserting it pinned the default theme into a test
    # about whether the stylesheet is *reachable*. The property is that it declares the classes the
    # compiler emits, whichever theme is active.
    assert ".rounded-xl" in css or "rounded-xl" in css, "expected rules for the emitted classes"
