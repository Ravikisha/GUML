//! Svelte 5 backend: one `.svelte` file, runes for reactivity.
//!
//! # What this backend is for
//!
//! Three backends is where "GUML is an IR" stops being a claim and starts being a demonstration. React
//! and static HTML share a design-system table but sit at opposite ends of the runtime question — one
//! is all client state, the other has none. Svelte is the interesting middle: it has reactivity, and it
//! expresses it *declaratively*, so it exercises a part of the AST neither of the others does.
//!
//! It is also the compile-away-the-framework story. The same GUML that emits a React component with
//! four hooks emits a Svelte component with none, and the output is smaller because the framework did
//! at build time what React does at runtime.
//!
//! # What it shares, and why that matters
//!
//! * `react::classes` — the same theme, so the same document looks the same in all three backends.
//! * `crate::expr` — the same expression lowering. Svelte templates are JavaScript expressions, so
//!   `{tasks.open.count}` lowers identically. A second lowering would be a second answer waiting to
//!   disagree, which this project has already been bitten by twice.
//!
//! # Runes, not stores
//!
//! `$state`, `$derived`, `$effect`. Svelte 5's runes are the current model and they map onto GUML's
//! declarations almost exactly: a `state` directive is `$state`, a derived aggregate is `$derived`, and
//! a resource's fetch-on-mount is `$effect`. Stores would need imports and subscriptions to express the
//! same thing.

use crate::expr::{self, Ctx};
use crate::react::classes;
use crate::{Backend, Emitted, OutFile, component_name, modifiers_of, unsupported_in};
use guml_ast::{Element, Positional, Program, Resource, Value};
use guml_diagnostics::Diagnostics;
use std::fmt::Write as _;

#[derive(Debug, Default)]
pub struct SvelteBackend;

impl Backend for SvelteBackend {
    fn name(&self) -> &'static str {
        "svelte"
    }

    fn emit(&self, program: &Program) -> Emitted {
        let mut out = Emitted::default();
        let name = component_name(program.page.as_ref().map(|p| p.name.as_str()).unwrap_or("Page"));

        // Same dead-declaration elimination as the other backends, from the same shared liveness
        // answer — see `guml_ast::referenced_names`.
        let live = guml_ast::referenced_names(program);
        let states: Vec<_> = program.states.iter().filter(|s| live.contains(&s.name)).collect();
        let resources: Vec<_> =
            program.resources.iter().filter(|r| live.contains(&r.name)).collect();

        let mut script = String::new();
        let collections: Vec<String> = program.resources.iter().map(|r| r.name.clone()).collect();
        let row_bool = crate::row_bool_fields(program);
        let ctx = Ctx::default().with_collections(&collections).with_row_bool(&row_bool);

        for s in &states {
            let _ = writeln!(script, "  let {} = $state({});", s.name, initial(&s.init, &s.domain));
        }

        if !resources.is_empty() {
            script.push('\n');
            script.push_str(crate::RETRY_JS);
            // The cache builds on `retrying`, so it follows it. Same policy as the React backend's, in
            // the untyped spelling — held together by a test on the behaviour rather than on the text.
            script.push_str(&indent_js(crate::CACHE_JS));
        }

        for r in &resources {
            script.push('\n');
            script.push_str(&resource_runes(r));
        }

        // `js` blocks are component-body code in Svelte exactly as in React — and, exactly as in React, they
        // come *before* the derived values, because a repeater may iterate a `js`-computed array and its
        // `$derived` reads it. The other order emitted a read above the declaration.
        for block in js_blocks(&program.tree) {
            script.push('\n');
            for line in block {
                let _ = writeln!(script, "  {line}");
            }
        }

        // `where=` filtering is `$derived`, which is the whole argument for this backend: the same
        // declaration that needs `useMemo` plus a dependency array in React needs neither here.
        let mut derived = String::new();
        collect_derived(&program.tree, program, &mut derived);
        if !derived.is_empty() {
            script.push('\n');
            script.push_str(&derived);
        }

        let mut g = Gen {
            program,
            diags: &mut out.diagnostics,
            collections: &collections,
            row_bool: &row_bool,
        };
        let mut body = String::new();
        for el in &program.tree {
            body.push_str(&g.element(el, 0, &ctx));
        }

        // Declared effects. This is the case that shows why the directive is worth having: Svelte's
        // `$effect` tracks *every* reactive read in its body, so the naive translation would re-run
        // whenever anything the action touches changes — not when the declared trigger does. Reading
        // the trigger and wrapping the body in `untrack` makes the dependency exactly what the author
        // wrote, and it is not a thing a person writes by hand.
        let mut effects = String::new();
        for e in &program.effects {
            match &e.trigger {
                guml_ast::Trigger::Mount => {
                    let body = g.effect_body(e);
                    let _ = writeln!(effects, "\n  onMount(() => {{ {body} }});");
                }
                guml_ast::Trigger::Change(expr) => {
                    let dep = expr::lower_in(expr, &ctx);
                    let body = g.effect_body(e);
                    let _ = writeln!(
                        effects,
                        "\n  $effect(() => {{\n    void {dep};\n    untrack(() => {{ {body} }});\n  }});"
                    );
                }
            }
        }
        if !effects.is_empty() {
            // Prepended, because an import has to precede the code that uses it and the script is
            // assembled top-down.
            let mut imports: Vec<&str> = Vec::new();
            if program.effects.iter().any(|e| e.trigger == guml_ast::Trigger::Mount) {
                imports.push("onMount");
            }
            if program.effects.iter().any(|e| matches!(e.trigger, guml_ast::Trigger::Change(_))) {
                imports.push("untrack");
            }
            script = format!(
                "  import {{ {} }} from \"svelte\";\n\n{script}{effects}",
                imports.join(", ")
            );
        }

        let mut src = String::new();
        if !script.trim().is_empty() {
            src.push_str("<script>\n");
            src.push_str(&script);
            src.push_str("</script>\n\n");
        }
        if body.trim().is_empty() {
            let _ = writeln!(src, "<!-- `{name}` has no renderable elements -->");
        } else {
            src.push_str(&body);
        }

        out.files.push(OutFile { path: format!("{name}.svelte"), contents: src, source_map: None });
        out
    }
}

