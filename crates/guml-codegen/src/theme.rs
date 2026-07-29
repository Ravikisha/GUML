//! Themes: the mapping from semantic vocabulary to presentation, as data.
//!
//! # Why this is not a table inside a backend
//!
//! GUML's contract is Markdown's: the document carries meaning, the host carries appearance. A
//! `.guml` file has no classes in it, which is both the token lever and the correctness guarantee.
//! But until this module existed, the *other* half of that contract was missing — the appearance was
//! a `match` statement compiled into the binary, so "the compiler owns presentation" meant "we own
//! presentation, and you cannot have it". A colour literal inside a compiler is a theme nobody can
//! override.
//!
//! A theme is therefore data: `(tag, [modifiers]) -> class string`. Load one and every document
//! compiled with it is re-themed, which is what turns a design system from a request in a prompt
//! into a guarantee.
//!
//! # The accessibility catch
//!
//! Handing the class table to a host trades one of the project's two claims for the other: the token
//! saving is unaffected (the source is identical either way), but the *correctness* guarantee is not
//! — a theme can specify unreadable colour pairs or remove focus rings, and the compiler would emit
//! it obediently. So a theme declares its contrast and focus commitments in `Contract`, and
//! `Theme::validate` refuses a theme that omits them. That check is the reason a themeable compiler
//! can still promise accessible output.
//!
//! # Shape
//!
//! ```json
//! {
//!   "name": "my-brand",
//!   "contract": { "focus_visible": "focus:ring-2 focus:ring-brand-600", "min_contrast": 4.5 },
//!   "rules": [
//!     { "tag": "btn", "base": "rounded-md px-4 py-2 text-sm font-medium" },
//!     { "tag": "btn", "when": ["primary"], "add": "bg-brand-600 text-white" }
//!   ]
//! }
//! ```
//!
//! Rules apply in order: every rule whose `tag` matches and whose `when` modifiers are all present
//! contributes its classes. `else_after` marks a rule as a fallback that applies only when no earlier
//! rule in the same `group` matched, which is how `btn` picks exactly one intent.

use serde::{Deserialize, Serialize};

/// What a theme promises about the output it produces.
///
/// Required, because a theme that says nothing about focus or contrast is a theme that can silently
/// produce inaccessible pages — and the compiler's accessibility guarantee is a hard error elsewhere,
/// so it cannot become advisory here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contract {
    /// Classes applied to every focusable control, so keyboard focus is always visible.
    pub focus_visible: String,
    /// The contrast ratio this theme's text/background pairs meet. WCAG AA is 4.5 for body text.
    pub min_contrast: f32,
    /// Classes applied to a disabled control, so "disabled" is not conveyed by colour alone.
    #[serde(default)]
    pub disabled: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub tag: String,
    /// Modifiers that must all be present for this rule to apply.
    #[serde(default)]
    pub when: Vec<String>,
    /// Classes contributed when the rule matches.
    #[serde(default)]
    pub add: String,
    /// Alias for `add`, read better on the rule that establishes a tag's baseline.
    #[serde(default)]
    pub base: String,
    /// Rules sharing a group are mutually exclusive: the first match wins, and a rule with an empty
    /// `when` acts as that group's fallback. This is what makes `btn` pick one intent rather than
    /// concatenating `primary` and `danger`.
    #[serde(default)]
    pub group: Option<String>,
}

