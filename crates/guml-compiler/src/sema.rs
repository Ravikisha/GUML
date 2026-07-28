//! Resolver-lite and the accessibility lint.
//!
//! Three diagnostic codes were declared long before anything emitted them:
//! `GUML0033` (unknown state), `GUML0050` and `GUML0051` (unlabelled controls).
//! Until this pass existed, "an unlabelled control is a compile error" was a
//! claim in the documentation rather than behaviour in the compiler — which is
//! exactly the kind of gap that makes the "convention as correctness" argument
//! unfalsifiable. This closes it.
//!
//! It is deliberately *not* the full resolver from ROADMAP Phase 3: there is no
//! type checking and no expression parsing yet, so binding paths are matched by
//! their head identifier only.

use guml_ast::{Element, Positional, Program, Value};
use guml_diagnostics::{Code, Diagnostic, Diagnostics};
use guml_registry::Registry;

/// Names a binding or action may legally reference: declared state, declared
/// resources, and — inside a repeater — the fields of the iterated type.
struct Scope {
    names: Vec<String>,
}

impl Scope {
    fn knows(&self, head: &str) -> bool {
        self.names.iter().any(|n| n == head)
    }

    fn with(&self, extra: impl IntoIterator<Item = String>) -> Scope {
        let mut names = self.names.clone();
        names.extend(extra);
        Scope { names }
    }
}

pub fn analyse(program: &Program, reg: &Registry, diags: &mut Diagnostics) {
    let mut names: Vec<String> = program.states.iter().map(|s| s.name.clone()).collect();
    names.extend(program.resources.iter().map(|r| r.name.clone()));

    let scope = Scope { names };
    for el in &program.tree {
        walk(el, program, reg, &scope, diags);
    }
}

fn walk(el: &Element, program: &Program, reg: &Registry, scope: &Scope, diags: &mut Diagnostics) {
    walk_in(el, program, reg, scope, diags, false);
}

/// `named_by_row` is true when this element is part of an item template inside a
/// repeater whose row carries a text binding the compiler can name controls from.
fn walk_in(
    el: &Element,
    program: &Program,
    reg: &Registry,
    scope: &Scope,
    diags: &mut Diagnostics,
    named_by_row: bool,
) {
    check_label(el, reg, diags, named_by_row);

    for b in bindings_of(el) {
        check_reference(&b, el, scope, diags, "binding");
    }
    for action in &el.actions {
        for target in action_targets(action) {
            check_reference(&target, el, scope, diags, "action");
        }
    }

    // A repeater introduces its item's fields into scope for its children.
    let child_scope = match repeater_fields(el, program) {
        Some(fields) => scope.with(fields),
        None => Scope { names: scope.names.clone() },
    };

    // Inside a repeater, a row that renders a text binding gives the compiler
    // something to name its controls from — the same thing a human does when
    // labelling a row checkbox from the row's title. See `row_text_binding`.
    let children_named_by_row =
        matches!(el.tag.as_str(), "list" | "table") && row_text_binding(el).is_some();

    for child in &el.children {
        walk_in(child, program, reg, &child_scope, diags, children_named_by_row);
    }
}

/// The binding a repeater's row is identified by: the first text-kind child that
/// renders a binding. `text {title}` in a task row yields `title`.
pub fn row_text_binding(repeater: &Element) -> Option<String> {
    repeater.children.iter().find_map(|child| {
        if !matches!(child.tag.as_str(), "text" | "p" | "h" | "h1" | "h2" | "h3" | "head") {
            return None;
        }
        if let Some(b) = child.binding() {
            return Some(b.to_string());
        }
        child
            .content
            .as_deref()
            .and_then(|c| interpolations(c).into_iter().next())
    })
}