struct Gen<'a> {
    program: &'a Program,
    diags: &'a mut Diagnostics,
    collections: &'a [String],
    /// See `crate::row_bool_fields`.
    row_bool: &'a [(String, String)],
}

impl Gen<'_> {
    fn element(&mut self, el: &Element, depth: usize, ctx: &Ctx) -> String {
        self.element_in(el, depth, ctx, false)
    }

    fn element_in(&mut self, el: &Element, depth: usize, ctx: &Ctx, in_form: bool) -> String {
        let pad = "  ".repeat(depth);

        // Escape hatches. `raw svelte` is this backend's; anything else belongs to another.
        if el.tag == "raw" {
            let target = el.positionals.iter().find_map(|p| match p {
                Positional::Text(t) => Some(t.as_str()),
                _ => None,
            });
            if target.is_some_and(|t| t != "svelte") {
                return String::new();
            }
            return el.text_lines.iter().map(|l| format!("{pad}{l}\n")).collect();
        }
        if el.tag == "js" {
            return String::new(); // hoisted into `<script>`
        }

        match el.tag.as_str() {
            "list" | "table" => self.repeater(el, depth, ctx),
            "tabs" => self.tabs(el, depth),
            "faq" => self.faq(el, depth),
            _ => self.plain_in(el, depth, ctx, in_form),
        }
    }

    fn plain(&mut self, el: &Element, depth: usize, ctx: &Ctx) -> String {
        self.plain_in(el, depth, ctx, false)
    }

    fn plain_in(&mut self, el: &Element, depth: usize, ctx: &Ctx, in_form: bool) -> String {
        let pad = "  ".repeat(depth);
        // Layout attributes folded in — see `crate::layout_classes` for the drift this closes.
        let class = crate::class_list(el);
        let tag = html_tag(&el.tag);
        let text = el
            .content
            .clone()
            .or_else(|| el.binding().map(|b| format!("{{{b}}}")))
            .or_else(|| el.label().map(str::to_string))
            .unwrap_or_default();

        let mut attrs: Vec<String> = vec![format!("class={class:?}")];
        if let Some(a) = el.anchor() {
            attrs.push(format!("id={a:?}"));
        }
        for a in &el.attrs {
            match a.name.as_str() {
                "aria" => attrs.push(format!("aria-label={}", attr_value(&a.value, ctx))),
                // Presentation folded into the class list by the theme, or consumed by a feature.
                "id" | "where" | "cta" | "open" | "sort" | "of" | "cols" | "gap" | "w" | "kind"
                | "busy" | "strike" => {}
                _ => attrs.push(format!("{}={}", a.name, attr_value(&a.value, ctx))),
            }
        }

        // Two-way binding is a `bind:` directive rather than a value/handler pair — the one place
        // Svelte is meaningfully terser than React rather than merely different.
        if matches!(el.tag.as_str(), "input" | "select")
            && let Some(field) = el.label()
        {
            attrs.push(format!("bind:value={{{field}}}"));
        }
        if el.tag == "check"
            && let Some(b) = el.binding()
        {
            attrs.push(format!("bind:checked={{{}}}", expr::lower_in(b, ctx)));
        }

        if let Some(action) = el.actions.first() {
            let event = match el.tag.as_str() {
                "check" | "toggle" | "input" | "select" => "onchange",
                "form" => "onsubmit",
                _ => "onclick",
            };
            let js = self.lower_action(action, ctx, Some(el));
            if !js.is_empty() {
                let prevent = if el.tag == "form" { "e.preventDefault(); " } else { "" };
                let arg = if el.tag == "form" { "e" } else { "" };
                attrs.push(format!("{event}={{({arg}) => {{ {prevent}{js} }}}}"));
            }
        }
        if el.tag == "btn" {
            // A button inside a form with no action of its own *is* the form's submit control. Emitting
            // `type="button"` there would produce a form that cannot be submitted by keyboard.
            let submit = in_form && el.actions.is_empty();
            attrs.push(if submit { "type=\"submit\"" } else { "type=\"button\"" }.to_string());
        }

        let joined = attrs.join(" ");
        let mut out = String::new();

        // A void element cannot have children or text.
        if matches!(el.tag.as_str(), "input" | "check" | "toggle") {
            let kind = match el.tag.as_str() {
                "check" | "toggle" => "checkbox".to_string(),
                _ => attr_text(el, "kind").unwrap_or_else(|| "text".to_string()),
            };
            let _ = writeln!(out, "{pad}<input {joined} type={kind:?} />");
            return out;
        }
        // The other void elements. `<hr></hr>` and `<img></img>` are not valid markup, and Svelte's
        // compiler rejects them rather than fixing them up the way a browser would.
        if matches!(el.tag.as_str(), "divider" | "img") {
            let _ = writeln!(out, "{pad}<{tag} {joined} />");
            return out;
        }

        // A `select` renders its choices, from `crate::select_options` — the same reconciliation of
        // `option` children and state domain that the other backends use, so the three cannot disagree
        // about what a dropdown contains.
        if el.tag == "select" {
            let _ = writeln!(out, "{pad}<{tag} {joined}>");
            if let Some(hint) = attr_text(el, "placeholder") {
                let _ = writeln!(out, "{pad}  <option value=\"\" disabled>{hint}</option>");
            }
            for opt in crate::select_options(self.program, el) {
                let _ = writeln!(out, "{pad}  <option value={opt:?}>{opt}</option>");
            }
            let _ = writeln!(out, "{pad}</{tag}>");
            return out;
        }

        if el.children.is_empty() && el.text_lines.is_empty() {
            let _ =
                writeln!(out, "{pad}<{tag} {joined}>{}</{tag}>", expr::lower_text_in(&text, ctx));
            return out;
        }

        let _ = writeln!(out, "{pad}<{tag} {joined}>");
        if !text.is_empty() {
            let _ = writeln!(
                out,
                "{pad}  <h3 class=\"font-medium\">{}</h3>",
                expr::lower_text_in(&text, ctx)
            );
        }
        for line in &el.text_lines {
            let _ = writeln!(out, "{pad}  <li>{line}</li>");
        }
        let child_in_form = in_form || el.tag == "form";
        for child in &el.children {
            out.push_str(&self.element_in(child, depth + 1, ctx, child_in_form));
        }
        let _ = writeln!(out, "{pad}</{tag}>");
        out
    }

    /// `{#each}` over the resource, with the loading and empty branches Svelte expresses as `{#if}`.
    fn repeater(&mut self, el: &Element, depth: usize, _ctx: &Ctx) -> String {
        let pad = "  ".repeat(depth);
        let Some(source) = el.label() else {
            unsupported_in(self.diags, "svelte", el.span, "a repeater needs a resource name");
            return String::new();
        };
        // A derived source — a `js` array named with `of=Type` — gets the row scope and the empty state, and
        // none of the fetch scaffolding, because there is no request. See `Program::repeater_rows`.
        let fetched = self.program.resources.iter().any(|r| r.name == source);
        let fields =
            if fetched { self.item_fields(source) } else { self.program.repeater_fields(el) };
        let row_ctx =
            Ctx::item(&fields).with_collections(self.collections).with_row_bool(self.row_bool);
        let visible = if el.attr("where").is_some() {
            format!("visible{}", capitalize(source))
        } else {
            source.to_string()
        };

        let empty = el.children.iter().find(|c| c.tag == "empty");
        let template: Vec<&Element> = el.children.iter().filter(|c| c.tag != "empty").collect();
        let key = if fields.iter().any(|f| f == "id") { "item.id" } else { "item" };

        let mut out = String::new();
        // The error and loading branches belong to a fetched source. A `js` array has no `matchesError`, and
        // Svelte compiles a read of an undeclared name into a runtime `undefined` rather than refusing it —
        // so getting this wrong would be invisible until the page rendered nothing.
        if fetched {
            let _ = writeln!(out, "{pad}{{#if {source}Error}}");
            let _ = writeln!(
                out,
                "{pad}  <p role=\"alert\" class=\"mt-4 rounded-md bg-red-50 px-3 py-2 text-sm text-red-700\">{{{source}Error}}</p>"
            );
            let _ = writeln!(out, "{pad}{{:else if {source}Loading}}");
        }
        // The skeleton takes the shape of what it stands in for, matching React. A `<ul>` skeleton followed
        // by a `<table>` is a visible layout shift the moment the data lands, and two backends disagreeing
        // about the placeholder for one document is the drift invariant 8 forbids.
        let (list_open, list_close, row) = if crate::is_tabular(el) {
            let span = crate::column_headers(el).len().max(1);
            // The `<thead>` is rendered *while loading* too. The headers are static, so showing them removes
            // the layout shift entirely rather than shrinking it — and it is what the wc backend already
            // does, by keeping its `<thead>` outside the `<tbody>` it rewrites.
            let head = crate::table_head(el, &pad, "class").unwrap_or_default();
            (
                format!(
                    "<table class={:?}>\n{head}{pad}    <tbody>",
                    classes("table", &modifiers_of(el))
                ),
                format!("</tbody>\n{pad}  </table>"),
                format!(
                    "<tr><td colspan=\"{span}\" class=\"h-12 animate-pulse rounded-md bg-slate-100\"></td></tr>"
                ),
            )
        } else {
            (
                "<ul class=\"mt-6 space-y-2\">".to_string(),
                "</ul>".to_string(),
                "<li class=\"h-12 animate-pulse rounded-md bg-slate-100\"></li>".to_string(),
            )
        };
        if fetched {
            let _ = writeln!(out, "{pad}  {list_open}");
            let _ = writeln!(out, "{pad}    {{#each [0, 1, 2] as n (n)}}");
            let _ = writeln!(out, "{pad}      {row}");
            let _ = writeln!(out, "{pad}    {{/each}}");
            let _ = writeln!(out, "{pad}  {list_close}");
        }
        // `{#if}` rather than `{:else if}` when there was no branch before it.
        let _ = writeln!(
            out,
            "{pad}{}{visible}.length === 0}}",
            if fetched { "{:else if " } else { "{#if " }
        );
        match empty {
            Some(e) => out.push_str(&self.plain(e, depth + 1, &Ctx::default())),
            None => {
                let _ = writeln!(
                    out,
                    "{pad}  <p class=\"mt-10 text-center text-sm text-slate-500\">Nothing here yet.</p>"
                );
            }
        }
        let _ = writeln!(out, "{pad}{{:else}}");

        // `table` lowers to a real table here too. Not doing so would have been the drift invariant 8
        // forbids in its most visible form: the same document, tabular in React and a bare list in Svelte.
        if crate::is_tabular(el) {
            crate::check_columns(self.diags, "svelte", el, template.len());
            let _ = writeln!(out, "{pad}  <table class={:?}>", classes("table", &modifiers_of(el)));
            if let Some(head) = crate::table_head(el, &pad, "class") {
                out.push_str(&head);
            }
            let _ = writeln!(out, "{pad}    <tbody>");
            let _ = writeln!(out, "{pad}      {{#each {visible} as item ({key})}}");
            let _ = writeln!(out, "{pad}        <tr>");
            let cell_class = classes("td", &[]);
            let cell_attr = if cell_class.is_empty() {
                String::new()
            } else {
                format!(" class={cell_class:?}")
            };
            for child in &template {
                let _ = writeln!(out, "{pad}          <td{cell_attr}>");
                out.push_str(&self.element(child, depth + 6, &row_ctx));
                let _ = writeln!(out, "{pad}          </td>");
            }
            let _ = writeln!(out, "{pad}        </tr>");
            let _ = writeln!(out, "{pad}      {{/each}}");
            let _ = writeln!(out, "{pad}    </tbody>");
            let _ = writeln!(out, "{pad}  </table>");
            let _ = writeln!(out, "{pad}{{/if}}");
            return out;
        }

        let _ = writeln!(out, "{pad}  <ul class={:?}>", classes(&el.tag, &modifiers_of(el)));
        let _ = writeln!(out, "{pad}    {{#each {visible} as item ({key})}}");
        let _ = writeln!(out, "{pad}      <li class=\"flex items-center gap-3 px-3 py-3\">");
        for child in &template {
            out.push_str(&self.element(child, depth + 4, &row_ctx));
        }
        let _ = writeln!(out, "{pad}      </li>");
        let _ = writeln!(out, "{pad}    {{/each}}");
        let _ = writeln!(out, "{pad}  </ul>");
        let _ = writeln!(out, "{pad}{{/if}}");
        out
    }

    /// A segmented control over an enumerated state's domain.
    fn tabs(&mut self, el: &Element, depth: usize) -> String {
        let pad = "  ".repeat(depth);
        let Some(name) = el.label().map(str::to_string) else { return String::new() };
        let Some(state) = self.program.state(&name) else { return String::new() };
        if state.domain.is_empty() {
            unsupported_in(
                self.diags,
                "svelte",
                el.span,
                format!("`tabs {name}` needs an enumerated state"),
            );
            return String::new();
        }
        let options: Vec<String> = state.domain.iter().map(|d| format!("{d:?}")).collect();
        let mut out = String::new();
        let _ = writeln!(out, "{pad}<div class=\"mt-4 flex gap-2\">");
        let _ = writeln!(out, "{pad}  {{#each [{}] as option (option)}}", options.join(", "));
        let _ = writeln!(out, "{pad}    <button");
        let _ = writeln!(out, "{pad}      type=\"button\"");
        let _ = writeln!(out, "{pad}      aria-pressed={{{name} === option}}");
        let _ = writeln!(out, "{pad}      onclick={{() => ({name} = option)}}");
        let _ = writeln!(
            out,
            "{pad}      class={{{name} === option ? \"rounded-full bg-slate-900 px-3 py-1 text-xs font-medium text-white\" : \"rounded-full border border-slate-300 px-3 py-1 text-xs text-slate-600\"}}"
        );
        let _ = writeln!(out, "{pad}    >{{option}}</button>");
        let _ = writeln!(out, "{pad}  {{/each}}");
        let _ = writeln!(out, "{pad}</div>");
        out
    }

    /// `<details>`, which needs no framework in any backend.
    fn faq(&mut self, el: &Element, depth: usize) -> String {
        let pad = "  ".repeat(depth);
        let open = el.attr("open").and_then(|v| v.as_text().map(str::to_string));
        let open_index: Option<usize> = open.and_then(|v| v.parse().ok());
        let mut out = String::new();
        let _ = writeln!(out, "{pad}<div class=\"mt-8 divide-y divide-slate-200\">");
        for (i, line) in el.text_lines.iter().enumerate() {
            let (q, a) = line.split_once('|').unwrap_or((line.as_str(), ""));
            let is_open = open_index.is_some_and(|n| n == i + 1);
            let _ = writeln!(
                out,
                "{pad}  <details class=\"py-3\"{}>",
                if is_open { " open" } else { "" }
            );
            let _ = writeln!(
                out,
                "{pad}    <summary class=\"cursor-pointer text-sm font-medium\">{}</summary>",
                q.trim()
            );
            let _ =
                writeln!(out, "{pad}    <p class=\"mt-2 text-sm text-slate-600\">{}</p>", a.trim());
            let _ = writeln!(out, "{pad}  </details>");
        }
        let _ = writeln!(out, "{pad}</div>");
        out
    }

    fn item_fields(&self, resource: &str) -> Vec<String> {
        let Some(r) = self.program.resources.iter().find(|r| r.name == resource) else {
            return Vec::new();
        };
        let ty = r.ty.trim_end_matches("[]");
        self.program
            .types
            .iter()
            .find(|t| t.name == ty)
            .map(|t| t.fields.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Statements separated by `;`. Assignment is plain in Svelte — a rune is a variable, so there is
    /// no setter to call, which is most of why the emitted file is smaller than the React one.
    /// The lowered statements of a declared effect, through the same action path a button uses.
    fn effect_body(&mut self, e: &guml_ast::Effect) -> String {
        let ctx = Ctx::default().with_collections(self.collections).with_row_bool(self.row_bool);
        e.actions.iter().map(|a| self.lower_action(a, &ctx, None)).collect::<Vec<_>>().join(" ")
    }

    /// `el` is `None` for a declared `on` effect, which has no element. Everything else about the
    /// lowering is identical — an effect that ran a different action language from a button's would be
    /// a second thing to learn for no gain.
    fn lower_action(&mut self, action: &str, ctx: &Ctx, el: Option<&Element>) -> String {
        // A `check {done}` whose action is a bare mutation is a toggle: the body is the negated field.
        // Deriving it here matches the React backend, and without it the mutation would post `{}` and
        // silently save nothing.
        let toggle = el
            .filter(|e| e.tag == "check" || e.tag == "toggle")
            .and_then(|e| e.binding().map(|b| b.trim().to_string()));
        let mut stmts: Vec<String> = Vec::new();
        for raw in action.split(';') {
            let stmt = raw.trim();
            if stmt.is_empty() {
                continue;
            }
            if let Some(name) = stmt.strip_suffix("++") {
                stmts.push(format!("{}++", name.trim()));
                continue;
            }
            if let Some(name) = stmt.strip_suffix("--") {
                stmts.push(format!("{}--", name.trim()));
                continue;
            }
            // A resource mutation: `tasks.add{title:draft}`.
            if let Some((head, rest)) = stmt.split_once('.')
                && self.collections.iter().any(|c| c == head.trim())
            {
                let call = rest.split('{').next().unwrap_or(rest).trim();
                // `>tasks.list` re-runs the resource's own GET. No body, no row, either way.
                if call == "list" {
                    stmts.push(format!("{}List()", head.trim()));
                    continue;
                }
                let body = match (&toggle, rest.contains('{')) {
                    (Some(field), false) => format!("{{ {field}: !item.{field} }}"),
                    _ => rest
                        .split_once('{')
                        .map(|(_, b)| {
                            let inner = b.trim_end_matches('}');
                            let pairs: Vec<String> = inner
                                .split(',')
                                .filter_map(|p| p.split_once(':'))
                                .map(|(k, v)| {
                                    format!("{}: {}", k.trim(), expr::lower_in(v.trim(), ctx))
                                })
                                .collect();
                            format!("{{ {} }}", pairs.join(", "))
                        })
                        .unwrap_or_else(|| "{}".to_string()),
                };
                let item = if ctx.item_fields.is_empty() { "" } else { "item, " };
                stmts.push(format!("{}{}({item}{body})", head.trim(), capitalize(call)));
                continue;
            }
            if let Some((lhs, rhs)) = stmt.split_once('=') {
                stmts.push(format!("{} = {}", lhs.trim(), expr::lower_in(rhs.trim(), ctx)));
                continue;
            }
            stmts.push(expr::lower_in(stmt, ctx));
        }
        stmts.iter().map(|s| format!("{s};")).collect::<Vec<_>>().join(" ")
    }
}

/// `$state` for the rows, `$effect` for the fetch, a plain async function per mutation.
/// The plain-JavaScript cache helper, indented to sit inside a Svelte `<script>` block.
///
/// The constant is written at module indentation because the React backend emits it at a file's top
/// level. Re-indenting here rather than keeping a third copy of the policy: a third copy is a third
/// chance for one of them to be fixed and the others not.
fn indent_js(src: &str) -> String {
    src.lines()
        .map(|l| if l.trim().is_empty() { String::from("\n") } else { format!("  {l}") + "\n" })
        .collect()
}

fn resource_runes(r: &Resource) -> String {
    let name = &r.name;
    let url = &r.url;
    let mut out = String::new();

    let _ = writeln!(out, "  // resource `{name}`");
    let _ = writeln!(out, "  let {name} = $state([]);");
    let _ = writeln!(out, "  let {name}Loading = $state(true);");
    let _ = writeln!(out, "  let {name}Error = $state(null);");
    let _ = writeln!(out, "  let {name}Saving = $state(false);");

    // Named, so `>tasks.list` can re-run it — from a Reload button, or from a declared `on` effect.
    // `$effect` then takes the function directly: it returns its own teardown, so the abort needs no
    // dependency array and no wrapper closure.
    let _ = write!(
        out,
        "\n  function {name}List() {{\n\
         \x20   const controller = new AbortController();\n\
         \x20   {name}Loading = true;\n\
         \x20   {name}Error = null;\n\
         \x20   cached({url:?}, {{ signal: controller.signal }})\n\
         \x20     .then((rows) => ({name} = rows))\n\
         \x20     .catch((err) => {{\n\
         \x20       if (err.name === \"AbortError\") return;\n\
         \x20       {name}Error = err.message ?? \"Unknown error\";\n\
         \x20     }})\n\
         \x20     .finally(() => ({name}Loading = false));\n\
         \x20   return () => controller.abort();\n\
         \x20 }}\n\
         \n  $effect({name}List);\n"
    );

    for m in &r.mutations {
        let fname = format!("{name}{}", capitalize(&m.name));
        let method = &m.method;
        let murl = &m.url;
        // Optimistic apply and rollback, the same shape the React backend emits — the snapshot is a
        // plain variable here because a rune is a variable.
        let _ = write!(
            out,
            "\n  async function {fname}(item, body = {{}}) {{\n\
             \x20   const snapshot = {name};\n\
             \x20   {name}Saving = true;\n"
        );
        match m.optimistic.as_deref() {
            Some("prepend") => {
                let _ = writeln!(out, "    {name} = [{{ ...body }}, ...{name}];");
            }
            Some("append") => {
                let _ = writeln!(out, "    {name} = [...{name}, {{ ...body }}];");
            }
            Some(_) if method == "DELETE" => {
                let _ = writeln!(out, "    {name} = {name}.filter((it) => it !== item);");
            }
            Some(_) => {
                let _ = writeln!(
                    out,
                    "    {name} = {name}.map((it) => (it === item ? {{ ...it, ...body }} : it));"
                );
            }
            None => {}
        }
        let _ = write!(
            out,
            "    try {{\n\
             \x20     const res = await retrying(`{}`, {{\n\
             \x20       method: {method:?},\n\
             \x20       headers: {{ \"Content-Type\": \"application/json\" }},\n\
             \x20       body: JSON.stringify(body),\n\
             \x20     }});\n\
             \x20     if (!res.ok) throw new Error(`Request failed: ${{res.status}}`);\n\
             \x20     invalidate({prefix:?});\n\
             \x20   }} catch (err) {{\n\
             \x20     {name} = snapshot;\n\
             \x20     {name}Error = err.message ?? \"Could not save\";\n\
             \x20   }} finally {{\n\
             \x20     {name}Saving = false;\n\
             \x20   }}\n\
             \x20 }}\n",
            murl.replace("{id}", "${item.id}"),
            // Without this the next read is a cache hit on the pre-mutation list, so the row the user just
            // added visibly disappears — and it reads as a broken optimistic update rather than a stale
            // cache. Same prefix rule as the React backend, from the same shared function.
            prefix = crate::invalidation_prefix(url)
        );
    }
    out
}

/// `$derived` for every `where=` filter. The React backend needs `useMemo` plus a hand-built
/// dependency array for the same thing; Svelte tracks it, which is the compile-away-the-framework
/// argument in one line.
fn collect_derived(els: &[Element], program: &Program, out: &mut String) {
    for el in els {
        if matches!(el.tag.as_str(), "list" | "table")
            && let Some(source) = el.label()
            && let Some(Value::Binding(b)) = el.attr("where")
        {
            let filter = b.source.trim();
            let fields = program
                .resources
                .iter()
                .find(|r| r.name == source)
                .map(|r| r.ty.trim_end_matches("[]").to_string())
                .and_then(|ty| program.types.iter().find(|t| t.name == ty))
                .map(|t| t.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>())
                .unwrap_or_default();

            // `crate::where_filter` decides this, not a paraphrase of it. The comment here used to say
            // "matching the React backend so the two agree" above a hand-copied ternary — which is
            // agreement by inspection, and it stopped being true the moment React's version was fixed
            // for a boolean field not called `done`.
            let domain = program.state(filter).map(|s| s.domain.clone()).unwrap_or_default();
            let flag = crate::row_bool_fields(program)
                .into_iter()
                .find(|(c, _)| c == source)
                .map(|(_, f)| f);
            let text_fields = crate::search_fields(program, source, filter);
            let expr_src = crate::where_filter(
                source,
                filter,
                &domain,
                &fields,
                &text_fields,
                flag.as_deref(),
            )
            // A single line here: a Svelte `$derived` is one expression, and the multi-line
            // shape React needs inside `useMemo` would only add noise.
            .map(|body| body.replace('\n', "").split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| source.to_string());
            let _ = writeln!(out, "  const visible{} = $derived({expr_src});", capitalize(source));
        }
        collect_derived(&el.children, program, out);
    }
}

fn js_blocks(els: &[Element]) -> Vec<&Vec<String>> {
    let mut out = Vec::new();
    for el in els {
        if el.tag == "js" {
            out.push(&el.text_lines);
        }
        out.extend(js_blocks(&el.children));
    }
    out
}

fn attr_value(value: &Value, ctx: &Ctx) -> String {
    match value {
        Value::Binding(b) => format!("{{{}}}", expr::lower_expr(&b.expr, ctx)),
        // Interpolations inside an attribute string need the row qualifier: `aria="Delete {title}"`
        // must become `item.title`, or the template references a name that is not in scope. The
        // `{`…`}` form this emits is valid in Svelte as well as JSX.
        Value::Str(t) => expr::lower_attr_value_in(t, ctx),
        Value::Num(n) => format!("{{{n}}}"),
        Value::Bool(b) => format!("{{{b}}}"),
        Value::Word(w) => format!("{w:?}"),
        Value::Flag => "{true}".to_string(),
    }
}

fn attr_text(el: &Element, name: &str) -> Option<String> {
    el.attr(name).and_then(|v| v.as_text().map(str::to_string))
}

fn initial(init: &Value, domain: &[String]) -> String {
    if let Some(first) = domain.first() {
        return format!("{first:?}");
    }
    match init {
        Value::Num(n) => format!("{n}"),
        Value::Bool(b) => format!("{b}"),
        Value::Str(s) => format!("{s:?}"),
        Value::Word(w) => format!("{w:?}"),
        _ => "\"\"".to_string(),
    }
}

/// The element this backend emits for a tag.
///
/// Delegates to [`crate::element_for`], shared with the React and static-HTML backends. This was a
/// third copy of the table, and like the second it had already drifted: `hero` fell through to `div`
/// here while React emitted `<header>`, and every 0.2 tag would have silently become a `<div>` —
/// a `divider` as an empty box, a `stepper` with no list semantics, a `progress` with no bar.
fn html_tag(tag: &str) -> &'static str {
    crate::element_for(tag).unwrap_or("div")
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