impl Rule {
    fn classes(&self) -> &str {
        if self.base.is_empty() { &self.add } else { &self.base }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub contract: Contract,
    pub rules: Vec<Rule>,
    /// A stylesheet implementing this theme's classes, inlined by the static HTML backend.
    ///
    /// Required for a themed no-JavaScript document to be styled at all: that backend has no build
    /// step, so it cannot run a utility-class compiler. A theme that omits this still works for the
    /// React backend, where the host's own pipeline processes the classes.
    #[serde(default)]
    pub css: Option<String>,
}

#[derive(Debug)]
pub enum ThemeError {
    Parse(serde_json::Error),
    /// The theme does not state a focus treatment, or states a contrast below WCAG AA.
    WeakContract(String),
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeError::Parse(e) => write!(f, "theme is not valid JSON: {e}"),
            ThemeError::WeakContract(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for ThemeError {}

impl Theme {
    /// The theme the compiler ships: Tailwind utilities on a slate palette.
    ///
    /// Data rather than code, so it is exactly as replaceable as any other theme — and so the builtin
    /// output stays byte-identical to what it was before themes existed, which is what let this land
    /// without changing a single emitted fixture.
    pub fn builtin() -> Self {
        let mut theme: Theme = serde_json::from_str(include_str!("../themes/slate.json"))
            .expect("the builtin theme is checked by a test");
        // Kept beside the rules rather than inside the JSON, so the stylesheet stays editable as CSS.
        theme.css = Some(include_str!("../themes/slate.css").to_string());
        theme
    }

    pub fn from_json(json: &str) -> Result<Self, ThemeError> {
        let theme: Theme = serde_json::from_str(json).map_err(ThemeError::Parse)?;
        theme.validate()?;
        Ok(theme)
    }

    /// Refuse a theme that would make the compiler's accessibility promise untrue.
    pub fn validate(&self) -> Result<(), ThemeError> {
        if self.contract.focus_visible.trim().is_empty() {
            return Err(ThemeError::WeakContract(format!(
                "theme `{}` declares no `contract.focus_visible`; a control nobody can see focus on \
                 is unusable from a keyboard, and the compiler cannot supply one it does not know",
                self.name
            )));
        }
        if self.contract.min_contrast < 4.5 {
            return Err(ThemeError::WeakContract(format!(
                "theme `{}` declares `min_contrast` {}, below the WCAG AA floor of 4.5 for body text",
                self.name, self.contract.min_contrast
            )));
        }
        Ok(())
    }

    /// Classes for a tag with a set of modifiers.
    pub fn classes(&self, tag: &str, mods: &[&str]) -> String {
        let mut out: Vec<&str> = Vec::new();
        let mut matched_groups: Vec<&str> = Vec::new();

        // Tag rules first, then wildcard rules, so a `*` rule lands at the end of the class list —
        // which is where the old hardcoded table put the one rule that had this shape (`full`).
        let ordered = self
            .rules
            .iter()
            .filter(|r| r.tag == tag)
            .chain(self.rules.iter().filter(|r| r.tag == "*"));
        for rule in ordered {
            let applies = rule.when.iter().all(|m| mods.iter().any(|got| got == m));
            if !applies {
                continue;
            }
            if let Some(group) = &rule.group {
                // Mutually exclusive: the first matching rule in a group wins. A fallback rule has an
                // empty `when`, so it matches trivially and is placed last in the theme.
                if matched_groups.contains(&group.as_str()) {
                    continue;
                }
                matched_groups.push(group);
            }
            let classes = rule.classes();
            if !classes.is_empty() {
                out.push(classes);
            }
        }

        // The contract is appended by the compiler, not by the theme's rules, so a theme cannot
        // forget it on one control. This is the mechanism behind `validate`: the promise is applied,
        // not merely declared.
        if is_focusable(tag) && !self.contract.focus_visible.is_empty() {
            out.push(&self.contract.focus_visible);
        }
        // Only form controls can be disabled; on an `<a>` the utility would be inert.
        if is_form_control(tag) && !self.contract.disabled.is_empty() {
            out.push(&self.contract.disabled);
        }

        out.join(" ")
    }
}

/// Tags that receive the theme's focus and disabled treatment.
///
/// Deliberately not read from the registry: `guml-codegen` must not depend on it (there is a crate
/// cycle through the driver), and this is a small, stable list of the interactive HTML elements the
/// backends emit.
fn is_focusable(tag: &str) -> bool {
    matches!(tag, "btn" | "link" | "input" | "select" | "check" | "toggle")
}

fn is_form_control(tag: &str) -> bool {
    matches!(tag, "btn" | "input" | "select" | "check" | "toggle")
}

use std::sync::OnceLock;
/// The theme this process compiles with.
///
/// A process-wide value rather than a parameter on `Options`, for one reason: `classes` is called from
/// twenty-five places across two backends, several of them deep inside recursive renderers that would
/// each have to grow a `&Theme` argument. A single override point keeps the change to the theme system
/// rather than to every emitter.
///
/// `set` is therefore write-once per process — a compiler run has one theme, and silently switching
/// mid-run would produce a document styled two ways.
static ACTIVE: OnceLock<Theme> = OnceLock::new();

pub fn active() -> &'static Theme {
    ACTIVE.get_or_init(Theme::builtin)
}

/// Install a theme for this process. Returns `Err` with the loaded theme if one is already active.
pub fn set(theme: Theme) -> Result<(), Box<Theme>> {
    ACTIVE.set(theme).map_err(Box::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_theme_ships_a_stylesheet_for_its_own_classes() {
        // Without this, `--backend html` emits a document whose classes mean nothing: there is no
        // build step in that path to turn utilities into CSS.
        let theme = Theme::builtin();
        let css = theme.css.as_deref().expect("the shipped theme must carry its stylesheet");
        // Spot-check the two the contract depends on, plus a layout primitive.
        for selector in [".flex", ".rounded-md", r"focus\:ring-2", r"disabled\:opacity-40"] {
            assert!(css.contains(selector), "stylesheet is missing `{selector}`");
        }
    }

    #[test]
    fn every_class_the_builtin_theme_emits_has_a_rule_in_its_stylesheet() {
        // The failure this catches is silent and visual: a rule gains a utility, the stylesheet does
        // not, and the no-JavaScript output is subtly unstyled with nothing to indicate it.
        let theme = Theme::builtin();
        let css = theme.css.clone().unwrap();
        let mut missing = Vec::new();
        let mut classes: Vec<String> = Vec::new();
        for rule in &theme.rules {
            classes.extend(rule.classes().split_whitespace().map(str::to_string));
        }
        classes.extend(theme.contract.focus_visible.split_whitespace().map(str::to_string));
        classes.extend(theme.contract.disabled.split_whitespace().map(str::to_string));
        for class in classes {
            // CSS escapes `:` and `/` in selectors.
            let selector = class.replace(':', r"\:").replace('/', r"\/");
            if !css.contains(&format!(".{selector}")) {
                missing.push(class);
            }
        }
        assert!(missing.is_empty(), "the stylesheet does not implement: {missing:?}");
    }

    #[test]
    fn the_builtin_theme_parses_and_satisfies_its_own_contract() {
        // `Theme::builtin` unwraps, so this test is what makes that unwrap honest.
        let theme = Theme::builtin();
        theme.validate().expect("the shipped theme must satisfy the contract it enforces");
        assert_eq!(theme.name, "slate");
        assert!(theme.rules.len() > 20, "expected a rule per tag, got {}", theme.rules.len());
    }

    #[test]
    fn a_group_picks_exactly_one_intent() {
        // `btn primary danger` must not concatenate two background colours.
        let theme = Theme::builtin();
        let classes = theme.classes("btn", &["primary", "danger"]);
        assert!(classes.contains("bg-slate-900"), "{classes}");
        assert!(!classes.contains("bg-red-600"), "two intents were applied: {classes}");
    }

    #[test]
    fn an_unstyled_tag_yields_nothing_rather_than_a_default() {
        assert_eq!(Theme::builtin().classes("nosuchtag", &[]), "");
    }

    #[test]
    fn the_focus_contract_is_applied_by_the_compiler_not_by_the_rules() {
        // A theme cannot forget the focus ring on one control, because it is appended per focusable
        // tag rather than written into each rule.
        let theme = Theme::builtin();
        for tag in ["btn", "input", "check", "link"] {
            assert!(
                theme.classes(tag, &[]).contains(&theme.contract.focus_visible),
                "`{tag}` lost the focus treatment"
            );
        }
        // A container is not focusable, so it does not get one.
        assert!(!theme.classes("card", &[]).contains("focus:"));
    }

    #[test]
    fn a_theme_without_a_focus_treatment_is_refused() {
        let json =
            r#"{"name":"bad","contract":{"focus_visible":"","min_contrast":7.0},"rules":[]}"#;
        assert!(matches!(Theme::from_json(json), Err(ThemeError::WeakContract(_))));
    }

    #[test]
    fn a_theme_below_wcag_aa_is_refused() {
        // The trade this guards: handing the class table to a host must not quietly hand over the
        // accessibility guarantee too.
        let json =
            r#"{"name":"bad","contract":{"focus_visible":"ring","min_contrast":3.0},"rules":[]}"#;
        match Theme::from_json(json) {
            Err(ThemeError::WeakContract(why)) => assert!(why.contains("4.5"), "{why}"),
            other => panic!("expected a contract rejection, got {other:?}"),
        }
    }

    #[test]
    fn a_custom_theme_rethemes_every_document() {
        // The claim the report makes about the design-system table, now actually true from outside.
        let json = r#"{
          "name": "brand",
          "contract": { "focus_visible": "focus:ring-2 focus:ring-brand-600", "min_contrast": 4.6 },
          "rules": [
            { "tag": "btn", "base": "rounded-full px-5 py-2" },
            { "tag": "btn", "when": ["primary"], "add": "bg-brand-600 text-white", "group": "intent" },
            { "tag": "btn", "add": "bg-brand-100 text-brand-900", "group": "intent" }
          ]
        }"#;
        let theme = Theme::from_json(json).expect("loads");
        let primary = theme.classes("btn", &["primary"]);
        assert!(primary.contains("rounded-full"), "{primary}");
        assert!(primary.contains("bg-brand-600"), "{primary}");
        assert!(
            primary.contains("focus:ring-brand-600"),
            "the contract was not applied: {primary}"
        );
        // The fallback rule covers a button with no intent.
        assert!(theme.classes("btn", &[]).contains("bg-brand-100"));
    }
}
