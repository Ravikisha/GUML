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
use guml_registry::{Registry, TagKind};

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
    // A `js` block's own `const`/`let`/`function` names, so the escape hatch composes with the language
    // instead of being a dead end — see `Element::escape_declares` for the case that forced it.
    names.extend(js_declarations(program));

    let scope = Scope { names };
    for el in &program.tree {
        walk(el, program, reg, &scope, diags);
    }

    // Declared effects. Both halves are references: `on {filter} >tasks.list` reads `filter` and calls
    // `tasks`, and either can name something that does not exist. Skipping them meant a typo in an
    // effect compiled to a call to `undefined` — the whole class of mistake `GUML0033` exists to catch,
    // silently exempt because the pass only walked the element tree.
    for e in &program.effects {
        if let guml_ast::Trigger::Change(expr) = &e.trigger {
            check_reference(expr, e.span, &scope, diags, "effect trigger");
        }
        for action in &e.actions {
            for target in action_targets(action) {
                check_reference(&target, e.span, &scope, diags, "effect action");
            }
        }
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
    check_positionals(el, reg, diags);
    check_children(el, reg, diags);
    check_modifier_in_prose(el, reg, diags);

    for b in bindings_of(el) {
        check_reference(&b, el.span, scope, diags, "binding");
    }
    for action in &el.actions {
        for target in action_targets(action) {
            check_reference(&target, el.span, scope, diags, "action");
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
        child.content.as_deref().and_then(|c| interpolations(c).into_iter().next())
    })
}