/// `GUML0050` / `GUML0051` — accessible names.
///
/// Severity is graded by how much the compiler can recover on the author's
/// behalf, which is the whole "convention as correctness" idea applied to names:
///
/// * explicit `aria`/`title`, or a text label — fine.
/// * a control in a repeater row that renders a text binding — fine; the backend
///   names it from that binding, exactly as a human would name a row checkbox
///   from the row's title.
/// * a field whose only hint is a `placeholder` — **warning**. A placeholder is
///   not an accessible name (axe-core agrees), but it is not nothing either.
/// * nothing at all — **error**.
fn check_label(el: &Element, reg: &Registry, diags: &mut Diagnostics, named_by_row: bool) {
    let Some(def) = reg.get(&el.tag) else { return };
    if !def.requires_label {
        return;
    }

    let is_field = matches!(el.tag.as_str(), "input" | "select");
    // A field's first positional is the state it binds, not a label.
    let has_text = (!is_field && el.label().is_some()) || el.content.is_some();
    let has_aria = el.attr("aria").is_some() || el.attr("title").is_some();

    if has_text || has_aria || (named_by_row && !is_field) {
        return;
    }

    if is_field && el.attr("placeholder").is_some() {
        diags.push(
            Diagnostic::warning(
                Code::InputWithoutLabel,
                format!("`{}` is labelled only by its placeholder", el.tag),
                el.span,
            )
            .with_help("a placeholder disappears on input and is not an accessible name; add `aria=\"…\"`"),
        );
        return;
    }

    let (code, what) = if is_field {
        (Code::InputWithoutLabel, "field")
    } else {
        (Code::IconControlWithoutLabel, "control")
    };

    diags.push(
        Diagnostic::error(code, format!("`{}` has no accessible name", el.tag), el.span)
            .with_help(format!(
                "give the {what} a text label, or add `aria=\"…\"` describing what it does"
            ))
            .with_suggestion(format!("{} aria=\"…\"", el.tag)),
    );
}

/// `GUML0033` — a binding or action naming something that was never declared.
fn check_reference(
    reference: &str,
    el: &Element,
    scope: &Scope,
    diags: &mut Diagnostics,
    kind: &str,
) {
    let head = head_ident(reference);
    if head.is_empty() || scope.knows(head) {
        return;
    }
    // Literals, and anything that is not an identifier, are not references.
    if !head.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        return;
    }

    let mut d = Diagnostic::error(
        Code::UnknownState,
        format!("{kind} refers to `{head}`, which is not declared"),
        el.span,
    );
    if let Some(near) = nearest(head, &scope.names) {
        d = d
            .with_help(format!("did you mean `{near}`?"))
            .with_suggestion(near.clone());
    } else {
        d = d.with_help(format!(
            "declare it with `state {head}=…`, or a `data {head}` resource"
        ));
    }
    diags.push(d);
}

/// Every binding on an element: positionals, attribute values, and the
/// interpolations inside prose content.
fn bindings_of(el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    for p in &el.positionals {
        if let Positional::Binding(b) = p {
            out.push(b.clone());
        }
    }
    for a in &el.attrs {
        if let Value::Binding(b) = &a.value {
            out.push(b.clone());
        }
    }
    if let Some(content) = &el.content {
        out.extend(interpolations(content));
    }
    out
}

/// `Tasks — {tasks.open.count} open` -> ["tasks.open.count"]
fn interpolations(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                out.push(after[..close].trim().to_string());
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    out
}

/// Assignment and increment targets in an action body. Mutation calls
/// (`tasks.add{…}`) resolve through their resource name.
fn action_targets(action: &str) -> Vec<String> {
    action
        .split(';')
        .filter_map(|stmt| {
            let s = stmt.trim();
            if s.is_empty() {
                return None;
            }
            let target = s
                .strip_suffix("++")
                .or_else(|| s.strip_suffix("--"))
                .map(str::trim)
                .or_else(|| s.split_once('=').map(|(lhs, _)| lhs.trim()))
                .unwrap_or(s);
            Some(target.to_string())
        })
        .collect()
}

/// Leading identifier of a path or expression: `!draft.trim()` -> `draft`.
fn head_ident(expr: &str) -> &str {
    let trimmed = expr.trim_start_matches(|c: char| !(c.is_alphanumeric() || c == '_'));
    let end = trimmed
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(trimmed.len());
    &trimmed[..end]
}

/// Fields a repeater puts in scope for its children, from its resource's type.
fn repeater_fields(el: &Element, program: &Program) -> Option<Vec<String>> {
    if !matches!(el.tag.as_str(), "list" | "table") {
        return None;
    }
    let source = el.label()?;
    let resource = program.resources.iter().find(|r| r.name == source)?;
    let ty = resource.ty.trim_end_matches("[]");
    let decl = program.types.iter().find(|t| t.name == ty)?;
    Some(decl.fields.iter().map(|f| f.name.clone()).collect())
}

