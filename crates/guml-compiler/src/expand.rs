//! Expands user-defined components (`def`) before anything else looks at the tree.
//!
//! # Why expansion, not a runtime concept
//!
//! A `def` is a compile-time macro. By the time codegen runs, the tree contains no trace of one —
//! which is what lets a `def` work in *every* backend, including the no-JavaScript HTML one, with no
//! per-backend support and no new runtime idea for a reader to learn. The emitted output looks exactly
//! as if the author had written the body inline, because that is what happened.
//!
//! It also keeps the language's "no imports, no framework concepts" property: a `def` is a way of not
//! repeating yourself, not a module system.
//!
//! # Where it runs
//!
//! Before resolution and validation. Everything downstream — the resolver, the accessibility lint, the
//! type checker, codegen — sees the expanded tree and needs no knowledge of `def` at all. The one cost
//! is that a diagnostic about expanded markup points at the `def` body, which is where the author wrote
//! it, so that is the honest answer rather than a compromise.
//!
//! # What substitution covers
//!
//! A parameter is substituted where the compiler can see it as a *value*:
//!
//! * a binding positional — `h {label}`
//! * an attribute value — `aria={name}`
//! * inside prose — `p Total: {value}`
//!
//! Not an action body. Actions lower to JavaScript, and substituting a parameter into one means
//! deciding whether the argument is a variable reference or a literal — a question the call site does
//! not answer. `GUML0097` rejects it rather than guessing, which is invariant 3 applied to a language
//! feature instead of to a backend.

use guml_ast::{DefDecl, Element, Positional, Program, Value};
use guml_diagnostics::{Code, Diagnostic, Diagnostics, Span};

/// Rewrite `program.tree`, replacing every call to a `def` with that def's body.
pub fn expand(program: &mut Program, diags: &mut Diagnostics) {
    if program.defs.is_empty() {
        return;
    }

    let defs = program.defs.clone();
    check_cycles(&defs, diags);
    report_unsupported_params(&defs, diags);
    report_unused_params(&defs, diags);

    // A def whose expansion would not terminate is skipped rather than expanded, so a cycle produces
    // one diagnostic instead of hanging the compiler.
    let cyclic = cyclic_names(&defs);

    let tree = std::mem::take(&mut program.tree);
    program.tree = expand_all(&tree, &defs, &cyclic, diags, 0);
}

/// Depth cap as a backstop. Cycle detection above should make this unreachable; a compiler that hangs
/// on a pathological document is worse than one that reports a limit.
const MAX_DEPTH: usize = 32;

fn expand_all(
    els: &[Element],
    defs: &[DefDecl],
    cyclic: &[String],
    diags: &mut Diagnostics,
    depth: usize,
) -> Vec<Element> {
    let mut out = Vec::with_capacity(els.len());
    for el in els {
        match defs.iter().find(|d| d.name == el.tag) {
            Some(def) if !cyclic.contains(&def.name) && depth < MAX_DEPTH => {
                out.extend(expand_call(el, def, defs, cyclic, diags, depth));
            }
            _ => {
                let mut copy = el.clone();
                copy.children = expand_all(&el.children, defs, cyclic, diags, depth);
                out.push(copy);
            }
        }
    }
    out
}

fn expand_call(
    call: &Element,
    def: &DefDecl,
    defs: &[DefDecl],
    cyclic: &[String],
    diags: &mut Diagnostics,
    depth: usize,
) -> Vec<Element> {
    // Arguments are the call's positionals, in order.
    let args: Vec<&Positional> = call.positionals.iter().collect();
    if args.len() != def.params.len() {
        diags.push(
            Diagnostic::error(
                Code::DefArity,
                format!(
                    "`{}` takes {} argument{}, but {} {} given",
                    def.name,
                    def.params.len(),
                    if def.params.len() == 1 { "" } else { "s" },
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" }
                ),
                call.span,
            )
            .with_help(format!("`def {} {}`", def.name, def.params.join(" "))),
        );
        return Vec::new();
    }

    // Children at a call site have nowhere to go: slots are deliberately not implemented yet, and
    // dropping them silently is the failure invariant 3 exists to prevent.
    if !call.children.is_empty() || !call.text_lines.is_empty() {
        diags.push(
            Diagnostic::error(
                Code::DefParamUnsupported,
                format!("`{}` cannot take children: a `def` has no slot to put them in", def.name),
                call.span,
            )
            .with_help(
                "move them inside the `def` body, or use a container at the call site instead",
            ),
        );
    }

    let bindings: Vec<(&str, &Positional)> =
        def.params.iter().map(String::as_str).zip(args).collect();

    let body: Vec<Element> =
        def.body.iter().map(|el| substitute(el, &bindings, call.span)).collect();

    // A def may call another def, so the substituted body is expanded in turn.
    expand_all(&body, defs, cyclic, diags, depth + 1)
}

/// Replace `{param}` throughout one element and its descendants.
fn substitute(el: &Element, bindings: &[(&str, &Positional)], _at: Span) -> Element {
    let mut out = el.clone();

    // A binding positional: `h {label}` becomes the argument itself, so passing a binding gives a
    // binding and passing a word gives a word.
    for p in &mut out.positionals {
        if let Positional::Binding(b) = p
            && let Some((_, arg)) = bindings.iter().find(|(name, _)| b.source.trim() == *name)
        {
            *p = (*arg).clone();
        }
    }

    for attr in &mut out.attrs {
        if let Value::Binding(b) = &attr.value
            && let Some((_, arg)) = bindings.iter().find(|(name, _)| b.source.trim() == *name)
        {
            attr.value = positional_as_value(arg);
        }
    }

    if let Some(content) = &out.content {
        out.content = Some(substitute_text(content, bindings));
    }
    out.text_lines = out.text_lines.iter().map(|l| substitute_text(l, bindings)).collect();

    out.children = out.children.iter().map(|c| substitute(c, bindings, _at)).collect();
    out
}

