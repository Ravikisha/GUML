//! Static HTML backend: one `.html` file, no JavaScript at all.
//!
//! # What this backend is for
//!
//! Two things the React backend cannot demonstrate.
//!
//! 1. **The IR claim.** "GUML is an IR with several backends" is a claim about the *language*, and a
//!    single backend cannot support it. This one shares the design-system table (`react::classes`)
//!    with React, so the same GUML produces visually identical markup down to the class strings —
//!    which is the demonstration. A second table would have made this a coincidence.
//! 2. **The content floor.** A landing page is mostly prose, and prose needs no runtime. For that
//!    category this backend is the *right* output, not a degraded one: no hydration, no bundle.
//!
//! # What it refuses to do
//!
//! There is no JavaScript, so there is no state, no actions, and no fetch. Invariant 3 says a
//! construct the backend cannot lower gets a warning and a visible marker rather than silence — so a
//! `btn` with an action still renders, `disabled`, with `data-guml-inert` and a warning naming the
//! action that was dropped. The alternative is a page that looks complete and does nothing, which is
//! the worst outcome for a compiler whose entire pitch is reliability.
//!
//! `faq` is the interesting case: `<details>`/`<summary>` is interactive *without* script, so it
//! lowers fully. That is the shape of the whole backend — everything declarative survives.

use crate::react::classes;
use crate::{Backend, Emitted, OutFile, component_name, modifiers_of, unsupported_in};
use guml_ast::{Element, Positional, Program, Value};
use guml_diagnostics::Diagnostics;
use std::fmt::Write as _;

/// How the emitted document gets its styling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Style {
    /// Inline the active theme's stylesheet. Self-contained, deployable, no network at render time.
    #[default]
    Inline,
    /// A Tailwind Play CDN script tag. Convenient for a preview; a runtime dependency on a third
    /// party, so never the default.
    Cdn,
    /// No styling at all: the host supplies it. The right choice when the classes are processed by an
    /// existing pipeline.
    None,
}

#[derive(Debug, Default)]
pub struct HtmlBackend {
    pub style: Style,
    /// Emit the content only — no `<!doctype>`, no `<html>`, no `<head>`, and **no `<main>`**.
    ///
    /// For embedding in a page the host already owns: a Jinja template, a Django block, an htmx swap
    /// target. The dropped `<main>` is the part worth stating, because it looks like a loss and is the
    /// opposite: a document may contain exactly one `main` landmark, so a fragment that carried its own
    /// would produce a second one the moment it were embedded in any real page. Landmarks belong to
    /// whoever owns the document, and in fragment mode that is not us.
    pub fragment: bool,
}