pub(crate) fn nearest(unknown: &str, candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .map(|c| (c, distance(unknown, c)))
        .filter(|(_, d)| *d <= 2)
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c.clone())
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use guml_diagnostics::{Code, Diagnostics};
    use guml_registry::Registry;

    fn check(src: &str) -> Diagnostics {
        let reg = Registry::builtin();
        let parsed = guml_parser::parse(src, &reg);
        let mut diags = parsed.diagnostics;
        super::analyse(&parsed.program, &reg, &mut diags);
        diags
    }

    fn codes(src: &str) -> Vec<Code> {
        check(src).items.iter().map(|d| d.code).collect()
    }

    #[test]
    fn declared_state_is_fine() {
        let src = "page P\nstate count=0\n\nbtn Add primary >count++\nmetric {count}\n";
        assert!(!check(src).has_errors());
    }

    #[test]
    fn unknown_state_in_a_binding_is_an_error() {
        let src = "page P\nstate count=0\n\nmetric {kount}\n";
        let d = check(src);
        assert!(d.items.iter().any(|d| d.code == Code::UnknownState));
        // The typo is one edit away, so the fix is offered directly.
        assert_eq!(
            d.items
                .iter()
                .find(|d| d.code == Code::UnknownState)
                .and_then(|d| d.suggestion.clone()),
            Some("count".to_string())
        );
    }

    #[test]
    fn unknown_action_target_is_an_error() {
        let src = "page P\nstate count=0\n\nbtn Reset quiet >total=0\n";
        assert!(codes(src).contains(&Code::UnknownState));
    }

    #[test]
    fn resources_and_repeater_fields_are_in_scope() {
        let src = "page Tasks\n\
                   type Task {id, title, done:bool}\n\
                   data tasks:Task[] GET /api/tasks\n\
                   \x20 drop DELETE /api/tasks/{id} optimistic\n\
                   head Tasks — {tasks.count} open\n\
                   list tasks\n\
                   \x20 text {title} strike={done}\n\
                   \x20 btn Delete quiet >tasks.drop\n";
        assert!(!check(src).has_errors(), "{:?}", check(src).items);
    }

    #[test]
    fn a_field_from_the_wrong_type_is_still_caught() {
        let src = "page Tasks\n\
                   type Task {id, title}\n\
                   data tasks:Task[] GET /api/tasks\n\
                   list tasks\n\
                   \x20 text {titel}\n";
        assert!(codes(src).contains(&Code::UnknownState));
    }

    #[test]
    fn item_fields_do_not_leak_outside_the_repeater() {
        let src = "page Tasks\n\
                   type Task {id, title}\n\
                   data tasks:Task[] GET /api/tasks\n\
                   list tasks\n\
                   \x20 text {title}\n\
                   p {title}\n";
        assert!(codes(src).contains(&Code::UnknownState));
    }

    #[test]
    fn a_control_without_an_accessible_name_fails() {
        let src = "page P\nstate n=0\n\nbtn quiet >n++\n";
        assert!(codes(src).contains(&Code::IconControlWithoutLabel));
    }

    #[test]
    fn aria_satisfies_the_label_requirement() {
        let src = "page P\nstate n=0\n\nbtn quiet aria=\"Add one\" >n++\n";
        assert!(!check(src).has_errors());
    }

    #[test]
    fn a_field_with_nothing_at_all_is_an_error() {
        // The first positional of a field is the state it binds, not a label.
        let d = check("page P\nstate draft=\"\"\n\ninput draft\n");
        assert!(d.has_errors());
        assert!(d.items.iter().any(|d| d.code == Code::InputWithoutLabel));
    }

    #[test]
    fn a_placeholder_only_field_warns_rather_than_failing() {
        let d = check("page P\nstate draft=\"\"\n\ninput draft placeholder=\"Add a task…\"\n");
        assert!(!d.has_errors(), "placeholder-only is a warning, not an error");
        assert!(d.items.iter().any(|d| d.code == Code::InputWithoutLabel));
    }

    #[test]
    fn aria_on_a_field_is_clean() {
        assert!(!check("page P\nstate draft=\"\"\n\ninput draft aria=\"New task\"\n").has_errors());
    }

    #[test]
    fn a_row_control_is_named_by_the_rows_text_binding() {
        // `check {done}` has no label of its own, but the row renders `{title}`,
        // so the backend can name it — the same call a human makes.
        let src = "page Tasks\n\
                   type Task {id, title, done:bool}\n\
                   data tasks:Task[] GET /api/tasks\n\
                   list tasks\n\
                   \x20 check {done}\n\
                   \x20 text {title}\n";
        assert!(!check(src).has_errors(), "{:?}", check(src).items);
    }

    #[test]
    fn a_row_with_no_text_binding_cannot_name_its_controls() {
        let src = "page Tasks\n\
                   type Task {id, title, done:bool}\n\
                   data tasks:Task[] GET /api/tasks\n\
                   list tasks\n\
                   \x20 check {done}\n";
        assert!(codes(src).contains(&Code::IconControlWithoutLabel));
    }

    #[test]
    fn literals_are_not_treated_as_references() {
        let src = "page P\nstate n=0\n\nbtn Reset quiet >n=0\ntext {42}\n";
        assert!(!check(src).has_errors());
    }
}