/// `Total: {value}` with `value` bound to `{revenue}` becomes `Total: {revenue}`; bound to `"Q3"` it
/// becomes `Total: Q3`.
fn substitute_text(text: &str, bindings: &[(&str, &Positional)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let inner = &after[..close];
        match bindings.iter().find(|(name, _)| inner.trim() == *name) {
            Some((_, arg)) => out.push_str(&positional_in_text(arg)),
            // Not a parameter: a binding on the surrounding document, left alone.
            None => {
                out.push('{');
                out.push_str(inner);
                out.push('}');
            }
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// How an argument reads inside prose. A binding stays a binding so it still interpolates; a literal
/// becomes the literal.
fn positional_in_text(arg: &Positional) -> String {
    match arg {
        Positional::Binding(b) => format!("{{{}}}", b.source),
        Positional::Text(t) => t.clone(),
        Positional::Modifier(m) => m.clone(),
        Positional::Route(r) => r.clone(),
        Positional::Anchor(a) => format!("#{a}"),
    }
}

fn positional_as_value(arg: &Positional) -> Value {
    match arg {
        Positional::Binding(b) => Value::Binding(b.clone()),
        Positional::Text(t) => Value::Str(t.clone()),
        Positional::Modifier(m) | Positional::Route(m) => Value::Word(m.clone()),
        Positional::Anchor(a) => Value::Word(format!("#{a}")),
    }
}

/// A parameter used in an action body cannot be substituted, so it is reported rather than left to
/// resolve against the surrounding document and fail with a confusing message.
fn report_unsupported_params(defs: &[DefDecl], diags: &mut Diagnostics) {
    fn walk(el: &Element, params: &[String], def: &str, diags: &mut Diagnostics) {
        for action in &el.actions {
            for param in params {
                // Word-boundary match, so `count` in a parameter list does not fire on `counter`.
                if action
                    .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .any(|w| w == param.as_str())
                {
                    diags.push(
                        Diagnostic::error(
                            Code::DefParamUnsupported,
                            format!(
                                "`def {def}` uses the parameter `{param}` in an action, which expansion cannot substitute"
                            ),
                            el.span,
                        )
                        .with_help(
                            "put the action at the call site, where the scope is unambiguous — `guml explain GUML0097`",
                        ),
                    );
                }
            }
        }
        for child in &el.children {
            walk(child, params, def, diags);
        }
    }

    for def in defs {
        for el in &def.body {
            walk(el, &def.params, &def.name, diags);
        }
    }
}

/// A parameter nothing in the body reads is almost always a mistake, and it is free to notice —
/// exactly the reasoning behind `GUML0074` for an unused `state`.
fn report_unused_params(defs: &[DefDecl], diags: &mut Diagnostics) {
    for def in defs {
        let mut used = std::collections::BTreeSet::new();
        fn walk(el: &Element, used: &mut std::collections::BTreeSet<String>) {
            for p in &el.positionals {
                if let Positional::Binding(b) = p {
                    used.insert(b.source.trim().to_string());
                }
            }
            for a in &el.attrs {
                if let Value::Binding(b) = &a.value {
                    used.insert(b.source.trim().to_string());
                }
            }
            for text in el.content.iter().chain(el.text_lines.iter()) {
                for name in guml_syntax::expr::interpolations(text) {
                    used.insert(name.trim().to_string());
                }
            }
            for child in &el.children {
                walk(child, used);
            }
        }
        for el in &def.body {
            walk(el, &mut used);
        }

        for param in &def.params {
            if !used.contains(param) {
                diags.push(
                    Diagnostic::warning(
                        Code::DefParamUnsupported,
                        format!("`def {}` never uses its parameter `{param}`", def.name),
                        def.span,
                    )
                    .with_help("remove it, or reference it as `{param}` in the body"),
                );
            }
        }
    }
}

/// Names that take part in a cycle.
fn cyclic_names(defs: &[DefDecl]) -> Vec<String> {
    defs.iter()
        .filter(|d| reaches(&d.name, &d.name, defs, &mut Vec::new()))
        .map(|d| d.name.clone())
        .collect()
}

fn check_cycles(defs: &[DefDecl], diags: &mut Diagnostics) {
    for def in defs {
        let mut path = Vec::new();
        if reaches(&def.name, &def.name, defs, &mut path) {
            let mut shown = vec![def.name.clone()];
            shown.extend(path);
            diags.push(
                Diagnostic::error(
                    Code::RecursiveDef,
                    format!("`{}` expands into itself: {}", def.name, shown.join(" → ")),
                    def.span,
                )
                .with_help(
                    "expansion happens at compile time, so there is no base case to stop at; a repeating structure wants `list` over a resource",
                ),
            );
        }
    }
}

/// Whether `from`'s body can reach `target`, recording the path taken.
fn reaches(from: &str, target: &str, defs: &[DefDecl], path: &mut Vec<String>) -> bool {
    let Some(def) = defs.iter().find(|d| d.name == from) else { return false };

    fn calls(els: &[Element], out: &mut Vec<String>) {
        for el in els {
            out.push(el.tag.clone());
            calls(&el.children, out);
        }
    }
    let mut tags = Vec::new();
    calls(&def.body, &mut tags);

    for tag in tags {
        if tag == target {
            path.push(tag);
            return true;
        }
        // Only follow other defs, and only ones not already on the path, or a diamond would loop.
        if defs.iter().any(|d| d.name == tag) && !path.contains(&tag) {
            path.push(tag.clone());
            if reaches(&tag, target, defs, path) {
                return true;
            }
            path.pop();
        }
    }
    false
}
