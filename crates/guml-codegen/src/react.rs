//! React + TypeScript + Tailwind backend, including the desugar pass.
//!
//! This is where "convention as compression" is actually cashed out. A single
//! `data` declaration expands to state, an aborting fetch effect, one callback per
//! mutation with optimistic application and rollback, and loading / empty / error
//! rendering — roughly 1,200 tokens of React the model never writes and therefore
//! cannot get wrong.
//!
//! Two invariants hold this together:
//!
//! * **The design system lives in `classes`.** Every string there is a token the
//!   model no longer emits, and swapping the table re-themes every compiled page.
//! * **Expressions go through `crate::expr`,** which mirrors the browser runtime's
//!   evaluator. A preview that disagrees with emitted code is worse than none.

use crate::expr::{self, Ctx};
use crate::{Backend, Emitted, OutFile, component_name, modifiers_of, setter, unsupported};
use guml_ast::{Element, Positional, Program, Resource, Value};
use guml_diagnostics::Diagnostics;
use std::collections::BTreeSet;
use std::fmt::Write as _;

#[derive(Debug, Default)]
pub struct ReactBackend;

impl Backend for ReactBackend {
    fn name(&self) -> &'static str {
        "react"
    }

    fn emit(&self, program: &Program) -> Emitted {
        let mut out = Emitted::default();
        let name = component_name(program.page.as_ref().map(|p| p.name.as_str()).unwrap_or("Page"));

        let mut g = Gen {
            program,
            diags: &mut out.diagnostics,
            hooks: Hooks::default(),
            marks: Vec::new(),
            collections: program.resources.iter().map(|r| r.name.clone()).collect(),
            pending: None,
        };
        let (body, body_offsets) = g.tree();
        let hooks = g.hooks.clone();

        // Dead-declaration elimination. A `state` or `data` nothing refers to becomes ~1 line of
        // `useState` or ~60 lines of fetch/effect/callbacks that can never run, and in the resource
        // case a network request on mount for data no element reads.
        //
        // The live set is `guml_ast::referenced_names`, which is the *same* function the validator
        // uses to decide `GUML0074`/`GUML0075`. That sharing is what makes this safe to do silently:
        // anything elided here has already been reported to the author as unused, and a reference
        // form the walker knows about — including a bare mention inside a `js` body — keeps the
        // declaration alive.
        let live = guml_ast::referenced_names(program);
        let states: Vec<_> = program.states.iter().filter(|s| live.contains(&s.name)).collect();
        let resources: Vec<_> =
            program.resources.iter().filter(|r| live.contains(&r.name)).collect();

        let mut src = String::new();

        // Imports, driven by what the body actually needed — after elimination, so an elided
        // declaration does not leave an unused import behind.
        let mut imports: Vec<&str> = Vec::new();
        if !states.is_empty() || !resources.is_empty() {
            imports.push("useState");
        }
        if !resources.is_empty() {
            imports.push("useCallback");
            imports.push("useEffect");
        }
        if hooks.needs_memo {
            imports.push("useMemo");
        }
        if !imports.is_empty() {
            let _ = writeln!(src, "import {{ {} }} from \"react\";\n", imports.join(", "));
        }

        // Types, so emitted code is typed rather than `any`-shaped.
        for ty in &program.types {
            let fields = ty
                .fields
                .iter()
                .map(|f| format!("{}: {}", f.name, ts_type(&f.ty)))
                .collect::<Vec<_>>()
                .join("; ");
            let _ = writeln!(src, "type {} = {{ {fields} }};\n", ty.name);
        }

        for (_, decl) in &hooks.hoisted {
            let _ = writeln!(src, "{decl}\n");
        }

        let _ = writeln!(src, "export default function {name}() {{");

        // Provenance. Every declaration and every element, nested ones included — `Gen::tree`
        // explains how a nested position is known.
        //
        // Still line granularity, not column: one GUML line becomes a *region* of TSX, so a column
        // claim would send a debugger to an arbitrary character. What nesting adds is that a binding
        // error inside a row template now resolves to the row's own line instead of to the `list`
        // twenty lines above it.
        let mut map = crate::sourcemap::SourceMap::new();
        let line_of = |text: &str| text.lines().count() as u32;

        for s in &states {
            map.mark(line_of(&src), s.span.line);
            let ty = state_type(&s.init, &s.domain);
            let _ = writeln!(
                src,
                "  const [{}, {}] = useState{ty}({});",
                s.name,
                setter(&s.name),
                initial(&s.init)
            );
        }

        let busy = busy_resources(&program.tree, None);
        for r in &resources {
            src.push('\n');
            // One `data` line becomes ~60 lines of state, effect and callbacks. This is the
            // mapping that matters most: a failed fetch should point at the declaration.
            map.mark(line_of(&src), r.span.line);
            src.push_str(&resource_hooks(r, busy.contains(&r.name)));
        }

        for derived in &hooks.derived {
            src.push('\n');
            src.push_str(derived);
        }

        // `js` blocks are component-body code, so they are hoisted here: after the state and the
        // resource hooks they will want to reference, and before the return. Emitted verbatim —
        // not checked, not reformatted, not escaped. That is the deal, and the `GUML0090` note
        // already said so.
        for block in js_blocks(&program.tree) {
            src.push('\n');
            for line in block {
                let _ = writeln!(src, "  {line}");
            }
        }

        let _ = writeln!(src, "\n  return (");

        // The body was rendered before this header existed, so every offset inside it shifts by
        // the number of lines that precede the JSX — plus one when a fragment wraps it.
        let body_start = line_of(&src) + u32::from(program.tree.len() > 1);
        for (offset, source_line) in &body_offsets {
            map.mark(body_start + offset, *source_line);
        }

        if body.trim().is_empty() {
            src.push_str("    <></>\n");
        } else if program.tree.len() > 1 {
            // JSX allows exactly one root, and a page is normally several siblings.
            // `tsc` caught this on both multi-section fixtures.
            src.push_str("    <>\n");
            for line in body.lines() {
                let _ = writeln!(src, "  {line}");
            }
            src.push_str("    </>\n");
        } else {
            src.push_str(&body);
        }
        src.push_str("  );\n}\n");

        // Serialised by the driver, which is the layer that holds the source text.
        let source_map = (!map.is_empty()).then_some(map);
        out.files.push(OutFile { path: format!("{name}.tsx"), contents: src, source_map });
        out
    }
}

