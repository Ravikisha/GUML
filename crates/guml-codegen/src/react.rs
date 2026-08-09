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
            cse: Vec::new(),
            collections: program.resources.iter().map(|r| r.name.clone()).collect(),
            row_bool: crate::row_bool_fields(program),
            pending: None,
        };
        // Common-subexpression elimination, planned before rendering so every `Ctx` created during it
        // already carries the substitutions.
        g.plan_cse();
        let (body, body_offsets) = g.tree();
        // Lowered while `g` still holds the diagnostics borrow, and emitted further down in the
        // component body. An effect uses the same action lowering a button does.
        let effects = g.effect_hooks();
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
        // An error boundary is a class component, so it needs `Component` — and `ReactNode` for its
        // children prop. Only for a document that uses an escape hatch: see `crate::ERROR_BOUNDARY_TSX`
        // for why wrapping every page would be ceremony rather than safety.
        let needs_boundary = crate::uses_escape_hatch(&program.tree);
        if needs_boundary {
            imports.push("Component");
        }
        if !imports.is_empty() {
            let _ = writeln!(src, "import {{ {} }} from \"react\";\n", imports.join(", "));
        }
        if needs_boundary {
            // `import type` rather than a value import: `ReactNode` is only a type, and a value import of
            // it fails under `verbatimModuleSyntax` and `isolatedModules`.
            let _ = writeln!(src, "import type {{ ReactNode }} from \"react\";\n");
        }

        // Imports for host components a registry package contributed. Grouped per module and sorted, so
        // the emitted header is stable across builds — a diff that reorders imports on every run makes
        // every other change unreadable.
        //
        // Emitted only for tags the document actually uses: a design system with 300 components must not
        // put 300 imports at the top of a page that uses two.
        let mut host: BTreeSet<(String, String)> = BTreeSet::new();
        collect_host_components(&program.tree, &mut host);
        let mut by_module: std::collections::BTreeMap<String, BTreeSet<String>> =
            std::collections::BTreeMap::new();
        for (module, name) in host {
            by_module.entry(module).or_default().insert(name);
        }
        for (module, names) in &by_module {
            let list: Vec<&str> = names.iter().map(String::as_str).collect();
            let _ = writeln!(src, "import {{ {} }} from {module:?};\n", list.join(", "));
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

        // Retry with backoff, emitted once per file and only when something fetches.
        if !resources.is_empty() {
            src.push_str(crate::RETRY_TS);
            // The cache builds on `retrying`, so it has to come after it. Together they are ~150 lines of
            // output, once per file, and zero tokens of *input* — which is the whole trade
            // convention-as-compression makes.
            src.push_str(crate::CACHE_TS);
        }

        // Before the component, so it is defined by the time the component's JSX references it.
        if needs_boundary {
            src.push_str(crate::ERROR_BOUNDARY_TSX);
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

        // `js` blocks come *before* the derived values, and the order is load-bearing.
        //
        // A repeater may now iterate a `js`-computed array (`list matches of=Event`), and its `visible…`
        // memo reads that array. With the blocks emitted after, the output was
        // `const visibleMatches = matches;` followed by `const matches = …` — a temporal dead zone error
        // that throws on first render. `js` still lands after the state and the resource hooks, which is
        // what it needs to reference; only the two later groups moved below it.
        //
        // Emitted verbatim: not checked, not reformatted, not escaped. That is the deal, and `GUML0090`
        // already said so.
        for block in js_blocks(&program.tree) {
            src.push('\n');
            for line in block {
                let _ = writeln!(src, "  {line}");
            }
        }

        for derived in &hooks.derived {
            src.push('\n');
            src.push_str(derived);
        }

        // Declared effects, after the resource callbacks they invoke and after the derived values a
        // trigger may read.
        for (source_line, hook) in &effects {
            src.push('\n');
            map.mark(line_of(&src), *source_line);
            src.push_str(hook);
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
        } else if needs_boundary {
            // The boundary *is* the single root, so a multi-root page needs no fragment as well.
            src.push_str("    <GumlBoundary>\n");
            for line in body.lines() {
                let _ = writeln!(src, "  {line}");
            }
            src.push_str("    </GumlBoundary>\n");
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

/// Every `(module, component)` pair the tree needs, walked recursively.
///
/// Only tags actually present in the document: a 300-component design system must not put 300 imports at
/// the top of a page that uses two. A `BTreeSet` both deduplicates and orders, so two uses of `callout`
/// produce one import and the header does not reshuffle between builds.
fn collect_host_components(els: &[Element], out: &mut BTreeSet<(String, String)>) {
    for el in els {
        if let Some((element, Some(module))) = crate::custom_element(&el.tag)
            && element.starts_with(char::is_uppercase)
        {
            out.insert((module.to_string(), element.to_string()));
        }
        collect_host_components(&el.children, out);
    }
}

/// `tasks.open.count` → `tasksOpenCount`. Deterministic, so the emitted name does not move when
/// unrelated parts of the document change — a memo variable that renames itself between builds makes
/// every diff unreadable.
fn memo_name(source: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in source.chars() {
        if ch.is_alphanumeric() {
            if upper {
                out.extend(ch.to_uppercase());
                upper = false;
            } else {
                out.push(ch);
            }
        } else {
            // Any separator — `.`, `(`, a space — starts a new word.
            upper = !out.is_empty();
        }
    }
    if out.is_empty() { "memo".to_string() } else { out }
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
    /// Lowered JavaScript → memo variable, for expressions used more than once. See `plan_cse`.
    cse: Vec<(String, String)>,
    diags: &'a mut Diagnostics,
    hooks: Hooks,
    /// Resource names, built once. Rebuilding this per element cost 5 ms on a 200-line
    /// document — the benchmark caught it immediately, which is the argument for having one.
    collections: Vec<String>,
    /// See `crate::row_bool_fields`.
    row_bool: Vec<(String, String)>,
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
        let ctx = Ctx::default()
            .with_collections(&self.collections)
            .with_row_bool(&self.row_bool)
            .with_cse(&self.cse);
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

    /// Find expressions worth computing once, and record how to substitute them.
    ///
    /// Only *aggregates* qualify. `{count}` used ten times lowers to `count` ten times, which costs
    /// nothing; `{tasks.open.count}` lowers to an O(n) `filter().length`, so three uses is three scans
    /// of the list per render for a single number.
    ///
    /// Row-scoped expressions are excluded: inside a repeater the value depends on `item`, so hoisting
    /// it out of the map would be wrong rather than merely unhelpful.
    fn plan_cse(&mut self) {
        let ctx = Ctx::default()
            .with_collections(&self.collections)
            .with_row_bool(&self.row_bool)
            .with_cse(&self.cse);
        let mut counts: Vec<(String, String, usize)> = Vec::new();

        fn walk(
            els: &[Element],
            ctx: &Ctx,
            counts: &mut Vec<(String, String, usize)>,
            in_repeater: bool,
        ) {
            for el in els {
                // A repeater's children are a row template; their expressions are per-row.
                let row = in_repeater || matches!(el.tag.as_str(), "list" | "table");
                if !row && !el.is_escape() {
                    let mut sources: Vec<String> = Vec::new();
                    for p in &el.positionals {
                        if let Positional::Binding(b) = p {
                            sources.push(b.source.clone());
                        }
                    }
                    for a in &el.attrs {
                        if let Value::Binding(b) = &a.value {
                            sources.push(b.source.clone());
                        }
                    }
                    for text in el.content.iter().chain(el.text_lines.iter()) {
                        for inner in guml_syntax::expr::interpolations(text) {
                            sources.push(inner.to_string());
                        }
                    }

                    for source in sources {
                        let lowered = expr::lower_in(&source, ctx);
                        // The marker of an aggregate: a scan of a collection.
                        if !lowered.contains(".filter(") && !lowered.contains(".reduce(") {
                            continue;
                        }
                        match counts.iter_mut().find(|(l, _, _)| *l == lowered) {
                            Some((_, _, n)) => *n += 1,
                            None => counts.push((lowered, source, 1)),
                        }
                    }
                }
                walk(&el.children, ctx, counts, row);
            }
        }
        walk(&self.program.tree, &ctx, &mut counts, false);

        for (lowered, source, n) in counts {
            if n < 2 {
                continue;
            }
            let name = memo_name(&source);
            // `useMemo` over the collections the expression reads, which is the same dependency rule
            // the `where=` memo already uses.
            let deps: Vec<&str> = self
                .collections
                .iter()
                .map(String::as_str)
                .filter(|c| lowered.contains(*c))
                .collect();
            self.hooks.needs_memo = true;
            self.hooks.derived.push(format!(
                "  // `{source}` is used {n} times; computed once.
  const {name} = useMemo(() => {lowered}, [{}]);
",
                deps.join(", ")
            ));
            self.cse.push((lowered, name));
        }
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
        let rendered = match el.tag.as_str() {
            "list" | "table" => self.repeater(el, depth),
            "tabs" => self.tabs(el, depth),
            "faq" => self.faq(el, depth),
            "tier" => self.tier(el, depth),
            "stat" => self.stat(el, depth, ctx),
            _ => self.plain(el, depth, ctx),
        };
        self.conditional(el, rendered, depth, ctx)
    }

    /// `if={expr}` → `{expr && ( … )}`.
    ///
    /// # This was a silent mis-lowering
    ///
    /// `if` has been in `GLOBAL_ATTRS` since the first release and was lowered by *nothing*. It fell
    /// through the attribute loop to the generic arm and came out as a DOM property: `card X if={open}`
    /// emitted `<div if={open}>`, which React forwards to the DOM as an unknown attribute and `tsc`
    /// rejects outright. The document said "show this when open" and the output showed it always.
    ///
    /// Nothing caught it because no fixture used `if=`, so neither the snapshot tests nor
    /// `typecheck-emitted` ever saw one — exactly the shape of gap invariant 3 exists to close, and the
    /// reason `modal`/`drawer`/`toast` could not have been added on top of it.
    ///
    /// Applied in the dispatcher rather than in `plain`, so it covers a repeater and a `tabs` too: the
    /// attribute is global, and a conditional list is at least as common as a conditional card.
    fn conditional(&mut self, el: &Element, rendered: String, depth: usize, ctx: &Ctx) -> String {
        let Some(cond) = el.attr("if") else { return rendered };
        if rendered.trim().is_empty() {
            return rendered;
        }
        let pad = " ".repeat(depth * 2);
        let test = match cond {
            Value::Binding(b) => expr::lower_expr(&b.expr, ctx),
            // A literal condition is legal but pointless, and a *false* one would delete the subtree
            // with no diagnostic. Report it rather than quietly emit dead JSX.
            other => {
                unsupported(
                    self.diags,
                    el.span,
                    format!(
                        "`if=` with the literal `{}` — a condition that cannot change is not a condition; use a binding",
                        other.to_js()
                    ),
                );
                return rendered;
            }
        };
        // Two spaces of extra indent on the body, so the emitted JSX still reads as a tree.
        let body: String =
            rendered.lines().map(|l| format!("  {l}\n")).collect::<Vec<_>>().join("");
        format!("{pad}{{{test} && (\n{body}{pad})}}\n")
    }

    /// `stat "Revenue" {total} delta="+12%"` — a KPI tile.
    ///
    /// Custom rather than a container with children because the whole point is that the three parts are
    /// *positional*: a dashboard has twenty of these, and `stat "Open" {tasks.open.count}` is one line
    /// where the nested form is four. The label is rendered before the value in the DOM and reordered
    /// visually, so a screen reader reads "Open, 12" rather than "12, Open".
    fn stat(&mut self, el: &Element, depth: usize, ctx: &Ctx) -> String {
        let pad = " ".repeat(depth * 2);
        let mut parts = el.positionals.iter().filter_map(|p| match p {
            Positional::Text(t) => Some(expr::lower_text_in(t, ctx)),
            Positional::Binding(b) => Some(format!("{{{}}}", expr::lower_expr(&b.expr, ctx))),
            _ => None,
        });
        let label = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();
        let delta = el.attr("delta").map(|v| match v {
            Value::Binding(b) => format!("{{{}}}", expr::lower_expr(&b.expr, ctx)),
            other => other.as_text().map(jsx_escape).unwrap_or_default(),
        });

        let mods = modifiers_of(el);
        // `<dl>`, not `<div>`: `<dt>`/`<dd>` are only valid inside a description list, and a mismatched
        // open/close pair (`<div>…</dl>`) is not parseable JSX at all.
        let mut out = format!("{pad}<dl className={:?}>\n", classes("stat", &mods));
        let _ =
            writeln!(out, "{pad}  <dt className={:?}>{label}</dt>", classes("stat-label", &mods));
        let _ =
            writeln!(out, "{pad}  <dd className={:?}>{value}</dd>", classes("stat-value", &mods));
        if let Some(delta) = delta {
            let _ = writeln!(
                out,
                "{pad}  <dd className={:?}>{delta}</dd>",
                classes("stat-delta", &mods)
            );
        }
        let _ = writeln!(out, "{pad}</dl>");
        out
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
        // A derived source — a `js` block's array, named with `of=Type` — is legitimate and gets no fetch,
        // no loading state and no error state, because there is no request. Those belong to `data`.
        //
        // This used to be a hard bail, and that is what made more than one client-side filter inexpressible:
        // `where=` takes a single enumerated state, a predicate over three states can only live in `js`, and
        // the resulting array could not be iterated. Two GUML-Bench reference answers filter on the server
        // and fail their own "one fetch, not one per change" criterion for exactly this reason.
        let rows = self.program.repeater_rows(el);
        let fetched = rows.as_ref().is_some_and(|r| r.from_resource);
        if rows.is_none() {
            // `GUML0104` already said so at compile time, with the fix in the message.
            unsupported(
                self.diags,
                el.span,
                format!(
                    "`{}` over `{source}`, which is neither a resource nor given an `of=` row type",
                    el.tag
                ),
            );
            return String::new();
        }

        let fields =
            if fetched { self.item_fields(&source) } else { self.program.repeater_fields(el) };
        let ctx =
            Ctx::item(&fields).with_collections(&self.collections).with_row_bool(&self.row_bool);
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
            // Both cases live in `crate::where_filter`, shared with the Svelte backend so the two
            // cannot filter the same document differently.
            let flag = self.row_bool.iter().find(|(c, _)| *c == source).map(|(_, f)| f.as_str());
            let text_fields = crate::search_fields(self.program, &source, &filter);
            if let Some(body) =
                crate::where_filter(&source, &filter, &domain, &fields, &text_fields, flag)
            {
                self.hooks.needs_memo = true;
                self.hooks.derived.push(format!(
                    "  const {visible} = useMemo(
    () =>
      {body},
    [{source}, {filter}],
  );
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

        // Per child as well as concatenated. The list form wants one string; the table form wants each
        // child separately, because a cell wraps a *child* and a child can render as many lines — a `modal`
        // in a row is `{cond && (` … `)}` across six of them. Wrapping per line tore those apart and emitted
        // JSX that did not parse: 42 syntax errors, caught by `typecheck-emitted` and by nothing else.
        let (cells, cell_marks) = self.collect(|g| {
            let mut cells: Vec<String> = Vec::new();
            let mut seen = 0u32;
            for child in &template {
                let (text, marks) = g.element_marked(child, depth + 5, &ctx);
                g.rebase(marks, seen);
                seen += text.lines().count() as u32;
                cells.push(text);
            }
            cells
        });
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

        // The error banner and the loading skeleton belong to a *fetched* source only. A `js`-computed array
        // has no `matchesError` and no `matchesLoading`, so emitting them would reference names that do not
        // exist and the output would not compile — the silent mis-lowering invariant 3 forbids. A derived
        // array still gets its empty state, because "no rows matched" is a real thing to say about one.
        if fetched {
            // Error first: a failed request is the most important thing on screen.
            let _ = writeln!(
                out,
                "{pad}{{{source}Error && (\n{pad}  <p role=\"alert\" className=\"mt-4 rounded-md bg-red-50 px-3 py-2 text-sm text-red-700\">\n{pad}    {{{source}Error}}\n{pad}  </p>\n{pad})}}"
            );
        }

        // The loading branch only exists when something is loading. A derived array opens straight into the
        // empty check, so the emitted ternary is one arm shorter rather than carrying a dead `false ?`.
        if fetched {
            let _ = writeln!(out, "{pad}{{{source}Loading ? (");
        }
        // The skeleton mirrors the shape it stands in for. A `<ul>` skeleton followed by a `<table>` is
        // valid HTML and a visible layout shift the moment the data lands — which is the thing a skeleton
        // exists to prevent.
        if crate::is_tabular(el) {
            let span = crate::column_headers(el).len().max(1);
            let _ =
                writeln!(out, "{pad}  <table className={:?}>", classes("table", &modifiers_of(el)));
            // The headers are static, so they are rendered *while loading* too. That removes the layout
            // shift entirely rather than shrinking it, and it is what the wc backend already does by keeping
            // its `<thead>` outside the `<tbody>` it rewrites.
            if let Some(head) = crate::table_head(el, &pad, "className") {
                out.push_str(&head);
            }
            let _ = writeln!(
                out,
                "{pad}    <tbody>\n{pad}      {{[0, 1, 2].map((n) => (\n{pad}        <tr key={{n}}>\n{pad}          <td colSpan={{{span}}} className=\"h-12 animate-pulse rounded-md bg-slate-100\" />\n{pad}        </tr>\n{pad}      ))}}\n{pad}    </tbody>\n{pad}  </table>"
            );
        } else {
            let _ = writeln!(
                out,
                "{pad}  <ul className=\"mt-6 space-y-2\">\n{pad}    {{[0, 1, 2].map((n) => (\n{pad}      <li key={{n}} className=\"h-12 animate-pulse rounded-md bg-slate-100\" />\n{pad}    ))}}\n{pad}  </ul>"
            );
        }
        let _ =
            writeln!(out, "{pad}{} {visible}.length === 0 ? (", if fetched { ") :" } else { "{" });
        match &empty {
            Some(e) => {
                let ctx = Ctx::default()
                    .with_collections(&self.collections)
                    .with_row_bool(&self.row_bool)
                    .with_cse(&self.cse);
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

        // `table` lowers to a real table; `list` to a list. Until this existed both emitted `<ul>`, so a
        // document that asked for tabular data got rows with no columns, no headers, and no header
        // association for a screen reader — and `render-emitted.mjs`'s "table without header cells"
        // assertion had never fired, because no `<table>` was ever emitted for it to check.
        if crate::is_tabular(el) {
            crate::check_columns(self.diags, "react", el, template.len());
            let _ =
                writeln!(out, "{pad}  <table className={:?}>", classes("table", &modifiers_of(el)));
            if let Some(head) = crate::table_head(el, &pad, "className") {
                out.push_str(&head);
            }
            let _ = writeln!(out, "{pad}    <tbody>");
            let _ = writeln!(out, "{pad}      {{{visible}.map((item, i) => (");
            let _ = writeln!(out, "{pad}        <tr key={{{key}}}>");
            // One `<td>` per *child* of the row template, which is why `check_columns` compares the header
            // count against exactly that number.
            let rows_at = out.lines().count() as u32;
            for cell in &cells {
                let _ = writeln!(out, "{pad}          <td{}>", class_attr_of("td"));
                out.push_str(cell);
                let _ = writeln!(out, "{pad}          </td>");
            }
            self.rebase(cell_marks, rows_at);
            self.mark_here(&out, el.span.line);
            let _ = writeln!(out, "{pad}        </tr>");
            let _ = writeln!(out, "{pad}      ))}}");
            let _ = writeln!(out, "{pad}    </tbody>");
            let _ = writeln!(out, "{pad}  </table>");
            let _ = writeln!(out, "{pad})}}");
            return out;
        }

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
        // A `tier`'s call to action is an `<a href>` built from its `cta` and its route, so an action on
        // one has nowhere to go. It used to be dropped in silence: `tier Team … >subscription.setPlan`
        // emitted a plain link and the plan never changed, with exit code 0. Report it — invariant 3 — and
        // the author gets a `card` with a `btn` in it, which works today.
        if !el.actions.is_empty() {
            crate::unsupported_in(
                self.diags,
                "react",
                el.span,
                "an action on a `tier`: its call to action is a link built from `cta` and the route. Put a `btn` in a `card` instead",
            );
        }
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

        // A **host component** takes its declared attributes as props, verbatim.
        //
        // The match below encodes what each attribute means *for a builtin*, and applying that to a
        // component the compiler knows nothing about is wrong in both directions. It silently dropped
        // props: `chart … of=revenue kind=line` emitted `<Chart rows={points} label="month">` — `of` is in
        // the "consumed elsewhere" list because a *repeater* uses it, and `kind` is folded into `type` for
        // an `<input>`. Two declared attributes gone, no diagnostic, and a chart plotting nothing.
        //
        // The registry entry is the authority here: it lists what the component accepts, and the compiler's
        // job is to pass those through rather than to reinterpret them. `aria` still becomes `aria-label`,
        // because that mapping is the accessibility contract the entry declares and not a DOM detail.
        if let Some(def) = crate::registry().get(&el.tag).filter(|d| d.is_host_component()) {
            for a in &el.attrs {
                if a.name == "aria" {
                    attrs.push(attr_out("aria-label", &a.value, ctx));
                } else if COMPILER_OWNED.contains(&a.name.as_str()) {
                    // Global attributes the *compiler* acts on, never props. `if` is the conditional and is
                    // applied by `conditional()`; forwarding it as well emitted
                    // `<CommandMenu if={palette} …>` inside `{palette && (…)}` — the guard applied twice,
                    // once as an unknown prop on someone else's component. An entry that lists one of these
                    // is mistaken about what it owns, and passing it through would honour the mistake.
                } else if def.attrs.contains(&a.name) {
                    attrs.push(attr_out(&a.name, &a.value, ctx));
                }
                // An attribute the entry does not declare is already `GUML0032` from the parser; emitting
                // it anyway would put an unknown prop on someone else's component.
            }
            // `requires_label` on a container means its accessible name comes from the title positional —
            // which is exactly what the package audit warns about being easy to omit. So it *is* the name,
            // rather than children the component may or may not render.
            if def.a11y.requires_label && el.attr("aria").is_none() {
                if let Some(label) = el.label() {
                    attrs.push(format!("aria-label={label:?}"));
                }
            }
            return self.host_component(el, tag_name, attrs, class_attr, depth, ctx);
        }

        for a in &el.attrs {
            match a.name.as_str() {
                "aria" => attrs.push(attr_out("aria-label", &a.value, ctx)),
                "busy" => busy_label = a.value.as_text().map(str::to_string),
                // Already folded into `type` by `html_tag`.
                "kind" => {}
                // A *number* is a grid column count. A string is a repeater's header list, handled by
                // `repeater` — and it must not reach here, because `md:grid-cols-Client, Amount` is not a
                // class and the emitted markup would carry it silently.
                "cols" => {
                    if let Value::Num(n) = &a.value {
                        layout.push(format!("grid gap-6 md:grid-cols-{}", *n as i64));
                    } else if !crate::is_tabular(el) {
                        // A bare word count (`cols=three`) is already `GUML0081`; anything else on a
                        // non-repeater is a mistake this backend should not guess at.
                        if let Some(text) = a.value.as_text() {
                            layout.push(format!("grid gap-6 md:grid-cols-{text}"));
                        }
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
                // Consumed elsewhere, so emitting them here would duplicate or invent a DOM property.
                // `if` is `conditional`, `delta` is `stat`, `src`/`alt` are folded into `html_tag`'s
                // fixed attributes, and `placeholder` on a `select` becomes a leading disabled option
                // rather than an attribute the element does not have.
                "id" | "where" | "cta" | "open" | "sort" | "of" | "if" | "delta" | "src"
                | "alt" => {}
                "placeholder" if el.tag == "select" => {}
                // `step done` / `step current`: a stage's status is a *rendering* decision plus an
                // announcement, never a DOM attribute. `aria-current="step"` is what lets a screen
                // reader say which stage the reader is on, and it is the one ARIA value defined for
                // exactly this.
                "current" => {
                    if !matches!(&a.value, Value::Bool(false)) {
                        attrs.push("aria-current=\"step\"".to_string());
                    }
                }
                "done" => {
                    let flag = match &a.value {
                        Value::Binding(b) => expr::lower_expr(&b.expr, ctx),
                        Value::Bool(false) => continue,
                        _ => "true".to_string(),
                    };
                    class_attr = Some(format!(
                        "className={{`{class} ${{{flag} ? \"text-slate-900\" : \"\"}}`}}"
                    ));
                }
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
                    // Deduplicated: `cols=` contributes `grid gap-6` so that `card cols=3` becomes a
                    // grid, and the `grid` tag's own theme rule says the same thing, so `grid cols=3`
                    // emitted every utility twice.
                    format!("className={:?}", crate::dedupe_classes(&format!("{inner} {joined}")))
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
                // An enumerated state has a *union* type, and `e.target.value` is `string`, so the
                // setter call needs a cast: `setSeverity(e.target.value)` is `TS2345` under `--strict`.
                //
                // This had never been emitted, because a `select` produced no options at all until the
                // options bug was fixed, and no fixture bound one to an enumerated state. The cast is
                // sound rather than a silencing `as any`: every `<option>` on this element is generated
                // from the same domain the union comes from, so the only values the event can carry are
                // members of it.
                let domain = self
                    .program
                    .states
                    .iter()
                    .find(|s| s.name == name)
                    .map(|s| s.domain.clone())
                    .unwrap_or_default();
                // A numeric state gets `Number(…)`. `e.target.value` is a `string` whatever the input's
                // `type` is, so `input qty kind=number` bound to `state qty=1` emitted
                // `setQty(e.target.value)` — `TS2345` under `--strict`, and at runtime a state that starts
                // as a number and becomes a string on the first keystroke, so `qty * price` concatenates.
                let numeric = self
                    .program
                    .states
                    .iter()
                    .any(|s| s.name == name && matches!(s.init, Value::Num(_)));
                let value = if el.tag == "select" && domain.len() > 1 {
                    let union =
                        domain.iter().map(|d| format!("{d:?}")).collect::<Vec<_>>().join(" | ");
                    format!("e.target.value as {union}")
                } else if numeric {
                    "Number(e.target.value)".to_string()
                } else {
                    "e.target.value".to_string()
                };
                attrs.push(format!("onChange={{(e) => {}({value})}}", setter(name)));
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
            let js = self.lower_action(action, el.span, ctx);
            if !js.is_empty() && !(has_change && handler == "onChange") {
                let head = if handler == "onSubmit" {
                    "(e) => { e.preventDefault(); "
                } else {
                    "() => { "
                };
                attrs.push(format!("{handler}={{{head}{js}; }}}}"));
            }
        }

        // A field the author left unnamed is named from the state it binds. `sema::check_label` warns
        // when this fires, and that warning is only honest because the name is emitted here — see
        // `crate::derived_aria_label`. Shared rather than per-backend for the usual reason: this is
        // the third accessible-name rule, and the first two had already drifted.
        if let Some(name) = crate::derived_aria_label(el) {
            attrs.push(format!("aria-label={name:?}"));
        }

        let attr_str =
            if attrs.is_empty() { String::new() } else { format!(" {}", attrs.join(" ")) };

        if is_void(&el.tag) {
            return format!("{pad}<{tag_name}{attr_str} />\n");
        }

        // A `select` renders its choices rather than its children generically: an authored `option`
        // child and a state domain are the same thing seen from two sides, and `crate::select_options`
        // is the one place that reconciles them. Before this, a `select` emitted no options at all —
        // and its bound state name leaked out as the element's *text*.
        if el.tag == "select" {
            let options = crate::select_options(self.program, el);
            if options.is_empty() {
                unsupported(
                    self.diags,
                    el.span,
                    "a `select` with no `option` children and no enumerated state domain — there is nothing to choose from",
                );
            }
            let mut out = format!("{pad}<{tag_name}{attr_str}>\n");
            // A placeholder is a disabled, empty-valued first option, which is the only spelling that
            // both shows the hint and cannot be submitted. `<select placeholder>` is not a thing.
            if let Some(hint) = el.attr("placeholder").and_then(|v| v.as_text()) {
                let _ = writeln!(
                    out,
                    "{pad}  <option value=\"\" disabled>{}</option>",
                    jsx_escape(hint)
                );
            }
            for opt in &options {
                let _ = writeln!(out, "{pad}  <option value={opt:?}>{}</option>", jsx_escape(opt));
            }
            let _ = writeln!(out, "{pad}</{tag_name}>");
            return out;
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

    /// A loaded package's own component: its declared props, a binding if it is a field, and its children.
    ///
    /// Separate from `plain` because almost nothing `plain` does applies. There is no theme class table for
    /// someone else's component, no element-specific fixed attributes, and no reinterpreting of attribute
    /// names — the registry entry says what it accepts and the compiler passes that through.
    fn host_component(
        &mut self,
        el: &Element,
        tag_name: &str,
        mut attrs: Vec<String>,
        class_attr: Option<String>,
        depth: usize,
        ctx: &Ctx,
    ) -> String {
        let pad = " ".repeat(depth * 2);
        if let Some(c) = class_attr {
            attrs.insert(0, c);
        }

        // A `field`-kind component is bound to a state, and the binding is the whole point of the kind:
        // `date from` has to become `value`/`onChange` or the control is decorative. Only `input` and
        // `select` were wired, so a package's own field emitted its state *name* as children — the same
        // shape of bug as the `select` that leaked its bound state name as element text.
        let is_field =
            crate::registry().get(&el.tag).is_some_and(|d| d.kind == guml_registry::TagKind::Field);
        let bound = if is_field { el.label().map(str::to_string) } else { None };
        if let Some(name) = &bound {
            attrs.push(format!("value={{{name}}}"));
            attrs.push(format!("onChange={{{}}}", setter(name)));

            // A choice among alternatives needs the alternatives. `value`/`onChange` alone left `radio` and
            // `combobox` emitting a control with nothing in it — bound correctly and offering the reader no
            // way to change the binding. The options are already reconciled from the two places they can be
            // written, `option` children or the bound state's domain, by the same function `select` uses; a
            // second rule here is a second chance to disagree about one document.
            let options = crate::select_options(self.program, el);
            if !options.is_empty() {
                let list = options.iter().map(|o| format!("{o:?}")).collect::<Vec<_>>().join(", ");
                attrs.push(format!("options={{[{list}]}}"));
            }
        }

        if let Some(action) = el.actions.first() {
            let js = self.lower_action(action, el.span, ctx);
            if !js.is_empty() {
                // `onSelect` for a container that offers choices, `onChange` for a field, `onClick`
                // otherwise — matching the three shapes the registry kinds describe.
                let handler = match crate::registry().get(&el.tag).map(|d| d.kind) {
                    Some(guml_registry::TagKind::Field) => "onChange",
                    Some(guml_registry::TagKind::Container) => "onSelect",
                    _ => "onClick",
                };
                attrs.push(format!("{handler}={{() => {{ {js}; }}}}"));
            }
        }

        let attr_str =
            if attrs.is_empty() { String::new() } else { format!(" {}", attrs.join(" ")) };

        // Children, and only real ones. A bound field's state name is not content, and a title that became
        // `aria-label` above must not be duplicated into the body.
        let named_by_title = crate::registry()
            .get(&el.tag)
            .is_some_and(|d| d.a11y.requires_label && el.attr("aria").is_none());
        let text = if bound.is_some() || named_by_title {
            None
        } else {
            el.content.clone().or_else(|| el.label().map(str::to_string))
        };

        // `option` children of a bound field became the `options` prop above. Rendering them again would put
        // a bare `<option>` inside a component that does not expect one — the list would appear twice, once
        // as data the component draws and once as stray markup it never asked for.
        let children: Vec<&Element> =
            el.children.iter().filter(|c| !(bound.is_some() && c.tag == "option")).collect();

        if children.is_empty() {
            let inner = text.map(|t| expr::lower_text_in(&t, ctx)).unwrap_or_default();
            if inner.is_empty() {
                return format!("{pad}<{tag_name}{attr_str} />\n");
            }
            return format!("{pad}<{tag_name}{attr_str}>{inner}</{tag_name}>\n");
        }

        let mut out = format!("{pad}<{tag_name}{attr_str}>\n");
        for child in children {
            let at = out.lines().count() as u32;
            let (rendered, marks) = self.element_marked(child, depth + 1, ctx);
            out.push_str(&rendered);
            self.rebase(marks, at);
        }
        let _ = writeln!(out, "{pad}</{tag_name}>");
        out
    }

    /// `on mount` / `on {expr}` → `useEffect`, paired with the source line each came from.
    ///
    /// The dependency array is *derived*, which is the entire reason the directive exists. A
    /// hand-written `useEffect` needs its deps to agree with its body, and they disagree in two
    /// directions: a missing entry reads a stale value, a spurious one re-runs forever. Here there is
    /// one list — the trigger — so the two cannot drift apart.
    fn effect_hooks(&mut self) -> Vec<(u32, String)> {
        let mut out = Vec::new();
        for e in &self.program.effects.clone() {
            let ctx = Ctx::default()
                .with_collections(&self.collections)
                .with_row_bool(&self.row_bool)
                .with_cse(&self.cse);
            // `lower_action` leaves the trailing `;` to its caller, as the event-handler path does.
            let body: Vec<String> = e
                .actions
                .iter()
                .map(|a| self.lower_action(a, e.span, &ctx))
                .filter(|js| !js.is_empty())
                .map(|js| format!("{js};"))
                .collect();
            let body = body.join(" ");

            // `mount` is the empty array. A trigger expression is lowered through the same path as
            // any other binding, so `on {filter}` and `{filter}` in prose cannot disagree about what
            // `filter` means.
            let deps = match &e.trigger {
                guml_ast::Trigger::Mount => String::new(),
                guml_ast::Trigger::Change(expr) => expr::lower_in(expr, &ctx),
            };
            let mut hook = String::new();
            let _ = writeln!(hook, "  useEffect(() => {{ {body} }}, [{deps}]);");
            out.push((e.span.line, hook));
        }
        out
    }

    /// Lower an action body, including resource mutations.
    /// Lower one action body to JavaScript statements.
    ///
    /// Takes a span rather than the element, because an `on` effect has no element and must lower
    /// through exactly this function — an effect running a different action language from a button's
    /// would be a second thing to learn for no gain.
    fn lower_action(&mut self, action: &str, span: guml_diagnostics::Span, ctx: &Ctx) -> String {
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

                // `>tasks.list` re-runs the resource's own GET. It takes no body and no row, in a row
                // or out of one, so it short-circuits everything below.
                if mutation == "list" {
                    stmts.push(format!("{fn_name}()"));
                    continue;
                }

                let body_js = if body.is_empty() {
                    // A body-less `save` on a row toggles the boolean, which is
                    // what a row checkbox means.
                    // Which field a body-less `save` toggles comes from the row type, not from the
                    // name `done` — the same rule `.open`/`.done` follow. An invoice's flag is
                    // `paid`, and hardcoding `done` here posted an empty body for it instead.
                    let flag = ctx.row_bool_field(head).to_string();
                    if in_row && mutation == "save" && ctx.item_fields.contains(&flag) {
                        format!("{{ {flag}: !item.{flag} }}")
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
                    let value = self.assigned_value(lhs, rhs.trim(), ctx);
                    stmts.push(format!("{}({value})", setter(lhs)));
                    continue;
                }
            }

            unsupported(self.diags, span, format!("action `{stmt}`"));
        }

        stmts.join("; ")
    }

    /// The right-hand side of `>state = value`, lowered.
    ///
    /// # A bare domain member is a string, not an identifier
    ///
    /// `state channel=all|web|ios` then `>channel = all` — the natural spelling, and the one the spec's own
    /// `state filter=all|open|done` example invites — emitted `setChannel(all)`. `all` is not a variable, so
    /// the output did not compile, and nothing said so: the expression lowerer sees a bare path and passes
    /// it through, which is right for `>draft = other` and wrong for a domain member.
    ///
    /// The domain is the discriminator and it is exact: `all` is a member of `channel`'s declared domain, so
    /// it is the string `"all"`. Anything not in the domain keeps whatever reading the expression lowerer
    /// gives it, so `>draft = query` still copies one state into another.
    fn assigned_value(&self, target: &str, rhs: &str, ctx: &Ctx) -> String {
        if let Some(state) = self.program.state(target) {
            if state.domain.iter().any(|m| m == rhs) {
                return format!("{rhs:?}");
            }
        }
        expr::lower_in(rhs, ctx)
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
    // `GET /api/events?channel={channel}` reads the state into the request.
    //
    // The braces used to reach the network verbatim: the emitted call was
    // `cached("/api/events?channel={channel}")`, so every request asked the server for a channel literally
    // named `{channel}`. `check_url` accepted it, no diagnostic fired, and the page looked like it was
    // filtering. Server-side filtering is the only filtering GUML can compose more than one of, so this is
    // not an exotic spelling.
    //
    // A *state* here, not `item` — a resource has no row. `interpolate_in` takes the scope prefix for that
    // reason and the mutation path keeps its own.
    let quoted = if url.contains('{') {
        format!("`{}`", interpolate_in(url, ""))
    } else {
        format!("{url:?}")
    };
    // Whatever the URL reads has to be in the callback's dependency array, or the closure keeps the value
    // the state had on the first render and the request never changes. Derived rather than written, so the
    // deps and the URL cannot disagree — which is the specific way a hand-written `useCallback` goes wrong.
    let url_deps = url_names(url);
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

    // The fetch, with cancellation — the part hand-written effects get wrong.
    //
    // Named rather than inlined into the mount effect, so `>tasks.list` can invoke it: from a Reload
    // button, or from a declared `on {filter} >tasks.list`. It returns its own cleanup, which is what
    // makes `useEffect(tasksList, deps)` correct with no wrapper — the abort still runs when the deps
    // change or the component unmounts, on every call site, without each one remembering to.
    // `cached` rather than `retrying` directly: it adds in-flight deduplication, stale-while-revalidate
    // and stale-on-failure, all of which every application needs and none of which is written by hand on
    // the first pass. See `crate::CACHE_TS` for what each one prevents.
    //
    // `alive` rather than only `AbortController`: a cache hit resolves synchronously-ish and may return
    // after unmount without the abort ever firing, so the guard is on the *setState* rather than on the
    // request. Setting state after unmount is React's most familiar warning and this is where it comes
    // from.
    let _ = write!(
        out,
        "\n  const {name}List = useCallback(() => {{\n\
         \x20   const controller = new AbortController();\n\
         \x20   let alive = true;\n\
         \x20   set{cap}Loading(true);\n\
         \x20   set{cap}Error(null);\n\
         \x20   cached<{ty}[]>({quoted}, {{ signal: controller.signal }})\n\
         \x20     .then((rows) => {{\n\
         \x20       if (alive) set{cap}(rows);\n\
         \x20     }})\n\
         \x20     .catch((err: unknown) => {{\n\
         \x20       if (!alive || (err instanceof Error && err.name === \"AbortError\")) return;\n\
         \x20       set{cap}Error(err instanceof Error ? err.message : \"Unknown error\");\n\
         \x20     }})\n\
         \x20     .finally(() => {{\n\
         \x20       if (alive) set{cap}Loading(false);\n\
         \x20     }});\n\
         \x20   return () => {{\n\
         \x20     alive = false;\n\
         \x20     controller.abort();\n\
         \x20   }};\n\
         \x20 }}, [{url_deps}]);\n\
         \n  useEffect({name}List, [{name}List]);\n"
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
            // `replace` and a delete both need to know *which* row, and `item` is only a parameter when
            // the mutation's path interpolates a field. A collection-level mutation —
            // `setPlan POST /api/subscription/plan {plan} optimistic` — took the same branch and emitted
            // `it === item` with no `item` in scope: the file did not compile, and nothing in the compiler
            // said so, because the row-context check (`GUML0101`) keys on the URL rather than on the
            // strategy.
            //
            // Without a row the patch applies to every row of the resource, which is the only reading the
            // request supports: the endpoint is the collection, so the update is the collection's. For the
            // one-row resource this is written for that is exactly right, and for a longer one it matches
            // what a `POST /api/thing` returning the new state of every row would do.
            Some(_) if method == "DELETE" && takes_item => {
                let _ =
                    writeln!(out, "      set{cap}((prev) => prev.filter((it) => it !== item));");
            }
            Some(_) if method == "DELETE" => {
                let _ = writeln!(out, "      set{cap}([]);");
            }
            Some(_) if takes_item => {
                let _ = writeln!(
                    out,
                    "      set{cap}((prev) => prev.map((it) => (it === item ? {{ ...it, ...body }} : it)));"
                );
            }
            Some(_) => {
                let _ = writeln!(
                    out,
                    "      set{cap}((prev) => prev.map((it) => ({{ ...it, ...body }})));"
                );
            }
            None => {}
        }

        let _ = write!(
            out,
            "      try {{\n\
             \x20       const res = await retrying({url_js}, {{\n\
             \x20         method: {method:?},\n\
             \x20         headers: {{ \"Content-Type\": \"application/json\" }},\n\
             \x20         body: JSON.stringify(body),\n\
             \x20       }});\n\
             \x20       if (!res.ok) throw new Error(`Request failed: ${{res.status}}`);\n"
        );

        // Invalidate the collection this mutation changed.
        //
        // The subtle bug this closes: without it, the next read of `/api/tasks` is a cache hit on the
        // *pre-mutation* list, so a row the user just added visibly disappears — and it looks like a
        // broken optimistic update rather than a stale cache. The prefix is the resource's own URL up to
        // its first interpolation, so a `PATCH /api/tasks/{id}` invalidates the list the row came from
        // and not merely the row's own URL, which nothing was caching.
        let _ = writeln!(out, "        invalidate({:?});", crate::invalidation_prefix(&r.url));

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

/// ` className="…"` for a tag, or nothing when the theme styles it with nothing.
///
/// A bare `className=""` is not wrong, it is just noise in a file someone opens — and "the emitted code
/// reads as hand-written" is a property this project holds itself to. It also keeps a theme that declines to
/// style a tag from showing up in the output at all, which is what `an_unstyled_tag_yields_nothing_rather_
/// than_a_default` asserts elsewhere.
fn class_attr_of(tag: &str) -> String {
    let c = classes(tag, &[]);
    if c.is_empty() { String::new() } else { format!(" className={c:?}") }
}

/// `/api/tasks/{id}` → `/api/tasks/${item.id}` inside a template literal.
fn interpolate_path(url: &str) -> String {
    interpolate_in(url, "item.")
}

/// The names a URL interpolates, comma-joined for a dependency array.
fn url_names(url: &str) -> String {
    let mut names: Vec<&str> = Vec::new();
    let mut rest = url;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        // The head identifier: `{user.id}` depends on `user`.
        let name = after[..close].split('.').next().unwrap_or("");
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
        rest = &after[close + 1..];
    }
    names.join(", ")
}

/// The same, with the scope named by the caller.
///
/// A mutation path reads the row (`item.`); a resource URL has no row and reads state (no prefix). Two
/// call sites, one rule, because a URL that interpolates in one place and not the other is the bug this
/// replaced — `GET /api/events?channel={channel}` reached `fetch` with its braces intact.
fn interpolate_in(url: &str, scope: &str) -> String {
    let mut out = String::new();
    let mut rest = url;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let _ = write!(out, "${{{scope}{}}}", &after[..close]);
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
///
/// The element itself comes from [`crate::element_for`], shared with the other backends so the same
/// document cannot get a `<nav>` from one and a `<div>` from another. What stays here is only what
/// depends on the *element's own content* — a button's `type`, a link's `href` — which is not a
/// property of the tag and so cannot live in a tag table.
/// Global attributes the compiler consumes itself, so they are never forwarded to a host component.
///
/// `if` is the conditional, `hidden` and `disabled` fold into rendering decisions, and `cols`/`gap`/`w` are
/// layout that joins the class list. A registry entry that lists one of these has misunderstood what it
/// owns — see the host-component branch in `plain`.
const COMPILER_OWNED: &[&str] =
    &["if", "cols", "gap", "w", "hidden", "loading", "busy", "strike", "where", "sort"];

pub(crate) fn html_tag(tag: &str, el: &Element) -> (Option<&'static str>, Vec<String>) {
    // A loaded registry package may declare what its component lowers to. Checked before the builtin
    // table so a package cannot be shadowed by a coincidental name — though `extend_from_json` already
    // refuses a package entry that collides with a builtin, so this ordering is belt and braces.
    let name = match crate::custom_element(tag) {
        Some((element, _)) => element,
        None => match crate::element_for(tag) {
            Some(name) => name,
            None => return (None, vec![]),
        },
    };
    let attrs: Vec<String> = match tag {
        // Inside a form the primary button submits; elsewhere it must not.
        "btn" => vec![if el.has_modifier("primary") && el.actions.is_empty() {
            "type=\"submit\"".to_string()
        } else {
            "type=\"button\"".to_string()
        }],
        "link" => {
            let href = el
                .route()
                .map(str::to_string)
                .or_else(|| el.anchor().map(|a| format!("#{a}")))
                .unwrap_or_else(|| "#".to_string());
            vec![format!("href={href:?}")]
        }
        "check" => vec!["type=\"checkbox\"".to_string()],
        "toggle" => vec!["type=\"checkbox\"".to_string(), "role=\"switch\"".to_string()],
        // `type` comes from the element's `kind` attribute when it has one; the caller
        // replaces this default. Emitting both produced `<input type="text" kind="email">`,
        // which `tsc` rejects — `kind` is not a DOM property.
        "input" => {
            vec![format!("type={:?}", el.attr("kind").and_then(|v| v.as_text()).unwrap_or("text"))]
        }

        // ---- 0.2 vocabulary ----
        //
        // A `role=` is added only where no element already implies it, which is the rule ARIA itself
        // states. `breadcrumb` and `pagination` are `<nav>` landmarks and need a name to be told apart;
        // `stepper` is an `<ol>` because the order *is* the meaning; `progress` is `<progress>` because
        // it works with no script at all.
        "toolbar" => vec!["role=\"toolbar\"".to_string()],
        "alert" => vec!["role=\"alert\"".to_string()],
        "menu" => vec!["role=\"menu\"".to_string()],
        "breadcrumb" => vec!["aria-label=\"Breadcrumb\"".to_string()],
        "pagination" => vec!["aria-label=\"Pagination\"".to_string()],
        "img" => {
            // `alt` is mandatory (`requires_label` in the registry), so by the time codegen runs the
            // analyser has already rejected an `img` without one. Defaulting to `""` here rather than
            // omitting the attribute keeps the emitted JSX valid in the error path, where the
            // diagnostic — not the output — is what the author is meant to read.
            let src = el.attr("src").and_then(|v| v.as_text()).unwrap_or("").to_string();
            let alt = el
                .attr("alt")
                .and_then(|v| v.as_text())
                .or_else(|| el.attr("aria").and_then(|v| v.as_text()))
                .unwrap_or("")
                .to_string();
            vec![format!("src={src:?}"), format!("alt={alt:?}")]
        }
        // A dialog needs `aria-modal` for a screen reader to treat the rest of the page as inert, and
        // an accessible name. The name comes from the title positional via `aria-label`, because
        // `aria-labelledby` would need a generated id that the no-JavaScript backend cannot share.
        "modal" | "drawer" => {
            let title = el.label().unwrap_or("Dialog").to_string();
            vec![
                "role=\"dialog\"".to_string(),
                "aria-modal=\"true\"".to_string(),
                format!("aria-label={title:?}"),
            ]
        }
        // `status` rather than `alert`: a toast is not an interruption, and `alert` preempts whatever
        // the screen reader is currently saying. `aria-live="polite"` is the same decision stated twice
        // for hosts whose AT does not map the role.
        "toast" => vec!["role=\"status\"".to_string(), "aria-live=\"polite\"".to_string()],
        _ => vec![],
    };
    (Some(name), attrs)
}

pub(crate) fn is_void(tag: &str) -> bool {
    matches!(tag, "input" | "check" | "toggle" | "divider" | "img")
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
        // **Asserted against neither palette.** These pinned `bg-slate-900` while slate was the
        // default, were rewritten to `bg-primary` when shadcn became it, and would have to be
        // rewritten a third time now that stock Tailwind is. A test that has to change every time the
        // default theme changes is testing the theme, not the thing it claims to.
        //
        // The property that actually holds is theme-independent: a modifier selects a *role*, so each
        // one must produce a distinct class string, and that string must be whatever the active theme
        // says it is. Which colour a role is belongs to the theme and to nothing here.
        // Note this compares `classes()` against itself rather than against `theme::active()`: the
        // backend appends the focus contract per focusable tag, so the two are *deliberately* not
        // equal, and asserting they were would pin the wrong thing again.
        let plain = classes("btn", &[]);
        let variants: Vec<String> =
            ["primary", "danger", "ghost"].iter().map(|m| classes("btn", &[m])).collect();

        for (modifier, styled) in ["primary", "danger", "ghost"].iter().zip(&variants) {
            assert_ne!(styled, &plain, "`{modifier}` selected no distinct role");
        }
        // And each intent differs from the others, not merely from the unmodified button — otherwise
        // "every modifier maps to something" would pass while they all mapped to the same thing.
        for (i, a) in variants.iter().enumerate() {
            for b in &variants[i + 1..] {
                assert_ne!(a, b, "two intents produced identical classes");
            }
        }
        // Two modifiers compose when they are in different groups; `sm` is size and `center` is alignment.
        assert!(classes("card", &["sm", "center"]).contains("max-w-sm"));
        assert!(classes("card", &["sm", "center"]).contains("text-center"));
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
