"""A Jinja2 extension, so GUML is a filter in your templates.

Shipped because it is the one framework integration that is genuinely annoying to write yourself, and
because Flask, Django and FastAPI-with-templates all sit on Jinja — one extension covers all three.
Everything else is a three-line snippet the README shows instead of a dependency this package would
have to track forever.

    from guml.jinja import GumlExtension
    app.jinja_env.add_extension(GumlExtension)

    {# fragment=True by default: no doctype, no <head>, no <main> — it is going inside a page you
       already own #}
    <div class="panel">{{ source | guml }}</div>

    {# a whole page, self-contained #}
    {{ source | guml(fragment=False) }}

Two things this does that a hand-written filter usually gets wrong:

**The output is marked safe.** Jinja escapes strings by default, so a filter returning plain ``str``
renders ``&lt;div&gt;`` — the HTML shows up as visible text. :class:`~markupsafe.Markup` says the
compiler already produced HTML.

**``level="core"`` is the default.** ``js`` and ``raw`` compile through unchanged, so rendering a
model's output at ``app`` level runs its JavaScript in your users' browsers. A template filter is
exactly where that would be reached for without thinking about it.

Requires ``pip install guml[jinja]``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Literal

import guml

if TYPE_CHECKING:  # pragma: no cover
    from jinja2 import Environment

try:
    from jinja2.ext import Extension
    from markupsafe import Markup
except ImportError as e:  # pragma: no cover
    raise ImportError(
        "guml.jinja needs Jinja2. Install it with: pip install 'guml[jinja]'"
    ) from e


def guml_filter(
    source: str,
    *,
    fragment: bool = True,
    level: Literal["core", "app"] = "core",
    style: Literal["inline", "cdn", "none"] = "none",
) -> Markup:
    """Compile GUML to HTML, marked safe for Jinja.

    The defaults are the ones a template wants and they differ from :func:`guml.render`:

    * ``fragment=True`` — a template is embedding this in a page it already owns, so a whole document
      would be wrong. :func:`guml.render` has no such context and defaults the other way.
    * ``style="none"`` — a fragment cannot carry a ``<head>``, and repeating the theme stylesheet in
      every rendered snippet would be absurd. Put it in your base template once; ``guml.stylesheet()``
      via ``compile(..., "html")`` has it, or serve the CSS from ``@guml/core``.
    * ``level="core"`` — as everywhere else a document might not be yours.
    """
    return Markup(guml.render(source, level=level, style=style, fragment=fragment))


class GumlExtension(Extension):
    """Registers ``guml`` as a filter.

        app.jinja_env.add_extension(GumlExtension)
    """

    def __init__(self, environment: Environment) -> None:
        super().__init__(environment)
        environment.filters["guml"] = guml_filter