/// `filter` → `FILTER`, `taskFilter` → `TASK_FILTER`. Module constants are screaming snake by
/// convention, and the emitted file has to read like one a person wrote.
fn screaming_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(ch.to_uppercase());
    }
    out
}

/// Anything the body decides the component needs above the JSX.
#[derive(Debug, Clone, Default)]
struct Hooks {
    needs_memo: bool,
    derived: Vec<String>,
    /// Module-scope constants hoisted out of the render: `(name, declaration)`, deduplicated by
    /// name because two `tabs` over the same state produce the same array.
    hoisted: Vec<(String, String)>,
}

struct Gen<'a> {
    program: &'a Program,
    /// `(line within the current renderer's output, GUML source line)`. Re-based by parents as
    /// child output is appended; see `Gen::tree`.
    marks: Vec<(u32, u32)>,
    diags: &'a mut Diagnostics,
    hooks: Hooks,
    /// Resource names, built once. Rebuilding this per element cost 5 ms on a 200-line
    /// document — the benchmark caught it immediately, which is the argument for having one.
    collections: Vec<String>,
    /// Loading flag of the resource the enclosing form submits to. A `busy` label
    /// belongs to the button, but the mutation is declared on the form around it.
    pending: Option<String>,
}

impl<'a> Gen<'a> {
    /// The JSX body, plus where *every* element landed in it — nested ones included.
    ///
    /// # How nested positions are known
    ///
    /// The renderers return strings, so a parent concatenating them cannot see inside a child. Each
    /// renderer therefore records its marks relative to *its own* output, and the parent re-bases
    /// them by the line at which it appended that output. `collect` is what makes that composable:
    /// it swaps `self.marks` out, runs the renderer, and hands back only what that renderer pushed.
    ///
    /// The alternative — threading an absolute line number down through every renderer — puts the
    /// same arithmetic at every call site instead of at the three places children are appended.
    fn tree(&mut self) -> (String, Vec<(u32, u32)>) {
        let mut out = String::new();
        let ctx = Ctx::default().with_collections(&self.collections);
        let (body, marks) = self.collect(|g| {
            for el in &g.program.tree {
                let at = out.lines().count() as u32;
                let (text, marks) = g.element_marked(el, 2, &ctx);
                out.push_str(&text);
                g.rebase(marks, at);
            }
            std::mem::take(&mut out)
        });
        (body, marks)
    }

    /// Run a renderer and take only the marks it produced, relative to its own output.
    fn collect<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> (T, Vec<(u32, u32)>) {
        let outer = std::mem::take(&mut self.marks);
        let value = f(self);
        let inner = std::mem::replace(&mut self.marks, outer);
        (value, inner)
    }

    /// Shift a child's marks by where its output was appended, and keep them.
    fn rebase(&mut self, marks: Vec<(u32, u32)>, at: u32) {
        self.marks.extend(marks.into_iter().map(|(line, source)| (line + at, source)));
    }

    /// Attribute the next line of `out` to `source_line`.
    ///
    /// A mapping is a *range*: every emitted line inherits the last mark at or before it. So after a
    /// child's output is appended, the lines that follow still belong to the child unless the parent
    /// says otherwise — which is how a repeater's own `<ul>`/`<li>` scaffolding ended up credited to
    /// whichever element happened to be emitted last.
    fn mark_here(&mut self, out: &str, source_line: u32) {
        self.marks.push((out.lines().count() as u32, source_line));
    }

    /// An element's output, with a mark for the element itself plus everything inside it.
    fn element_marked(
        &mut self,
        el: &Element,
        depth: usize,
        ctx: &Ctx,
    ) -> (String, Vec<(u32, u32)>) {
        let (text, mut marks) = self.collect(|g| g.element(el, depth, ctx));
        // The element's own output starts at its own line 0. An escape block emits nothing here, so
        // marking it would claim a line that belongs to whatever follows.
        if !text.is_empty() {
            marks.insert(0, (0, el.span.line));
        }
        (text, marks)
    }