impl Backend for HtmlBackend {
    fn name(&self) -> &'static str {
        "html"
    }

    fn emit(&self, program: &Program) -> Emitted {
        let mut out = Emitted::default();
        let style = self.style;
        let name = component_name(program.page.as_ref().map(|p| p.name.as_str()).unwrap_or("Page"));
        let meta = program.page.as_ref().map(|p| p.meta.clone()).unwrap_or_default();
        // The declared title if there is one, else the page name — which is a component name and a
        // poor document title, but better than an empty `<title>`.
        let title = meta
            .title
            .clone()
            .or_else(|| program.page.as_ref().map(|p| p.name.clone()))
            .unwrap_or_else(|| name.clone());

        // Reported once per document rather than per element: a page with eight buttons should not
        // produce eight copies of the same architectural fact.
        if !program.states.is_empty() {
            let span = program.states[0].span;
            crate::unsupported_in(
                &mut out.diagnostics,
                "html",
                span,
                format!(
                    "`state` needs a runtime — the `html` backend emits the initial value of {} and nothing updates it",
                    program
                        .states
                        .iter()
                        .map(|s| format!("`{}`", s.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
        if !program.resources.is_empty() {
            let span = program.resources[0].span;
            unsupported_in(
                &mut out.diagnostics,
                "html",
                span,
                format!(
                    "`data` needs a runtime — {} is not fetched, so its repeaters render their empty state",
                    program
                        .resources
                        .iter()
                        .map(|r| format!("`{}`", r.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }

        // An effect is behaviour, and this backend has none. Silence here would be the worst option:
        // the page would look complete and the refetch would simply never happen, which is exactly the
        // silent mis-lowering invariant 3 forbids.
        if !program.effects.is_empty() {
            let span = program.effects[0].span;
            unsupported_in(
                &mut out.diagnostics,
                "html",
                span,
                format!(
                    "{} declared effect(s) need a runtime — nothing runs them, so the page renders as it was on first paint",
                    program.effects.len()
                ),
            );
        }

        let mut body = String::new();
        let mut g = Gen { program, diags: &mut out.diagnostics };
        for el in &program.tree {
            body.push_str(&g.element(el, 3));
        }

        // Fragment mode: the content, and nothing that belongs to a document.
        //
        // The stylesheet is the part that needs care. A fragment has no `<head>`, and repeating a
        // 20 KB theme inline in every fragment would be absurd — but *silently* dropping it would be
        // invariant 3 in its purest form: a page that renders unstyled and reports nothing. So it is
        // emitted as a second file the host includes once, and `Style::None` is the way to say you
        // already have it.
        if self.fragment {
            let content = if body.trim().is_empty() {
                "<!-- the document has no renderable elements -->\n".to_string()
            } else {
                dedent(&body)
            };
            out.files.push(OutFile {
                path: format!("{name}.html"),
                contents: content,
                source_map: None,
            });

            if style == Style::Inline {
                match crate::theme::active().css.as_deref() {
                    Some(css) => out.files.push(OutFile {
                        path: format!("{name}.css"),
                        contents: css.to_string(),
                        source_map: None,
                    }),
                    None => unsupported_in(
                        &mut out.diagnostics,
                        "html",
                        program
                            .page
                            .as_ref()
                            .map(|p| p.span)
                            .unwrap_or(guml_diagnostics::Span::point(0, 1, 1)),
                        format!(
                            "theme `{}` ships no stylesheet, so this fragment has no styling; give the theme a `css` field, or ask for the `none` style if the host already provides it",
                            crate::theme::active().name
                        ),
                    ),
                }
            }
            return out;
        }

        let mut src = String::new();
        // `lang` is not cosmetic: without it a screen reader guesses pronunciation, and it is the
        // first thing every accessibility checker looks for. Defaulted here rather than in the AST so
        // a backend can tell "the author said `en`" from "the author said nothing".
        let lang = meta.lang.as_deref().unwrap_or("en");
        let dir = meta.dir.as_deref().map(|d| format!(" dir=\"{d}\"")).unwrap_or_default();
        let _ = writeln!(src, "<!doctype html>");
        let _ = writeln!(src, "<html lang={:?}{dir}>", escape(lang));
        src.push_str("  <head>\n");
        src.push_str("    <meta charset=\"utf-8\" />\n");
        src.push_str(
            "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n",
        );
        let _ = writeln!(src, "    <title>{}</title>", escape(&title));
        if let Some(description) = &meta.description {
            let _ = writeln!(
                src,
                "    <meta name=\"description\" content={:?} />",
                escape(description)
            );
        }
        // Styling. Two paths, and the deployable one is the default.
        //
        // The CDN was the only path for a while, which made every emitted document depend on a
        // third-party script at runtime — indefensible for a backend whose selling point is needing
        // no build step and no JavaScript. It is now opt-in, for previews.
        match style {
            Style::Inline => match crate::theme::active().css.as_deref() {
                Some(css) => {
                    src.push_str("    <style>\n");
                    for line in css.lines() {
                        let _ = writeln!(src, "      {line}");
                    }
                    src.push_str("    </style>\n");
                }
                // Invariant 3 again: a themed document with no stylesheet is silently unstyled, which
                // reads as a broken page rather than as a missing input.
                None => unsupported_in(
                    &mut out.diagnostics,
                    "html",
                    program
                        .page
                        .as_ref()
                        .map(|p| p.span)
                        .unwrap_or(guml_diagnostics::Span::point(0, 1, 1)),
                    format!(
                        "theme `{}` ships no stylesheet, so this document has no styling; give the theme a `css` field, or use `--backend html-cdn` for a preview",
                        crate::theme::active().name
                    ),
                ),
            },
            Style::Cdn => {
                src.push_str(
                    "    <!-- Play CDN: a preview convenience, not a production artifact. -->\n",
                );
                src.push_str("    <script src=\"https://cdn.tailwindcss.com\"></script>\n");
            }
            Style::None => {}
        }
        // No class on `<body>`.
        //
        // It carried `bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100` — four colour
        // literals compiled into a backend, on the one element every page has. So the *page background* was
        // the single thing no theme could change, and with a token-driven default that showed immediately: a
        // slate page around token-coloured cards.
        //
        // The theme's stylesheet sets `body { background-color: var(--background); color: var(--foreground) }`
        // in its reset, which is where a page-level surface belongs. A theme that wants a different one
        // changes a variable instead of fighting a class the compiler emitted.
        src.push_str("  </head>\n  <body>\n");
        src.push_str("    <main class=\"mx-auto max-w-3xl p-6\">\n");
        if body.trim().is_empty() {
            src.push_str("      <!-- the document has no renderable elements -->\n");
        } else {
            src.push_str(&body);
        }
        src.push_str("    </main>\n  </body>\n</html>\n");

        out.files.push(OutFile { path: format!("{name}.html"), contents: src, source_map: None });
        out
    }
}

struct Gen<'a> {
    program: &'a Program,
    diags: &'a mut Diagnostics,
}

impl Gen<'_> {
    fn element(&mut self, el: &Element, depth: usize) -> String {
        let pad = "  ".repeat(depth);

        // Escape hatches. `raw html` is exactly what this backend is for; a block aimed at another
        // backend is skipped rather than emitted as broken markup.
        if el.tag == "raw" {
            let target = el.positionals.iter().find_map(|p| match p {
                Positional::Text(t) => Some(t.as_str()),
                _ => None,
            });
            if target.is_some_and(|t| t != "html") {
                return String::new();
            }
            return el.text_lines.iter().map(|l| format!("{pad}{l}\n")).collect();
        }
        if el.tag == "js" {
            // A `js` block is JavaScript, and this backend emits none. Dropping it silently would
            // be the same class of bug as dropping an action.
            unsupported_in(
                self.diags,
                "html",
                el.span,
                "a `js` block cannot run in the `html` backend, which emits no JavaScript",
            );
            return String::new();
        }

        // `if={cond}` with no runtime. Nothing can *change* here, so the condition is fixed at whatever
        // the referenced state was initialised to — which is exactly the rule this backend already
        // applies to a binding in prose ("state renders its initial value"). Applying it consistently
        // means a `card X if={open}` with `state open=false` is omitted rather than shown, and the
        // reader gets the page as it first loads.
        //
        // Anything more complex than a bare state reference is reported instead of guessed at: a static
        // page that silently picked one branch of `{count > 3}` would be asserting a fact about runtime
        // data this backend cannot see.
        if let Some(cond) = el.attr("if") {
            match self.static_truth(cond) {
                Some(true) => {}
                Some(false) => {
                    return format!(
                        "{pad}<!-- guml: `{}` hidden — `if=` is false at initial state -->\n",
                        el.tag
                    );
                }
                None => {
                    unsupported_in(
                        self.diags,
                        "html",
                        el.span,
                        "`if=` cannot be evaluated without a runtime unless it is a plain state reference — the element is rendered",
                    );
                }
            }
        }

        // A component that declares `needs_runtime` cannot work here, and the *registry* is what says
        // so — this backend does not carry a list of such tags. That is what makes the check extend to
        // components it has never heard of: a `modal` and a third-party `combobox` are refused by the
        // same three lines.
        //
        // `tabs` and the repeaters are excluded because they have real partial lowerings below:
        // `tabs` is handled just under this, and a repeater renders its `empty` slot. "Needs a runtime"
        // decides whether a tag can be *complete* here, not whether it renders nothing.
        if crate::needs_runtime(&el.tag) && !matches!(el.tag.as_str(), "tabs" | "list" | "table") {
            unsupported_in(
                self.diags,
                "html",
                el.span,
                format!(
                    "`{}` needs a JavaScript runtime, and this backend emits none — it is marked `data-guml-inert` rather than rendered as a visible element that does nothing",
                    el.tag
                ),
            );
            let pad = "  ".repeat(depth);
            return format!(
                "{pad}<template data-guml-inert=\"needs runtime\" data-guml-tag=\"{}\"></template>\n",
                el.tag
            );
        }

        match el.tag.as_str() {
            "list" | "table" => self.repeater(el, depth),
            "faq" => self.faq(el, depth),
            "tier" => self.tier(el, depth),
            // A segmented control is a set of buttons over state, and state does not exist here.
            "tabs" => {
                unsupported_in(
                    self.diags,
                    "html",
                    el.span,
                    "`tabs` switches state, which the `html` backend has no way to do",
                );
                String::new()
            }
            _ => self.plain(el, depth),
        }
    }

    fn plain(&mut self, el: &Element, depth: usize) -> String {
        let pad = "  ".repeat(depth);
        let mods = modifiers_of(el);
        // `class_list`, not `classes`: it folds in the layout attributes too. Without them `grid cols=3`
        // emitted `grid gap-6` here while React emitted `grid gap-6 md:grid-cols-3` — the same document with
        // three columns in one representation and an unspecified number in the no-JavaScript build that
        // ships to a browser with no way to fix it.
        let class = crate::class_list(el);
        let text = self.text_of(el);

        let mut out = String::new();
        match el.tag.as_str() {
            "h1" | "h2" | "h" | "p" | "text" | "metric" | "head" | "label" | "note" | "badge"
            | "avatar" | "step" | "option" => {
                let tag = html_tag(&el.tag);
                let _ = writeln!(out, "{pad}<{tag} class={class:?}>{}</{tag}>", escape(&text));
            }
            // `<hr>` is void, so a closing tag would be a parse error; `skeleton` is a `<div>` and
            // needs one even though it has no content.
            "divider" => {
                let _ = writeln!(out, "{pad}<hr class={class:?} />");
            }
            "skeleton" => {
                let _ = writeln!(out, "{pad}<div class={class:?}></div>");
            }
            "img" => {
                let src = attr_of(el, "src").unwrap_or_default();
                // `alt` is required by the registry, so `sema` has already rejected the empty case.
                let alt = attr_of(el, "alt").or_else(|| aria_of(el)).unwrap_or_default();
                let _ = writeln!(
                    out,
                    "{pad}<img class={class:?} src={:?} alt={:?} />",
                    escape(&src),
                    escape(&alt)
                );
            }
            // `<progress>` is one of the few interactive-looking elements that needs no script at
            // all, so unlike every other control this one is *not* inert here.
            "progress" => {
                let value = attr_of(el, "value").unwrap_or_default();
                let max = attr_of(el, "max").unwrap_or_else(|| "100".to_string());
                let name = aria_of(el).unwrap_or_else(|| "Progress".to_string());
                let _ = writeln!(
                    out,
                    "{pad}<progress class={class:?} value={:?} max={:?} aria-label={:?}></progress>",
                    escape(&value),
                    escape(&max),
                    escape(&name)
                );
            }
            "stat" => {
                let mut parts = el.positionals.iter().filter_map(|p| match p {
                    Positional::Text(t) => Some(t.clone()),
                    Positional::Binding(b) => Some(format!("{{{}}}", b.source)),
                    _ => None,
                });
                let label = parts.next().unwrap_or_default();
                let value = parts.next().unwrap_or_default();
                let label = self.resolve_bindings(&label, el);
                let value = self.resolve_bindings(&value, el);
                let _ = writeln!(out, "{pad}<dl class={:?}>", classes("stat", &mods));
                let _ = writeln!(
                    out,
                    "{pad}  <dt class={:?}>{}</dt>",
                    classes("stat-label", &mods),
                    escape(&label)
                );
                let _ = writeln!(
                    out,
                    "{pad}  <dd class={:?}>{}</dd>",
                    classes("stat-value", &mods),
                    escape(&value)
                );
                if let Some(delta) = attr_of(el, "delta") {
                    let delta = self.resolve_bindings(&delta, el);
                    let _ = writeln!(
                        out,
                        "{pad}  <dd class={:?}>{}</dd>",
                        classes("stat-delta", &mods),
                        escape(&delta)
                    );
                }
                let _ = writeln!(out, "{pad}</dl>");
            }
            "btn" => {
                // An action is the whole point of a button, so a button without one is inert and
                // says so — in the markup, in the diagnostics, and to a screen reader.
                if !el.actions.is_empty() {
                    unsupported_in(
                        self.diags,
                        "html",
                        el.span,
                        format!(
                            "`btn {text}` has an action, and the `html` backend emits no JavaScript — rendered disabled"
                        ),
                    );
                }
                let _ = writeln!(
                    out,
                    "{pad}<button class={class:?} type=\"button\" disabled data-guml-inert=\"no runtime\">{}</button>",
                    escape(&text)
                );
            }
            "link" => {
                let href = el
                    .positionals
                    .iter()
                    .find_map(|p| match p {
                        Positional::Route(r) => Some(r.clone()),
                        Positional::Anchor(a) => Some(format!("#{a}")),
                        _ => None,
                    })
                    .unwrap_or_else(|| "#".to_string());
                let _ =
                    writeln!(out, "{pad}<a class={class:?} href={:?}>{}</a>", href, escape(&text));
            }
            // A `select` is a real `<select>`, not an `<input>`. It was the latter, which meant the
            // no-JavaScript build rendered a dropdown as a text box and showed none of its choices —
            // the same missing-options bug the React backend had, plus a wrong element on top.
            //
            // `disabled` rather than `readonly`: `readonly` has no effect on a `<select>`, so the
            // control would have looked inert and still been operable, changing a value nothing reads.
            "select" => {
                let name = aria_of(el).unwrap_or_else(|| el.tag.clone());
                let _ = writeln!(
                    out,
                    "{pad}<select class={class:?} aria-label={:?} disabled data-guml-inert=\"no runtime\">",
                    escape(&name)
                );
                if let Some(hint) = attr_of(el, "placeholder") {
                    let _ = writeln!(
                        out,
                        "{pad}  <option value=\"\" disabled>{}</option>",
                        escape(&hint)
                    );
                }
                for opt in crate::select_options(self.program, el) {
                    let _ =
                        writeln!(out, "{pad}  <option value={:?}>{}</option>", opt, escape(&opt));
                }
                let _ = writeln!(out, "{pad}</select>");
            }
            "input" | "check" | "toggle" => {
                let kind = match el.tag.as_str() {
                    "check" | "toggle" => "checkbox".to_string(),
                    _ => attr_of(el, "kind").unwrap_or_else(|| "text".to_string()),
                };
                let name = aria_of(el).unwrap_or_else(|| el.tag.clone());
                let _ = writeln!(
                    out,
                    "{pad}<input class={class:?} type={kind:?} aria-label={:?} readonly data-guml-inert=\"no runtime\" />",
                    escape(&name)
                );
            }
            _ => {
                // Containers, and anything else with children: a `div` with the same classes React
                // would have used.
                //
                // A container's *title* is its first quoted positional and its prose is the `|`
                // content — two different slots. Using `text_of` here rendered the prose as both,
                // because that helper prefers content (which is right for a text tag and wrong here).
                let tag = html_tag(&el.tag);
                // A void element reaching this arm would be emitted as `<hr>…</hr>`, which is not
                // parseable HTML. Every void tag has its own arm above; this is the backstop so adding
                // one and forgetting the arm produces valid markup rather than a broken document.
                if is_void(&el.tag) {
                    let _ = writeln!(out, "{pad}<{tag} class={class:?} />");
                    return out;
                }
                let title = self.title_of(el);
                let id = attr_of(el, "id").or_else(|| anchor_of(el));
                match id {
                    Some(id) => {
                        let _ = writeln!(out, "{pad}<{tag} class={class:?} id={:?}>", escape(&id));
                    }
                    None => {
                        let _ = writeln!(out, "{pad}<{tag} class={class:?}>");
                    }
                }
                if !title.is_empty() {
                    let _ =
                        writeln!(out, "{pad}  <h3 class=\"font-medium\">{}</h3>", escape(&title));
                }
                if let Some(line) = &el.content {
                    let _ = writeln!(
                        out,
                        "{pad}  <p class=\"mt-1 text-sm text-slate-500\">{}</p>",
                        escape(line)
                    );
                }
                for line in &el.text_lines {
                    let _ = writeln!(out, "{pad}  <li>{}</li>", escape(line));
                }
                for child in &el.children {
                    out.push_str(&self.element(child, depth + 1));
                }
                let _ = writeln!(out, "{pad}</{tag}>");
            }
        }

        if !el.actions.is_empty() && el.tag != "btn" {
            unsupported_in(
                self.diags,
                "html",
                el.span,
                format!(
                    "`{}` has an action, which needs a runtime the `html` backend has not",
                    el.tag
                ),
            );
        }
        out
    }

    /// A repeater with no data renders the state a first-time visitor sees: the empty slot.
    fn repeater(&mut self, el: &Element, depth: usize) -> String {
        let pad = "  ".repeat(depth);
        let mut out = String::new();
        let empty = el.children.iter().find(|c| c.tag == "empty");
        let message =
            empty.map(|e| self.text_of(e)).unwrap_or_else(|| "Nothing here yet.".to_string());
        let _ = writeln!(
            out,
            "{pad}<p class=\"mt-10 text-center text-sm text-slate-500\" data-guml-inert=\"no data at build time\">{}</p>",
            escape(&message)
        );
        out
    }

    /// `<details>`/`<summary>`. Interactive with no script, so this one lowers completely.
    fn faq(&mut self, el: &Element, depth: usize) -> String {
        let pad = "  ".repeat(depth);
        let open = attr_of(el, "open").and_then(|v| v.parse::<usize>().ok());
        let mut out = String::new();
        let _ = writeln!(out, "{pad}<div class=\"mt-8 divide-y divide-slate-200\">");
        for (i, line) in el.text_lines.iter().enumerate() {
            let (q, a) = line.split_once('|').unwrap_or((line.as_str(), ""));
            let is_open = open.is_some_and(|n| n == i + 1);
            let _ = writeln!(
                out,
                "{pad}  <details class=\"py-3\"{}>",
                if is_open { " open" } else { "" }
            );
            let _ = writeln!(
                out,
                "{pad}    <summary class=\"cursor-pointer text-sm font-medium\">{}</summary>",
                escape(q.trim())
            );
            let _ = writeln!(
                out,
                "{pad}    <p class=\"mt-2 text-sm text-slate-600\">{}</p>",
                escape(a.trim())
            );
            let _ = writeln!(out, "{pad}  </details>");
        }
        let _ = writeln!(out, "{pad}</div>");
        out
    }

    /// A pricing tier: a heading, a price, and the feature lines below it.
    fn tier(&mut self, el: &Element, depth: usize) -> String {
        let pad = "  ".repeat(depth);
        // A `tier`'s call to action is an `<a href>` built from its `cta` and its route, so an action on
        // one has nowhere to go. It used to be dropped in silence: `tier Team … >subscription.setPlan`
        // emitted a plain link and the plan never changed, with exit code 0. Report it — invariant 3 — and
        // the author gets a `card` with a `btn` in it, which works today.
        if !el.actions.is_empty() {
            unsupported_in(
                self.diags,
                "html",
                el.span,
                "an action on a `tier`: its call to action is a link built from `cta` and the route. Put a `btn` in a `card` instead",
            );
        }
        let mods = modifiers_of(el);
        let words: Vec<String> = el
            .positionals
            .iter()
            .filter_map(|p| match p {
                Positional::Text(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        let mut out = String::new();
        let _ = writeln!(out, "{pad}<div class={:?}>", classes("tier", &mods));
        if let Some(name) = words.first() {
            let _ = writeln!(out, "{pad}  <h3 class=\"font-medium\">{}</h3>", escape(name));
        }
        if let Some(price) = words.get(1) {
            let _ = writeln!(
                out,
                "{pad}  <p class=\"mt-2 text-3xl font-semibold\">{}</p>",
                escape(price)
            );
        }
        let _ = writeln!(out, "{pad}  <ul class=\"mt-4 space-y-1 text-sm text-slate-600\">");
        for line in &el.text_lines {
            let _ = writeln!(out, "{pad}    <li>{}</li>", escape(line));
        }
        let _ = writeln!(out, "{pad}  </ul>");
        // A tier's `cta` is a link, and a link works without script.
        if let Some(cta) = attr_of(el, "cta") {
            let href = el
                .positionals
                .iter()
                .find_map(|p| match p {
                    Positional::Route(r) => Some(r.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "#".to_string());
            let _ = writeln!(
                out,
                "{pad}  <a class=\"mt-4 inline-block rounded-md bg-slate-900 px-4 py-2 text-sm font-medium text-white\" href={href:?}>{}</a>",
                escape(&cta)
            );
        }
        let _ = writeln!(out, "{pad}</div>");
        out
    }

    /// A container's title: its first text positional, with bindings resolved. Empty when it has
    /// none, which is the common case — most containers are pure layout.
    /// The truth of an `if=` condition at the document's initial state, or `None` when this backend
    /// cannot know it.
    ///
    /// Deliberately narrow. A bare `{open}` naming a declared `state` is answerable — its initial value
    /// is in the document. `{!open}` is answerable for the same reason. Anything else (a comparison, an
    /// aggregate over a fetched resource, an unknown name) is not, and returning `None` there is what
    /// keeps this from asserting something about data no static page has.
    fn static_truth(&self, cond: &Value) -> Option<bool> {
        match cond {
            Value::Bool(b) => Some(*b),
            Value::Binding(b) => {
                let src = b.source.trim();
                let (negated, name) = match src.strip_prefix('!') {
                    Some(rest) => (true, rest.trim()),
                    None => (false, src),
                };
                if !name.chars().all(|c| c.is_alphanumeric() || c == '_') || name.is_empty() {
                    return None;
                }
                let state = self.program.states.iter().find(|s| s.name == name)?;
                let truth = match &state.init {
                    Value::Bool(v) => *v,
                    Value::Num(n) => *n != 0.0,
                    Value::Str(s) => !s.is_empty(),
                    Value::Word(w) => !w.is_empty(),
                    // A flag with no value is `true`; a binding initialiser is not statically known.
                    Value::Flag => true,
                    Value::Binding(_) => return None,
                };
                Some(truth != negated)
            }
            _ => None,
        }
    }

    fn title_of(&mut self, el: &Element) -> String {
        let raw = el.positionals.iter().find_map(|p| match p {
            Positional::Text(t) => Some(t.clone()),
            Positional::Binding(b) => Some(format!("{{{}}}", b.source)),
            _ => None,
        });
        match raw {
            Some(text) => self.resolve_bindings(&text, el),
            None => String::new(),
        }
    }

    /// Prose for an element, with `{bindings}` replaced by what a static page can honestly show.
    fn text_of(&mut self, el: &Element) -> String {
        let raw = el
            .content
            .clone()
            .or_else(|| {
                el.positionals.iter().find_map(|p| match p {
                    Positional::Text(t) => Some(t.clone()),
                    Positional::Binding(b) => Some(format!("{{{}}}", b.source)),
                    _ => None,
                })
            })
            .unwrap_or_default();
        self.resolve_bindings(&raw, el)
    }

    /// Substitute `{bindings}` with what a static page can honestly show.
    ///
    /// A binding reads state or a resource, and neither exists at build time. The initial value is
    /// used where one is known; where it is not, the binding is reported and an em dash is rendered,
    /// rather than leaving `{count}` on screen for a visitor to read.
    fn resolve_bindings(&mut self, raw: &str, el: &Element) -> String {
        let mut out = String::new();
        let mut rest = raw;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let Some(close) = after.find('}') else {
                out.push_str(&rest[open..]);
                return out;
            };
            let source = &after[..close];
            match self.static_value(source) {
                Some(value) => out.push_str(&value),
                None => {
                    unsupported_in(
                        self.diags,
                        "html",
                        el.span,
                        format!(
                            "`{{{source}}}` has no value at build time; rendered as an em dash"
                        ),
                    );
                    out.push('—');
                }
            }
            rest = &after[close + 1..];
        }
        out.push_str(rest);
        out
    }

    /// The value a binding has before any JavaScript runs.
    fn static_value(&self, source: &str) -> Option<String> {
        let head = guml_ast::head_ident(source);
        // An aggregate over a resource that was never fetched is zero, and honestly so.
        if source.ends_with(".count") && self.program.resources.iter().any(|r| r.name == head) {
            return Some("0".to_string());
        }
        let state = self.program.states.iter().find(|s| s.name == head)?;
        // Only a bare state read has a known value; `count + 1` would need an evaluator, and this
        // backend deliberately has none.
        if source.trim() != state.name {
            return None;
        }
        // An enumerated domain's first value is its initial value; otherwise the declared literal.
        let init = match state.domain.first() {
            Some(first) => first.clone(),
            None => value_text(&state.init).unwrap_or_default(),
        };
        Some(escape(init.trim()))
    }
}

/// GUML tag → HTML element. Kept beside the React backend's choices on purpose: `head` is a big
/// number, not a `<head>`.
/// The element this backend emits for a tag.
///
/// Delegates to [`crate::element_for`], which is shared with the React backend. This function used to
/// hold its own table with a `_ => "div"` fallback, and the two had drifted: `nav`, `hero` and `footer`
/// all became `<div>` here while React emitted `<nav>`, `<header>` and `<footer>`. The no-JavaScript
/// build therefore shipped a page with **no landmarks at all**, so a screen-reader user could not jump
/// to the navigation on a document where the React build let them — a pure accessibility regression
/// that no test could see, because each backend's snapshot only ever agreed with itself.
///
/// `div` remains the fallback for a tag no table knows, since this backend has nothing better to do
/// with an unknown container and `sema` has already rejected an unknown *tag*.
/// Strip the indentation the body was generated with, so a fragment starts at column 0.
///
/// The element writer is told depth 3 because in a full document the content sits inside
/// `<html><body><main>`. A fragment has none of those, so six leading spaces on every line is just
/// wrong — and it is not cosmetic once the fragment lands inside a `<pre>`, where leading whitespace
/// is content.
///
/// Removes a *common* prefix rather than a fixed six, so nesting inside the fragment is preserved and
/// the function stays correct if the caller's depth ever changes.
fn dedent(body: &str) -> String {
    let indent = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    body.lines()
        .map(|l| if l.len() >= indent { &l[indent..] } else { l.trim_start() })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn html_tag(tag: &str) -> &'static str {
    crate::element_for(tag).unwrap_or("div")
}

/// Elements with no closing tag. Getting this wrong emits `<hr></hr>`, which is not parseable HTML.
fn is_void(tag: &str) -> bool {
    matches!(tag, "input" | "check" | "toggle" | "divider" | "img")
}

fn attr_of(el: &Element, name: &str) -> Option<String> {
    el.attrs.iter().find(|a| a.name == name).and_then(|a| value_text(&a.value))
}

/// An attribute or initial value as text. `Flag` has no text, which is what distinguishes
/// `featured` from `cta="Go Pro"`.
fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::Str(t) | Value::Word(t) => Some(t.clone()),
        Value::Num(n) => Some(format_number(*n)),
        Value::Bool(b) => Some(b.to_string()),
        Value::Binding(b) => Some(b.source.clone()),
        Value::Flag => None,
    }
}

/// `0` rather than `0.0`: the initial value of `state count=0` is rendered for a reader.
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) } else { n.to_string() }
}

fn aria_of(el: &Element) -> Option<String> {
    attr_of(el, "aria").or_else(|| attr_of(el, "placeholder"))
}

fn anchor_of(el: &Element) -> Option<String> {
    el.positionals.iter().find_map(|p| match p {
        Positional::Anchor(a) => Some(a.clone()),
        _ => None,
    })
}

/// HTML text escaping. Prose reaches this verbatim from the lexer, so it has never been escaped
/// before now — which is exactly why GUML prose costs so few tokens, and exactly why this function
/// cannot be skipped.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}