/// `GUML0099` — bare positional words the tag has nowhere to put.
///
/// # The bug this exists for
///
/// `btn Add task primary` parsed as `Text("Add")`, `Text("task")`, `Modifier("primary")`. Codegen calls
/// `el.label()`, which returns the *first* text positional, so the emitted button read
/// `<button>Add</button>`. The word `task` was gone: no warning, no `TODO`, exit code 0.
///
/// That is the same failure as the older `p Set x=1 to enable the flag` bug — a rule that quietly
/// discards part of what the author wrote — and it is forbidden by the same reasoning. Prose surviving
/// verbatim is the content-floor claim, and a compiler that deletes a word from a label is not
/// compressing, it is losing data.
///
/// # Why report instead of joining
///
/// Joining the extra words into one label (`"Add task"`) is right for a `btn` and *wrong* for a `tier`,
/// whose three positionals are name, price and blurb — joining there would turn three slots into one.
/// So the arity is per-entry registry data (`ComponentDef::positionals`), and where a document exceeds
/// it the compiler reports rather than guesses. The suggested fix is quoting, which is unambiguous, so
/// `guml fix` applies it with no model call.
fn check_positionals(el: &Element, reg: &Registry, diags: &mut Diagnostics) {
    let Some(def) = reg.get(&el.tag) else { return };
    // An empty declaration means "unspecified", not "zero". A `TagKind::Text` tag has no positionals at
    // all — its line remainder is prose — so there is nothing here to count.
    if def.positionals.is_empty() {
        return;
    }

    let texts: Vec<&str> = el
        .positionals
        .iter()
        .filter_map(|p| match p {
            Positional::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    if texts.len() <= def.positionals.len() {
        return;
    }

    // Two readings of the same word, and only one fix can be applied.
    //
    // `btn Save primry` has two bare words where one slot exists, so this rule wants to quote them into
    // `"Save primry"`. But `primry` is a distance-1 miss for the modifier `primary`, and the parser has
    // already said so as `GUML0031` with its own applicable suggestion. Emitting both leaves `guml fix`
    // choosing between contradictory edits, and quoting a mistyped modifier into the label is the worse
    // of the two — it makes the typo permanent and silent.
    //
    // So the modifier reading wins, and this stays quiet. If it was the wrong reading, the next re-check
    // round sees the repaired line and reports whatever is still true; `guml fix` runs bounded rounds
    // precisely so a fix that reveals another problem is not the end of the story.
    let overflow_is_a_modifier_typo = texts[def.positionals.len() - 1..]
        .iter()
        .any(|w| Registry::suggest_modifier_close(w).is_some());
    if overflow_is_a_modifier_typo {
        return;
    }

    // The overflow words, in order, as the author wrote them.
    let extra = &texts[def.positionals.len() - 1..];
    let joined = extra.join(" ");
    let slots = def.positionals.join(", ");
    let slot_word = if def.positionals.len() == 1 { "slot" } else { "slots" };

    // The whole element line rewritten with the overflow quoted into the last slot. Reconstructed rather
    // than patched in place, because the extra words are not necessarily contiguous in the source once
    // modifiers are interleaved (`btn Add primary task`).
    //
    // **Everything on the line has to be reproduced, not just the positionals.** The first version of
    // this rebuilt only the positionals, so the suggestion for
    // `section #work Selected work cols=3` was `section #work "Selected work"` — it silently deleted
    // `cols=3`. A suggestion that drops an attribute while fixing a dropped word is the same defect
    // wearing a different hat, and `guml fix` applies these unattended.
    let mut rebuilt = vec![el.tag.clone()];
    let mut text_seen = 0usize;
    for p in &el.positionals {
        match p {
            Positional::Text(_) => {
                text_seen += 1;
                if text_seen < def.positionals.len() {
                    rebuilt.push(format!("{:?}", texts[text_seen - 1]));
                } else if text_seen == def.positionals.len() {
                    rebuilt.push(format!("{joined:?}"));
                }
                // Later text positionals were folded into `joined` above.
            }
            Positional::Modifier(m) => rebuilt.push(m.clone()),
            Positional::Route(r) => rebuilt.push(r.clone()),
            Positional::Anchor(a) => rebuilt.push(format!("#{a}")),
            Positional::Binding(b) => rebuilt.push(format!("{{{}}}", b.source)),
        }
    }
    for a in &el.attrs {
        rebuilt.push(match &a.value {
            Value::Flag => a.name.clone(),
            v => format!("{}={}", a.name, value_source(v)),
        });
    }
    for action in &el.actions {
        rebuilt.push(format!(">{action}"));
    }
    // `| prose` is last on the line, and only reachable on a tag that takes both positionals and
    // content — a `card "Title" | blurb`.
    if let Some(content) = &el.content {
        rebuilt.push(format!("| {content}"));
    }

    diags.push(
        Diagnostic::error(
            Code::DroppedPositional,
            format!(
                "`{}` reads {} positional {slot_word} ({slots}), and this line has {} bare words",
                el.tag,
                def.positionals.len(),
                texts.len()
            ),
            el.span,
        )
        .with_help(format!(
            "quote them so they stay one value — otherwise `{joined}` would be dropped from the output"
        ))
        .with_suggestion(rebuilt.join(" ")),
    );
}

/// An attribute value rendered back as the GUML that would parse to it.
///
/// Only used to build the `GUML0099` suggestion, so it has one requirement: round-tripping. A `Str`
/// re-quotes, a `Binding` reproduces the author's own text rather than the lowered expression — the
/// source is what belongs in a suggested edit.
fn value_source(v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{s:?}"),
        Value::Num(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        Value::Bool(b) => b.to_string(),
        Value::Word(w) => w.clone(),
        Value::Binding(b) => format!("{{{}}}", b.source),
        Value::Flag => String::new(),
    }
}

/// `GUML0102` — prose on a text tag that begins with a modifier, so the modifier renders as text.
///
/// # Why this warns instead of reclassifying the word
///
/// A text tag's remainder is prose, verbatim. That rule is frozen and it is load-bearing: the moment the
/// compiler starts pulling a leading word out of prose when it happens to match the modifier vocabulary,
/// `p center the label under the field` loses a word. Prose surviving intact is the content-floor claim,
/// and a rule that silently edits it is data loss dressed as compression.
///
/// So the text stays as written and the compiler says what it noticed. The case is not hypothetical:
/// `badge`'s own registry doc said "use `danger`/`primary`/`quiet` for tone", `themes/tailwind.json` carried
/// three tone rules keyed on those modifiers, and `badge danger Breaking` compiled with zero diagnostics
/// and rendered the literal string "danger Breaking". Two parts of the compiler advertised a feature the
/// third could not deliver, and the only reason it went unnoticed is that no fixture used it.
///
/// `badge` is a positional tag now, so that spelling works. This check covers the rest of the kind.
fn check_modifier_in_prose(el: &Element, reg: &Registry, diags: &mut Diagnostics) {
    let Some(def) = reg.get(&el.tag) else { return };
    if def.kind != TagKind::Text {
        return;
    }
    let Some(prose) = el.content.as_deref() else { return };
    let mut words = prose.split_whitespace();
    let Some(first) = words.next() else { return };
    // Exact match, so it is already case-sensitive: `p Start free today.` is prose, because the modifier
    // is `start` and the word is `Start`.
    if !guml_registry::MODIFIERS.contains(&first) {
        return;
    }

    // Case, as the discriminator between the two readings — and it is needed, because the first version of
    // this warned on `p center the label under the field`, which is ordinary prose that happens to begin
    // with a layout modifier. A warning that fires on legitimate content is worse than no warning: it
    // trains an author to ignore the code, and it would fire on marketing copy in `fixtures/c.guml`.
    //
    // The shape of the mistake is a lowercase modifier followed by *capitalised* content, because the
    // content was meant to be the label and the modifier was meant to be a modifier — `danger Breaking`,
    // `quiet Draft`. Sentence prose continues in lowercase. A remainder that is nothing but the modifier
    // is unambiguous either way.
    let looks_like_a_lever = match words.next() {
        None => true,
        Some(second) => second.starts_with(|c: char| c.is_uppercase() || c == '{'),
    };
    if !looks_like_a_lever {
        return;
    }

    // The prose is quoted back in the message so it shows exactly what will render.
    diags.push(
        Diagnostic::warning(
            Code::ModifierInProse,
            format!(
                "`{}` is a text tag, so its line renders verbatim as `{}` — the leading `{first}` is text, not a modifier",
                el.tag, prose
            ),
            el.span,
        )
        .with_help(
            "a text tag takes its remainder as prose and that rule does not bend; for tone, wrap the \
             line in a container that accepts modifiers, such as `alert danger`",
        ),
    );
}

/// `GUML0100` — a child the component's registry entry does not admit, or a required child that is
/// absent.
///
/// The constraint is registry data rather than a `match` arm here, which is the only version of the rule
/// a *loaded* third-party component can use. `select` accepting only `option` and `stepper` requiring at
/// least one `step` are the same mechanism a host's own `combobox` gets for free.
fn check_children(el: &Element, reg: &Registry, diags: &mut Diagnostics) {
    let Some(def) = reg.get(&el.tag) else { return };
    if def.children.is_unconstrained() {
        return;
    }

    for child in &el.children {
        if def.children.admits(&child.tag) {
            continue;
        }
        let allowed = if def.children.is_leaf() {
            format!("`{}` takes no children", el.tag)
        } else if def.children.allow.is_empty() {
            format!("`{}` does not accept a `{}` child", el.tag, child.tag)
        } else {
            format!(
                "`{}` accepts only {}",
                el.tag,
                def.children
                    .allow
                    .iter()
                    .map(|a| format!("`{a}`"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            )
        };
        diags.push(
            Diagnostic::error(
                Code::BadChild,
                format!("`{}` is not a valid child of `{}`", child.tag, el.tag),
                child.span,
            )
            .with_help(allowed),
        );
    }

    for required in &def.children.require {
        if el.children.iter().any(|c| &c.tag == required) {
            continue;
        }
        diags.push(
            Diagnostic::error(
                Code::BadChild,
                format!("`{}` needs at least one `{required}` child", el.tag),
                el.span,
            )
            .with_help(format!(
                "add an indented `{required}` line — without one this renders an empty container"
            )),
        );
    }
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
    if !def.requires_label() {
        return;
    }

    let is_field = matches!(el.tag.as_str(), "input" | "select");
    // A field's first positional is the state it binds, not a label.
    let has_text = (!is_field && el.label().is_some()) || el.content.is_some();
    // `alt` is the accessible name of an image — that is what the attribute *is*, and it is the
    // spelling every author already knows. Without this, `img src="/logo.png" alt="Our logo"` was
    // rejected as having no accessible name while carrying one, and the suggestion told the author to
    // add `aria=""` next to the `alt` they had already written. An empty `alt` is deliberately not
    // accepted: `alt=""` is the correct, meaningful way to mark an image as decorative, and a decorative
    // image is exactly the case this check should stay quiet about.
    let has_alt = el.tag == "img" && el.attr("alt").is_some();
    let has_aria = el.attr("aria").is_some() || el.attr("title").is_some();

    if has_text || has_aria || has_alt || (named_by_row && !is_field) {
        return;
    }

    if is_field && el.attr("placeholder").is_some() {
        diags.push(
            Diagnostic::warning(
                Code::InputWithoutLabel,
                format!("`{}` is labelled only by its placeholder", el.tag),
                el.span,
            )
            .with_help(
                "a placeholder disappears on input and is not an accessible name; add `aria=\"…\"`",
            ),
        );
        return;
    }

    // A field bound to a state can be named from that binding.
    //
    // `select colour` is a select whose purpose is the word `colour`, and a backend emitting
    // `aria-label="colour"` produces something a screen reader can announce — which is the entire
    // point of the check. Refusing it outright meant the compiler held a usable name and declined to
    // use it, and that was the **second most common reason model-generated GUML failed to compile**
    // (`bench/gen`, unchanged between an 8B and a 70B model).
    //
    // A **warning**, not silence, and the grading is the point. A state name is a variable name: it
    // is usually a real word and occasionally `c` or `x1`, and the compiler cannot tell which. So the
    // document compiles, the backend emits a name rather than nothing, and the author is told a
    // better one exists — the same treatment `placeholder` gets above, for the same reason.
    //
    // Only for a *field*: a field's first positional is the state it binds, so there is a name to
    // derive. A `btn` with no text has nothing.
    if is_field {
        if let Some(bound) = el.label() {
            diags.push(
                Diagnostic::warning(
                    Code::InputWithoutLabel,
                    format!("`{}` is named from the state it binds (`{bound}`)", el.tag),
                    el.span,
                )
                .with_help(
                    "a state name is a variable name and may read poorly to a screen reader; \
                     add `aria=\"…\"` if it does",
                )
                .with_suggestion(format!("{} {bound} aria=\"{bound}\"", el.tag)),
            );
            return;
        }
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
    span: guml_diagnostics::Span,
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
        span,
    );
    if let Some(near) = nearest(head, &scope.names) {
        d = d.with_help(format!("did you mean `{near}`?")).with_suggestion(near.clone());
    } else {
        d = d.with_help(format!("declare it with `state {head}=…`, or a `data {head}` resource"));
    }
    diags.push(d);
}

/// Every binding on an element: positionals, attribute values, and the
/// interpolations inside prose content.
fn bindings_of(el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    for p in &el.positionals {
        if let Positional::Binding(b) = p {
            out.push(b.source.clone());
        }
    }
    for a in &el.attrs {
        match &a.value {
            Value::Binding(b) => out.push(b.source.clone()),
            // A `{…}` inside a *quoted* attribute value is an interpolation, and it was resolved by
            // nothing. `aria="Delete {ttle}"` compiled clean and emitted ``aria-label={`Delete
            // ${item.ttle}`}`` — a read of a field that does not exist, so the accessible name came out as
            // the string "undefined" at runtime. Codegen had always interpolated these; only the resolver
            // did not know they were references, which is the asymmetry that let it through.
            //
            // Found by the mutation gate in `tests/mutation.rs`: a one-character typo inside an `aria=`
            // string was one of the mutants the compiler did not detect.
            Value::Str(s) => out.extend(interpolations(s)),
            _ => {}
        }
    }
    if let Some(content) = &el.content {
        out.extend(interpolations(content));
    }
    // A text tag's prose lands in `text_lines` rather than `content` for `tier`/`faq` bodies, and those are
    // verbatim content lines rather than expressions — deliberately not walked here.
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
    let end = trimmed.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(trimmed.len());
    &trimmed[..end]
}

/// Fields a repeater puts in scope for its children, from its resource's type.
fn repeater_fields(el: &Element, program: &Program) -> Option<Vec<String>> {
    // One shared answer — `Program::repeater_rows` — so the resolver, the type checker and the validator
    // cannot disagree about what a row is. They each had their own copy, and only the resolver's knew about
    // `of=`, which would have meant a row field resolving in one pass and not the next.
    let fields = program.repeater_fields(el);
    if fields.is_empty() { None } else { Some(fields) }
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

/// Every name the document's `js` blocks declare, at any depth in the tree.
///
/// One helper, used by this pass and by `types`, so the two cannot disagree about what is in scope: a
/// name one accepts and the other rejects is a document that fails on a rule nobody can find.
pub fn js_declarations(program: &Program) -> Vec<String> {
    fn walk(els: &[Element], out: &mut Vec<String>) {
        for el in els {
            out.extend(el.escape_declares());
            walk(&el.children, out);
        }
    }
    let mut out = Vec::new();
    walk(&program.tree, &mut out);
    out
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
    fn a_field_is_named_from_the_state_it_binds() {
        // This used to be an **error**, on the reasoning that a field's first positional is the state
        // it binds rather than a label. True, and it drew the wrong conclusion: `input draft` binds a
        // state called `draft`, and `aria-label="draft"` is a name a screen reader can announce —
        // which is the entire point of demanding one. The compiler was holding a usable name and
        // refusing the document instead of using it.
        //
        // It was also the second most common reason model-generated GUML failed to compile
        // (`bench/gen`, unchanged between an 8B and a 70B model).
        //
        // A warning rather than silence: a state name is a variable name, usually a real word and
        // occasionally `x1`, and nothing here can tell which. The backends emit the derived name — see
        // `guml_codegen::derived_aria_label` — which is what makes the warning honest rather than a
        // claim about output that does not carry it.
        let d = check("page P\nstate draft=\"\"\n\ninput draft\n");
        assert!(!d.has_errors(), "a derivable name should not fail the build: {:?}", d.items);
        assert!(d.items.iter().any(|d| d.code == Code::InputWithoutLabel));
    }

    #[test]
    fn a_field_with_no_binding_to_derive_from_is_still_an_error() {
        // The rule is *derive*, not *invent*. With no positional there is nothing to name it after,
        // and a control a screen reader cannot announce is still a real fault.
        let d = check("page P\n\ninput\n");
        assert!(d.has_errors(), "a field with nothing at all must still be refused: {:?}", d.items);
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