    fn resource(&self, name: &str) -> Option<&'a Resource> {
        self.program.resources.iter().find(|r| r.name == name)
    }

    /// Fields of a resource's item type, so row bindings can be qualified.
    fn item_fields(&self, resource: &str) -> Vec<String> {
        let Some(r) = self.resource(resource) else { return Vec::new() };
        let ty = r.ty.trim_end_matches("[]");
        self.program
            .types
            .iter()
            .find(|t| t.name == ty)
            .map(|t| t.fields.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default()
    }

    fn element(&mut self, el: &Element, depth: usize, ctx: &Ctx) -> String {
        match el.tag.as_str() {
            "list" | "table" => self.repeater(el, depth),
            "tabs" => self.tabs(el, depth),
            "faq" => self.faq(el, depth),
            "tier" => self.tier(el, depth),
            _ => self.plain(el, depth, ctx),
        }
    }

    // ------------------------------------------------------------ repeaters

    /// `list tasks where={filter}` → a filtered memo, a keyed map, and the
    /// loading / empty / error states the author never wrote.
    fn repeater(&mut self, el: &Element, depth: usize) -> String {
        let pad = " ".repeat(depth * 2);
        let Some(source) = el.label().map(str::to_string) else {
            unsupported(self.diags, el.span, "a repeater with no source");
            return String::new();
        };
        if self.resource(&source).is_none() {
            unsupported(self.diags, el.span, format!("`{}` over undeclared `{source}`", el.tag));
            return String::new();
        }

        let fields = self.item_fields(&source);
        let ctx = Ctx::item(&fields).with_collections(&self.collections);
        let cap = capitalize(&source);
        let visible = format!("visible{cap}");

        // The filter is a derived value, so it becomes a memo rather than state.
        //
        // What it filters *by* has to come from the enumerated state's domain and the row
        // type. The first version hardcoded fixture B's shape — `open`/`done` against a
        // `done` field — so `where={area}` over `all|compilers|web|research` emitted
        // comparisons that could never match and `.done` on a type without it. It compiled;
        // only `tsc` over the output caught it. That is a silent mis-lowering, which
        // invariant 3 forbids.
        if let Some(Value::Binding(b)) = el.attr("where") {
            let filter = expr::lower_expr(&b.expr, &Ctx::default());
            let domain = self
                .program
                .states
                .iter()
                .find(|st| st.name == filter)
                .map(|st| st.domain.clone())
                .unwrap_or_default();
            let has_done = fields.iter().any(|f| f == "done");
            let matching_field = fields.iter().find(|f| *f == &filter).cloned();
            // A member literally named `all` conventionally means "do not filter".
            let has_all = domain.iter().any(|m| m == "all");

            if has_done && domain.iter().any(|m| m == "open") {
                // The boolean idiom: `open` and `done` describe one boolean field.
                self.hooks.needs_memo = true;
                self.hooks.derived.push(format!(
                    "  const {visible} = useMemo(
    () =>
      {filter} === \"open\"
        ? {source}.filter((it) => !it.done)
        : {filter} === \"done\"
          ? {source}.filter((it) => it.done)
          : {source},
    [{source}, {filter}],
  );
"
                ));
            } else if let Some(field) = matching_field {
                // The general case: a row field of the same name holds the member.
                self.hooks.needs_memo = true;
                let body = if has_all {
                    format!(
                        "{filter} === \"all\" ? {source} : {source}.filter((it) => it.{field} === {filter})"
                    )
                } else {
                    format!("{source}.filter((it) => it.{field} === {filter})")
                };
                self.hooks.derived.push(format!(
                    "  const {visible} = useMemo(() => {body}, [{source}, {filter}]);
"
                ));
            } else {
                // Nothing to filter on. Warn and render everything rather than invent a
                // predicate: an unfiltered list is visibly wrong, a wrong filter is not.
                unsupported(
                    self.diags,
                    el.span,
                    format!(
                        "`where={{{filter}}}` — the row type has no `{filter}` field and the domain is not the open/done idiom, so the list is not filtered"
                    ),
                );
                self.hooks.derived.push(format!(
                    "  const {visible} = {source};
"
                ));
            }
        } else {
            self.hooks.derived.push(format!(
                "  const {visible} = {source};
"
            ));
        }

        let empty = el.children.iter().find(|c| c.tag == "empty").cloned();
        let template: Vec<Element> =
            el.children.iter().filter(|c| c.tag != "empty").cloned().collect();

        let (rows, row_marks) = self.collect(|g| {
            let mut rows = String::new();
            for child in &template {
                let at = rows.lines().count() as u32;
                let (text, marks) = g.element_marked(child, depth + 4, &ctx);
                rows.push_str(&text);
                g.rebase(marks, at);
            }
            rows
        });

        let key = if fields.iter().any(|f| f == "id") { "item.id" } else { "i" };
        let list_class = classes(&el.tag, &modifiers_of(el));
        let mut out = String::new();

        // Error first: a failed request is the most important thing on screen.
        let _ = writeln!(
            out,
            "{pad}{{{source}Error && (\n{pad}  <p role=\"alert\" className=\"mt-4 rounded-md bg-red-50 px-3 py-2 text-sm text-red-700\">\n{pad}    {{{source}Error}}\n{pad}  </p>\n{pad})}}"
        );

        let _ = writeln!(out, "{pad}{{{source}Loading ? (");
        let _ = writeln!(
            out,
            "{pad}  <ul className=\"mt-6 space-y-2\">\n{pad}    {{[0, 1, 2].map((n) => (\n{pad}      <li key={{n}} className=\"h-12 animate-pulse rounded-md bg-slate-100\" />\n{pad}    ))}}\n{pad}  </ul>"
        );
        let _ = writeln!(out, "{pad}) : {visible}.length === 0 ? (");
        match &empty {
            Some(e) => {
                let ctx = Ctx::default().with_collections(&self.collections);
                let at = out.lines().count() as u32;
                let (text, marks) = self.element_marked(e, depth + 1, &ctx);
                out.push_str(&text);
                self.rebase(marks, at);
            }
            None => {
                let _ = writeln!(
                    out,
                    "{pad}  <p className=\"mt-10 text-center text-sm text-slate-500\">Nothing here yet.</p>"
                );
            }
        }
        // Back to the repeater: the list scaffolding is the `list` line's, not the empty slot's.
        self.mark_here(&out, el.span.line);
        let _ = writeln!(out, "{pad}) : (");
        let _ = writeln!(out, "{pad}  <ul className={list_class:?}>");
        let _ = writeln!(out, "{pad}    {{{visible}.map((item, i) => (");
        let _ = writeln!(
            out,
            "{pad}      <li key={{{key}}} className=\"flex items-center gap-3 px-3 py-3\">"
        );
        let rows_at = out.lines().count() as u32;
        out.push_str(&rows);
        self.rebase(row_marks, rows_at);
        // The closing tags belong to the repeater, not to the last child of the row template.
        self.mark_here(&out, el.span.line);
        let _ = writeln!(out, "{pad}      </li>");
        let _ = writeln!(out, "{pad}    ))}}");
        let _ = writeln!(out, "{pad}  </ul>");
        let _ = writeln!(out, "{pad})}}");

        out
    }

    /// `tabs filter` → a segmented control over the state's enumerated domain.
    fn tabs(&mut self, el: &Element, depth: usize) -> String {
        let pad = " ".repeat(depth * 2);
        let Some(name) = el.label().map(str::to_string) else {
            unsupported(self.diags, el.span, "`tabs` with no bound state");
            return String::new();
        };
        let Some(state) = self.program.state(&name) else {
            unsupported(self.diags, el.span, format!("`tabs` over undeclared `{name}`"));
            return String::new();
        };
        if state.domain.is_empty() {
            unsupported(
                self.diags,
                el.span,
                format!("`tabs {name}` needs an enumerated state, e.g. `state {name}=a|b|c`"),
            );
            return String::new();
        }

        let options = state.domain.iter().map(|d| format!("{d:?}")).collect::<Vec<_>>().join(", ");
        let set = setter(&name);

        // Static hoist. The option array is a constant derived from the state's declared domain, so
        // rebuilding it on every render allocates for nothing. Hoisting it to module scope is also
        // what a person would write — which matters more than the allocation, because emitted code
        // that does not read as hand-written is the first thing a reviewer holds against a compiler.
        let konst = format!("{}_OPTIONS", screaming_snake(&name));
        if !self.hooks.hoisted.iter().any(|(n, _)| *n == konst) {
            self.hooks
                .hoisted
                .push((konst.clone(), format!("const {konst} = [{options}] as const;")));
        }

        format!(
            "{pad}<div className=\"mt-4 flex gap-2\">\n\
             {pad}  {{{konst}.map((option) => (\n\
             {pad}    <button\n\
             {pad}      key={{option}}\n\
             {pad}      type=\"button\"\n\
             {pad}      aria-pressed={{{name} === option}}\n\
             {pad}      onClick={{() => {set}(option)}}\n\
             {pad}      className={{\n\
             {pad}        {name} === option\n\
             {pad}          ? \"rounded-full bg-slate-900 px-3 py-1 text-xs font-medium text-white\"\n\
             {pad}          : \"rounded-full border border-slate-300 px-3 py-1 text-xs text-slate-600\"\n\
             {pad}      }}\n\
             {pad}    >\n\
             {pad}      {{option}}\n\
             {pad}    </button>\n\
             {pad}  ))}}\n\
             {pad}</div>\n"
        )
    }

    /// `faq` children are `question | answer` lines. Emitted as `<details>`, which
    /// is keyboard accessible and needs no state of its own.
    fn faq(&mut self, el: &Element, depth: usize) -> String {
        let pad = " ".repeat(depth * 2);
        let items = el
            .text_lines
            .iter()
            .map(|line| {
                let (q, a) = line.split_once('|').unwrap_or((line.as_str(), ""));
                format!("{pad}    {{ q: {:?}, a: {:?} }},", q.trim(), a.trim())
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "{pad}<div className=\"mt-8 divide-y divide-slate-200 border-y border-slate-200\">\n\
             {pad}  {{[\n{items}\n{pad}  ].map((item) => (\n\
             {pad}    <details key={{item.q}} className=\"py-4\">\n\
             {pad}      <summary className=\"cursor-pointer text-sm font-medium text-slate-900\">\n\
             {pad}        {{item.q}}\n\
             {pad}      </summary>\n\
             {pad}      <p className=\"mt-2 text-sm text-slate-600\">{{item.a}}</p>\n\
             {pad}    </details>\n\
             {pad}  ))}}\n\
             {pad}</div>\n"
        )
    }

    /// `tier` positionals are name / price / blurb; children are perk lines.
    fn tier(&mut self, el: &Element, depth: usize) -> String {
        let pad = " ".repeat(depth * 2);
        let mut texts = el.positionals.iter().filter_map(|p| match p {
            guml_ast::Positional::Text(t) => Some(t.as_str()),
            _ => None,
        });
        let name = texts.next().unwrap_or("");
        let price = texts.next().unwrap_or("");
        let blurb = texts.next().unwrap_or("");
        let featured = el.has_modifier("featured");
        let cta = el.attr("cta").and_then(|v| v.as_text()).unwrap_or("Choose").to_string();
        let href = el.route().unwrap_or("#").to_string();

        let perks = el
            .text_lines
            .iter()
            .map(|p| format!("{pad}    <li>• {}</li>", jsx_escape(p)))
            .collect::<Vec<_>>()
            .join("\n");

        let card = if featured {
            "rounded-xl border-2 border-slate-900 p-6 shadow-sm"
        } else {
            "rounded-xl border border-slate-200 p-6"
        };
        let button = if featured {
            "mt-6 block rounded-md bg-slate-900 px-4 py-2 text-center text-sm font-medium text-white"
        } else {
            "mt-6 block rounded-md border border-slate-300 px-4 py-2 text-center text-sm font-medium text-slate-700"
        };

        let name = jsx_escape(name);
        let price = jsx_escape(price);
        let blurb = jsx_escape(blurb);
        let cta = jsx_escape(&cta);

        format!(
            "{pad}<div className={card:?}>\n\
             {pad}  <h3 className=\"font-medium\">{name}</h3>\n\
             {pad}  <p className=\"mt-1 text-sm text-slate-500\">{blurb}</p>\n\
             {pad}  <p className=\"mt-4 text-3xl font-semibold\">{price}</p>\n\
             {pad}  <ul className=\"mt-6 space-y-2 text-sm text-slate-600\">\n{perks}\n{pad}  </ul>\n\
             {pad}  <a href={href:?} className={button:?}>\n{pad}    {cta}\n{pad}  </a>\n\
             {pad}</div>\n"
        )
    }

    // ------------------------------------------------------------ everything else

    fn plain(&mut self, el: &Element, depth: usize, ctx: &Ctx) -> String {
        let pad = " ".repeat(depth * 2);
        // Escape hatches, emitted verbatim.
        //
        // `raw` goes where it appears in the tree; `js` is component-body code and is hoisted
        // above the return by `emit`. Neither is checked, reformatted or escaped — that is the
        // deal, and the diagnostic already said so.
        if el.tag == "raw" {
            let target = el.positionals.iter().find_map(|p| match p {
                Positional::Text(t) => Some(t.as_str()),
                _ => None,
            });
            // `raw svelte` in a React build is not an error: a document can carry blocks for
            // several backends, and each emitter takes its own.
            if target.is_some_and(|t| t != "react") {
                return String::new();
            }
            let pad = "  ".repeat(depth);
            return el
                .text_lines
                .iter()
                .map(|line| {
                    format!(
                        "{pad}{line}
"
                    )
                })
                .collect::<Vec<_>>()
                .join("");
        }
        if el.tag == "js" {
            // Collected by `emit`; nothing belongs in the JSX.
            return String::new();
        }

        let mods = modifiers_of(el);
        let class = classes(&el.tag, &mods);
        let (tag_name, fixed) = html_tag(&el.tag, el);

        let Some(tag_name) = tag_name else {
            unsupported(self.diags, el.span, format!("tag `{}`", el.tag));
            return format!("{pad}{{/* TODO(guml): `{}` is not lowered yet */}}\n", el.tag);
        };

        let mut attrs: Vec<String> = fixed;
        let mut class_attr =
            if class.is_empty() { None } else { Some(format!("className={class:?}")) };
        if let Some(a) = el.anchor() {
            attrs.push(format!("id={a:?}"));
        }

        let mut busy_label = None;
        // Layout attributes are presentation, so they join the class list rather
        // than becoming DOM props — `cols={3}` on a <section> is not valid HTML,
        // which `tsc` caught on the landing fixture.
        let mut layout: Vec<String> = Vec::new();
        for a in &el.attrs {
            match a.name.as_str() {
                "aria" => attrs.push(attr_out("aria-label", &a.value, ctx)),
                "busy" => busy_label = a.value.as_text().map(str::to_string),
                // Already folded into `type` by `html_tag`.
                "kind" => {}
                "cols" => {
                    let n = match &a.value {
                        Value::Num(n) => Some(format!("{}", *n as i64)),
                        v => v.as_text().map(str::to_string),
                    };
                    if let Some(n) = n {
                        layout.push(format!("grid gap-6 md:grid-cols-{n}"));
                    }
                }
                "gap" => {
                    if let Value::Num(n) = &a.value {
                        layout.push(format!("gap-{}", *n as i64));
                    }
                }
                "w" => {
                    if let Some(w) = a.value.as_text() {
                        layout.push(format!("max-w-{w}"));
                    }
                }
                "id" | "where" | "cta" | "open" | "sort" | "of" => {}
                // `strike` folds into the class list rather than becoming a prop.
                "strike" => {
                    if let Value::Binding(b) = &a.value {
                        class_attr = Some(format!(
                            "className={{`{class} ${{{} ? \"line-through text-slate-400\" : \"\"}}`}}",
                            expr::lower_expr(&b.expr, ctx)
                        ));
                    }
                }
                _ => attrs.push(attr_out(&a.name, &a.value, ctx)),
            }
        }
        if !layout.is_empty() {
            let joined = layout.join(" ");
            class_attr = Some(match class_attr {
                Some(existing) if existing.starts_with("className=\"") => {
                    let inner = existing.trim_start_matches("className=\"").trim_end_matches('"');
                    format!("className={:?}", format!("{inner} {joined}"))
                }
                Some(other) => other,
                None => format!("className={joined:?}"),
            });
        }
        if let Some(c) = class_attr {
            attrs.insert(0, c);
        }

        // Two-way binding for fields.
        let mut has_change = false;
        if matches!(el.tag.as_str(), "input" | "select") {
            if let Some(name) = el.label() {
                attrs.push(format!("value={{{name}}}"));
                attrs.push(format!("onChange={{(e) => {}(e.target.value)}}", setter(name)));
                has_change = true;
            }
        }
        if el.tag == "check" {
            if let Some(b) = el.binding() {
                attrs.push(format!("checked={{{}}}", expr::lower_in(b, ctx)));
            }
        }

        // A row control with no name of its own takes the row's, matching what the
        // analyser accepted when it let this compile.
        if el.tag == "check"
            && !ctx.item_fields.is_empty()
            && el.attr("aria").is_none()
            && ctx.item_fields.iter().any(|f| f == "title")
        {
            attrs.push("aria-label={item.title}".to_string());
        }

        if let Some(action) = el.actions.first() {
            let handler = match el.tag.as_str() {
                "check" | "toggle" => "onChange",
                "form" => "onSubmit",
                "input" | "select" => "onChange",
                _ => "onClick",
            };
            let js = self.lower_action(action, el, ctx);
            if !js.is_empty() && !(has_change && handler == "onChange") {
                let head = if handler == "onSubmit" {
                    "(e) => { e.preventDefault(); "
                } else {
                    "() => { "
                };
                attrs.push(format!("{handler}={{{head}{js}; }}}}"));
            }
        }

        let attr_str =
            if attrs.is_empty() { String::new() } else { format!(" {}", attrs.join(" ")) };

        if is_void(&el.tag) {
            return format!("{pad}<{tag_name}{attr_str} />\n");
        }

        // Leaf content: prose, a binding, or a label — with `busy` swapping the
        // label while the resource is in flight.
        let text = el
            .content
            .clone()
            .or_else(|| el.binding().map(|b| format!("{{{b}}}")))
            .or_else(|| el.label().map(str::to_string));

        if el.children.is_empty() && el.text_lines.is_empty() {
            let inner = match (&text, &busy_label) {
                (Some(t), Some(busy)) => {
                    let flag = pending_flag(el)
                        .or_else(|| self.pending.clone())
                        .unwrap_or_else(|| "false".to_string());
                    format!("{{{flag} ? {busy:?} : {t:?}}}")
                }
                (Some(t), None) => expr::lower_text_in(t, ctx),
                (None, _) => String::new(),
            };
            return format!("{pad}<{tag_name}{attr_str}>{inner}</{tag_name}>\n");
        }

        // A `busy` label inside this form watches the resource the form submits to:
        // the label belongs to the button, but the mutation is declared on the form.
        let outer_pending = self.pending.clone();
        if el.tag == "form" {
            self.pending = el
                .actions
                .first()
                .and_then(|a| a.split('.').next())
                .filter(|head| self.resource(head).is_some())
                .map(|res| format!("{res}Saving"));
        }

        let mut out = format!("{pad}<{tag_name}{attr_str}>\n");
        if let Some(label) = el.label() {
            let _ =
                writeln!(out, "{pad}  <h3 className=\"font-medium\">{}</h3>", jsx_escape(label));
        }
        if let Some(c) = &el.content {
            let _ = writeln!(
                out,
                "{pad}  <p className=\"mt-2 text-sm text-slate-600\">{}</p>",
                expr::lower_text_in(c, ctx)
            );
        }
        for line in &el.text_lines {
            let _ = writeln!(out, "{pad}  <li>{}</li>", jsx_escape(line));
        }
        let had_children = !el.children.is_empty();
        for child in &el.children {
            let at = out.lines().count() as u32;
            let (text, marks) = self.element_marked(child, depth + 1, ctx);
            out.push_str(&text);
            self.rebase(marks, at);
        }
        if had_children {
            self.mark_here(&out, el.span.line);
        }
        let _ = writeln!(out, "{pad}</{tag_name}>");
        self.pending = outer_pending;
        out
    }

    /// Lower an action body, including resource mutations.
    fn lower_action(&mut self, action: &str, el: &Element, ctx: &Ctx) -> String {
        let mut stmts: Vec<String> = Vec::new();

        for raw in action.split(';') {
            let stmt = raw.trim();
            if stmt.is_empty() {
                continue;
            }

            if let Some(name) = stmt.strip_suffix("++") {
                let name = name.trim();
                stmts.push(format!("{}({name} + 1)", setter(name)));
                continue;
            }
            if let Some(name) = stmt.strip_suffix("--") {
                let name = name.trim();
                stmts.push(format!("{}({name} - 1)", setter(name)));
                continue;
            }

            // `tasks.add{title:draft}` or `tasks.drop`
            if let Some((head, rest)) = stmt.split_once('.')
                && self.resource(head).is_some()
            {
                let (mutation, body) = match rest.split_once('{') {
                    Some((m, b)) => (m.trim(), b.trim_end_matches('}')),
                    None => (rest.trim(), ""),
                };
                let fn_name = format!("{head}{}", capitalize(mutation));
                let in_row = !ctx.item_fields.is_empty();

                let body_js = if body.is_empty() {
                    // A body-less `save` on a row toggles the boolean, which is
                    // what a row checkbox means.
                    if in_row && mutation == "save" && ctx.item_fields.iter().any(|f| f == "done") {
                        "{ done: !item.done }".to_string()
                    } else {
                        "{}".to_string()
                    }
                } else {
                    let pairs = body
                        .split(',')
                        .filter_map(|p| {
                            let (k, v) = p.split_once(':')?;
                            Some(format!("{}: {}", k.trim(), expr::lower_in(v.trim(), ctx)))
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{{ {pairs} }}")
                };

                stmts.push(if in_row {
                    format!("{fn_name}(item, {body_js})")
                } else {
                    format!("{fn_name}({body_js})")
                });
                continue;
            }

            if let Some((lhs, rhs)) = stmt.split_once('=') {
                let lhs = lhs.trim();
                if !lhs.contains('.') && !lhs.contains('(') {
                    stmts.push(format!("{}({})", setter(lhs), expr::lower_in(rhs.trim(), ctx)));
                    continue;
                }
            }

            unsupported(self.diags, el.span, format!("action `{stmt}`"));
        }

        stmts.join("; ")
    }
}

// ---------------------------------------------------------------- resources

/// The desugared resource layer: state, an aborting fetch, and one callback per
/// mutation with optimistic application and rollback on failure.
fn resource_hooks(r: &Resource, wants_pending: bool) -> String {
    let name = &r.name;
    let cap = capitalize(name);
    let item_ty = r.ty.trim_end_matches("[]");
    let ty = if item_ty.is_empty() { "unknown".to_string() } else { item_ty.to_string() };
    let url = &r.url;
    let mut out = String::new();

    let _ = writeln!(out, "  // resource `{name}`");
    let _ = writeln!(out, "  const [{name}, set{cap}] = useState<{ty}[]>([]);");
    let _ = writeln!(out, "  const [{name}Loading, set{cap}Loading] = useState(true);");
    let _ = writeln!(out, "  const [{name}Error, set{cap}Error] = useState<string | null>(null);");
    // A mutation-in-flight flag, distinct from the initial-fetch flag: a `busy`
    // label driven by `{name}Loading` reads "Adding…" during the *page load*, which
    // is the wrong signal at the wrong moment. Only declared when something reads it.
    if wants_pending {
        let _ = writeln!(out, "  const [{name}Saving, set{cap}Saving] = useState(false);");
    }

    // Fetch on mount, with cancellation — the part hand-written effects get wrong.
    let _ = write!(
        out,
        "\n  useEffect(() => {{\n\
         \x20   const controller = new AbortController();\n\
         \x20   set{cap}Loading(true);\n\
         \x20   set{cap}Error(null);\n\
         \x20   fetch({url:?}, {{ signal: controller.signal }})\n\
         \x20     .then((res) => {{\n\
         \x20       if (!res.ok) throw new Error(`Request failed: ${{res.status}}`);\n\
         \x20       return res.json() as Promise<{ty}[]>;\n\
         \x20     }})\n\
         \x20     .then(set{cap})\n\
         \x20     .catch((err: unknown) => {{\n\
         \x20       if (err instanceof Error && err.name === \"AbortError\") return;\n\
         \x20       set{cap}Error(err instanceof Error ? err.message : \"Unknown error\");\n\
         \x20     }})\n\
         \x20     .finally(() => set{cap}Loading(false));\n\
         \x20   return () => controller.abort();\n\
         \x20 }}, []);\n"
    );

    for m in &r.mutations {
        let fname = format!("{name}{}", capitalize(&m.name));
        let body_ty = if m.body.is_empty() {
            "Partial<{ty}>".replace("{ty}", &ty)
        } else {
            format!("Partial<{ty}>")
        };
        let takes_item = m.url.contains('{');
        let args = if takes_item {
            format!("item: {ty}, body: {body_ty} = {{}}")
        } else {
            format!("body: {body_ty}")
        };
        let url_js = if takes_item {
            format!("`{}`", interpolate_path(&m.url))
        } else {
            format!("{:?}", m.url)
        };
        let method = &m.method;

        let _ = write!(out, "\n  const {fname} = useCallback(\n    async ({args}) => {{\n");
        let _ = writeln!(out, "      const snapshot = {name};");
        if wants_pending {
            let _ = writeln!(out, "      set{cap}Saving(true);");
        }

        // Optimistic application, per declared strategy.
        match m.optimistic.as_deref() {
            Some("prepend") => {
                let _ = writeln!(
                    out,
                    "      const optimistic = {{ id: `tmp-${{Date.now()}}`, ...body }} as {ty};\n      set{cap}((prev) => [optimistic, ...prev]);"
                );
            }
            Some("append") => {
                let _ = writeln!(
                    out,
                    "      const optimistic = {{ id: `tmp-${{Date.now()}}`, ...body }} as {ty};\n      set{cap}((prev) => [...prev, optimistic]);"
                );
            }
            Some(_) if method == "DELETE" => {
                let _ =
                    writeln!(out, "      set{cap}((prev) => prev.filter((it) => it !== item));");
            }
            Some(_) => {
                let _ = writeln!(
                    out,
                    "      set{cap}((prev) => prev.map((it) => (it === item ? {{ ...it, ...body }} : it)));"
                );
            }
            None => {}
        }

        let _ = write!(
            out,
            "      try {{\n\
             \x20       const res = await fetch({url_js}, {{\n\
             \x20         method: {method:?},\n\
             \x20         headers: {{ \"Content-Type\": \"application/json\" }},\n\
             \x20         body: JSON.stringify(body),\n\
             \x20       }});\n\
             \x20       if (!res.ok) throw new Error(`Request failed: ${{res.status}}`);\n"
        );

        // A create needs the server's row so the temporary id is replaced.
        if matches!(m.optimistic.as_deref(), Some("prepend") | Some("append")) {
            let _ = writeln!(
                out,
                "        const created = (await res.json()) as {ty};\n        set{cap}((prev) => prev.map((it) => (it === optimistic ? created : it)));"
            );
        }

        let _ = write!(
            out,
            "      }} catch (err: unknown) {{\n\
             \x20       set{cap}(snapshot);\n\
             \x20       set{cap}Error(err instanceof Error ? err.message : \"Could not save\");\n\
             \x20     }}"
        );
        // `finally`, not a line after the catch: the flag has to clear on the failure
        // path too, or one failed mutation leaves the button saying "Adding…" forever.
        if wants_pending {
            let _ = write!(out, " finally {{\n        set{cap}Saving(false);\n      }}");
        }
        let _ = write!(out, "\n    }},\n    [{name}],\n  );\n");
    }

    out
}

/// `/api/tasks/{id}` → `/api/tasks/${item.id}` inside a template literal.
fn interpolate_path(url: &str) -> String {
    let mut out = String::new();
    let mut rest = url;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let _ = write!(out, "${{item.{}}}", &after[..close]);
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

/// Every `js` block in the tree, in document order.
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

/// Resources whose mutations something actually watches with a `busy` label.
///
/// Walked up front so the pending flag is declared only where it is read: an unused
/// `const [tasksSaving, …]` is dead weight in every generated file that never asks
/// for a busy state.
fn busy_resources(els: &[Element], enclosing: Option<&str>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for el in els {
        let owner = el
            .actions
            .first()
            .and_then(|a| a.split('.').next())
            .filter(|h| !h.contains('=') && !h.contains('+') && !h.contains('-'))
            .or(enclosing);
        if el.attr("busy").is_some() {
            if let Some(res) = owner {
                found.insert(res.to_string());
            }
        }
        found.extend(busy_resources(&el.children, owner));
    }
    found
}

/// Which loading flag a `busy` label should watch, if the element itself carries
/// the mutation. `None` means "ask the enclosing form".
fn pending_flag(el: &Element) -> Option<String> {
    el.actions
        .first()
        .and_then(|a| a.split('.').next())
        .filter(|head| !head.contains('=') && !head.contains('+') && !head.contains('-'))
        .map(|res| format!("{res}Saving"))
}

// ---------------------------------------------------------------- helpers

fn attr_out(name: &str, v: &Value, ctx: &Ctx) -> String {
    match v {
        Value::Str(s) => format!("{name}={}", expr::lower_attr_value_in(s, ctx)),
        Value::Word(w) => format!("{name}={:?}", w),
        Value::Num(_) | Value::Bool(_) => format!("{name}={{{}}}", v.to_js()),
        Value::Binding(b) => format!("{name}={{{}}}", expr::lower_expr(&b.expr, ctx)),
        Value::Flag => name.to_string(),
    }
}

fn initial(v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{s:?}"),
        Value::Num(_) | Value::Bool(_) => v.to_js(),
        Value::Word(w) => format!("{w:?}"),
        Value::Binding(b) => b.source.clone(),
        Value::Flag => "true".into(),
    }
}

/// An enumerated state gets a union type, so an invalid value is a type error.
fn state_type(init: &Value, domain: &[String]) -> String {
    if domain.len() > 1 {
        let union = domain.iter().map(|d| format!("{d:?}")).collect::<Vec<_>>().join(" | ");
        return format!("<{union}>");
    }
    match init {
        Value::Bool(_) => "<boolean>".into(),
        _ => String::new(),
    }
}

fn ts_type(guml: &str) -> &str {
    match guml {
        "bool" => "boolean",
        "int" | "number" | "float" => "number",
        _ => "string",
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => format!("{}{}", f.to_uppercase(), c.as_str()),
        None => String::new(),
    }
}

/// JSX text is mostly literal, but braces would open an expression.
fn jsx_escape(s: &str) -> String {
    s.replace('{', "&#123;").replace('}', "&#125;")
}

/// GUML tag -> HTML element, plus fixed attributes. `None` means "not lowered".
pub(crate) fn html_tag(tag: &str, el: &Element) -> (Option<&'static str>, Vec<String>) {
    match tag {
        "card" | "row" | "col" | "metric" => (Some("div"), vec![]),
        "section" => (Some("section"), vec![]),
        "nav" => (Some("nav"), vec![]),
        "hero" => (Some("header"), vec![]),
        "footer" => (Some("footer"), vec![]),
        "form" => (Some("form"), vec![]),
        "h" | "h2" => (Some("h2"), vec![]),
        "h1" => (Some("h1"), vec![]),
        "h3" => (Some("h3"), vec![]),
        "p" | "head" | "empty" => (Some("p"), vec![]),
        "text" => (Some("span"), vec![]),
        // Inside a form the primary button submits; elsewhere it must not.
        "btn" => (
            Some("button"),
            vec![if el.has_modifier("primary") && el.actions.is_empty() {
                "type=\"submit\"".to_string()
            } else {
                "type=\"button\"".to_string()
            }],
        ),
        "link" => {
            let href = el
                .route()
                .map(str::to_string)
                .or_else(|| el.anchor().map(|a| format!("#{a}")))
                .unwrap_or_else(|| "#".to_string());
            (Some("a"), vec![format!("href={href:?}")])
        }
        "check" => (Some("input"), vec!["type=\"checkbox\"".to_string()]),
        "toggle" => {
            (Some("input"), vec!["type=\"checkbox\"".to_string(), "role=\"switch\"".to_string()])
        }
        // `type` comes from the element's `kind` attribute when it has one; the caller
        // replaces this default. Emitting both produced `<input type="text" kind="email">`,
        // which `tsc` rejects — `kind` is not a DOM property.
        "input" => (
            Some("input"),
            vec![format!("type={:?}", el.attr("kind").and_then(|v| v.as_text()).unwrap_or("text"))],
        ),
        "select" => (Some("select"), vec![]),
        _ => (None, vec![]),
    }
}

pub(crate) fn is_void(tag: &str) -> bool {
    matches!(tag, "input" | "check" | "toggle")
}

/// The design system. Every string here is a token the model does not produce,
/// and a presentational decision it cannot get wrong.
/// Classes for a tag and its modifiers, from the active theme.
///
/// This was a `match` statement with the Tailwind palette written into it, which meant "the compiler
/// owns presentation" also meant nobody else could. The table now lives in `crate::theme` as data;
/// see that module for why a themeable compiler still has to enforce an accessibility contract.
///
/// The signature is unchanged so every call site reads as before. A per-compilation theme belongs on
/// `Options`; until a backend threads one through, this is the shipped theme.
pub(crate) fn classes(tag: &str, mods: &[&str]) -> String {
    crate::theme::active().classes(tag, mods)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests that need to parse source live in `crates/guml-compiler/tests/desugar.rs`:
    // this crate must not depend on `guml-parser` (that would be a cycle through the
    // driver), so unit tests here cover the pure helpers only.

    #[test]
    fn classes_are_semantic_not_positional() {
        assert!(classes("btn", &["primary"]).contains("bg-slate-900"));
        assert!(classes("btn", &["quiet"]).contains("text-slate-500"));
        assert!(classes("btn", &[]).contains("border-slate-300"));
        assert!(classes("card", &["sm", "center"]).contains("max-w-sm"));
    }

    #[test]
    fn paths_interpolate_into_template_literals() {
        assert_eq!(interpolate_path("/api/tasks/{id}"), "/api/tasks/${item.id}");
        assert_eq!(interpolate_path("/api/tasks"), "/api/tasks");
    }

    #[test]
    fn enumerated_state_gets_a_union_type() {
        let domain = vec!["all".to_string(), "open".to_string()];
        assert_eq!(state_type(&Value::Word("all".into()), &domain), "<\"all\" | \"open\">");
        assert_eq!(state_type(&Value::Num(0.0), &[]), "");
    }

    #[test]
    fn braces_in_prose_cannot_open_a_jsx_expression() {
        assert_eq!(jsx_escape("a {b} c"), "a &#123;b&#125; c");
    }
}
