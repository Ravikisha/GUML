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
    /// **Stock Tailwind, and that is the whole reason it is the default.**
    ///
    /// The default used to be `shadcn`, which emits `bg-primary`, `text-foreground`, `border-border` —
    /// names that mean nothing to Tailwind on their own. They resolve only if the host has also
    /// installed shadcn's CSS variables and the `@theme inline` block that maps them. A user who ran
    /// `pnpm add tailwindcss`, compiled a document and opened it therefore got an **unstyled page**,
    /// with no error, because every class was real and none was defined.
    ///
    /// This theme uses literal utilities — `bg-white dark:bg-slate-900`, `border-slate-200` — which any
    /// Tailwind install resolves with no configuration at all. Nothing to install, nothing to import,
    /// no variables to define.
    ///
    /// `shadcn` is still shipped and is one word away (`"theme": "shadcn"` in `guml.json`, or
    /// `--theme shadcn`). The point is that it is a *choice* rather than an assumption about what the
    /// host already has.
    pub fn builtin() -> Self {
        Self::tailwind()
    }

    /// Stock Tailwind utilities on a literal palette. The default; see [`Theme::builtin`].
    pub fn tailwind() -> Self {
        let mut theme: Theme = serde_json::from_str(include_str!("../themes/tailwind.json"))
            .expect("the builtin theme is checked by a test");
        // Kept beside the rules rather than inside the JSON, so the stylesheet stays editable as CSS.
        theme.css = Some(include_str!("../themes/tailwind.css").to_string());
        theme
    }

    /// shadcn/ui: the same vocabulary expressed in shadcn's design tokens.
    ///
    /// Requires the host to define those tokens — `@guml/shadcn` ships them in `styles.css`, and any
    /// project already running shadcn has them. Opt in with `"theme": "shadcn"`.
    ///
    /// Kept, and kept tested, because it is the other half of the argument themes exist to make: it
    /// uses tokens (`bg-primary`) where the default uses literals (`bg-slate-900`), so the two together
    /// demonstrate that the *language* is unchanged by either choice — one document, both outputs.
    pub fn shadcn() -> Self {
        let mut theme: Theme = serde_json::from_str(include_str!("../themes/shadcn.json"))
            .expect("the shadcn theme is checked by a test");
        theme.css = Some(include_str!("../themes/shadcn.css").to_string());
        theme
    }

    /// Every theme compiled into the binary, so `--theme shadcn` needs no path.
    ///
    /// A name rather than a file is what makes "install the shadcn plugin" one word instead of a
    /// filesystem path into someone else's `node_modules`.
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "tailwind" => Some(Self::tailwind()),
            "shadcn" => Some(Self::shadcn()),
            _ => None,
        }
    }

    /// The names [`Theme::by_name`] accepts, for error messages and `--help`.
    pub fn builtin_names() -> &'static [&'static str] {
        &["tailwind", "shadcn"]
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

/// Tags whose emitted element receives the theme's focus treatment.
///
/// # Why this is derived from the element, not from the registry's `a11y.focusable`
///
/// Reading the registry flag looks like the obvious improvement over a hardcoded list, and it is wrong
/// here — the two answer different questions. `a11y.focusable` is a claim about the *component*: "this
/// is reachable and operable from the keyboard", which is true of `tabs`. The question this function
/// asks is about the *element being emitted right now*, and a `tabs` lowers to a `<div role="tablist">`
/// whose focusable parts are the buttons it generates. Driving the ring from the component flag put
/// `focus:ring-2` on the tablist container — a class that can never match, because the container never
/// takes focus.
///
/// So the basis is the HTML element, which is a fact with a definite answer, and shared via
/// [`crate::element_for`] so all backends agree. It reproduces the old hardcoded list exactly, without
/// being a list: adding a tag that lowers to `<button>` gets the focus treatment with no edit here.
///
/// `a11y.focusable` still earns its place — see
/// `every_component_claiming_to_be_focusable_lowers_to_something_focusable`, which is the check the
/// flag is actually good for.
fn is_focusable(tag: &str) -> bool {
    matches!(crate::element_for(tag), Some("a" | "button" | "input" | "select" | "textarea"))
}

/// Only form controls can be `disabled`; on an `<a>` the utility would be inert.
///
/// Still a list, because "is a form control" is a fact about the *HTML element a backend emits*, not
/// about the component's accessibility contract — `link` is focusable and is not disableable, so the
/// registry has no field that answers this.
fn is_form_control(tag: &str) -> bool {
    matches!(tag, "btn" | "input" | "select" | "check" | "toggle" | "progress")
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
        // Spot-check a layout primitive, a token-driven radius, and the two the *contract* names — read from
        // the contract rather than written out, so this cannot pin one theme's spelling of them. It did:
        // `focus\:ring-2` and `disabled\:opacity-40` were slate's, and shadcn's are
        // `focus-visible:ring-[3px]` and `disabled:opacity-50`.
        for selector in [".flex", ".rounded-md"] {
            assert!(css.contains(selector), "stylesheet is missing `{selector}`");
        }
        for class in theme
            .contract
            .focus_visible
            .split_whitespace()
            .chain(theme.contract.disabled.split_whitespace())
        {
            let selector = format!(".{}", css_escape(class));
            assert!(
                css.contains(&selector),
                "the contract promises `{class}` and the stylesheet does not implement it"
            );
        }
    }

    /// A utility class as it appears in a CSS selector.
    ///
    /// Escape *everything* outside `[A-Za-z0-9_-]` rather than keeping a list of the characters seen so far.
    /// That list has now been wrong twice, and both times the same way: a correctly-written stylesheet was
    /// reported as incomplete, which pushes a theme author towards a *wrong* stylesheet rather than a right
    /// one. First `.` was missing, so `py-0.5` failed. Then `[` and `]`, so every arbitrary-value utility
    /// failed — `ring-[3px]`, `h-[1.15rem]`, `rounded-[4px]`, which the shadcn theme uses throughout.
    ///
    /// A denylist grows one bug at a time. An allowlist of what needs no escape does not.
    fn css_escape(class: &str) -> String {
        class
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c.to_string()
                } else {
                    format!("\\{c}")
                }
            })
            .collect()
    }

    #[test]
    fn the_selector_escape_covers_every_utility_shape_a_theme_can_use() {
        // Pinned directly, because the failure mode is a *false* report and a false report is what makes
        // someone edit a file that was already correct.
        assert_eq!(css_escape("gap-2"), "gap-2");
        assert_eq!(css_escape("py-0.5"), r"py-0\.5");
        assert_eq!(css_escape("hover:bg-primary/90"), r"hover\:bg-primary\/90");
        assert_eq!(css_escape("ring-[3px]"), r"ring-\[3px\]");
        assert_eq!(css_escape("h-[1.15rem]"), r"h-\[1\.15rem\]");
        assert_eq!(css_escape("-translate-x-1/2"), r"-translate-x-1\/2");
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
            if !css.contains(&format!(".{}", css_escape(&class))) {
                missing.push(class);
            }
        }
        assert!(missing.is_empty(), "the stylesheet does not implement: {missing:?}");
    }

    #[test]
    fn every_shipped_theme_parses_and_satisfies_its_own_contract() {
        // `Theme::tailwind` and `Theme::shadcn` unwrap, so this test is what makes those unwraps
        // honest. Both are checked, not just the default: `--theme shadcn` is one word away, and a
        // theme that fails its own contract would only be discovered by whoever chose it.
        for name in Theme::builtin_names() {
            let theme = Theme::by_name(name).unwrap_or_else(|| panic!("`{name}` is advertised"));
            theme.validate().expect("a shipped theme must satisfy the contract it enforces");
            assert_eq!(
                &theme.name, name,
                "the file's `name` disagrees with the one it is loaded by"
            );
            assert!(theme.rules.len() > 20, "expected a rule per tag, got {}", theme.rules.len());
        }
    }

    #[test]
    fn the_default_is_stock_tailwind() {
        // The property, not the palette: the default must resolve under a bare `pnpm add tailwindcss`.
        // shadcn's tokens (`bg-primary`, `text-foreground`) are real class names that Tailwind does not
        // define, so a default emitting them renders unstyled — no error, every class spelled correctly,
        // nothing applied. That was the behaviour before this changed.
        let theme = Theme::builtin();
        assert_eq!(theme.name, "tailwind");

        let emitted: String =
            theme.rules.iter().map(|r| format!("{} ", r.classes())).collect::<String>();
        for token in ["bg-primary", "text-foreground", "bg-card", "border-border", "bg-background"]
        {
            assert!(
                !emitted.contains(token),
                "the default theme emits `{token}`, which stock Tailwind does not define — \
                 a host without shadcn's CSS variables would get an unstyled page"
            );
        }
    }

    #[test]
    fn a_group_picks_exactly_one_intent() {
        // `btn primary danger` must not concatenate two background colours.
        let theme = Theme::builtin();
        let classes = theme.classes("btn", &["primary", "danger"]);
        assert!(classes.contains("bg-slate-900"), "{classes}");
        assert!(!classes.contains("bg-red-600"), "two intents were applied: {classes}");
        // And the same holds for the theme that is *not* the default, which is the point of keeping it:
        // grouping is a property of the mechanism, not of one palette.
        let shadcn = Theme::shadcn().classes("btn", &["primary", "danger"]);
        assert!(shadcn.contains("bg-primary"), "{shadcn}");
        assert!(!shadcn.contains("bg-destructive"), "two intents were applied: {shadcn}");
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
    fn every_component_claiming_to_be_focusable_lowers_to_something_focusable() {
        // What `a11y.focusable` is good for. The flag is a *promise* — "reachable and operable from the
        // keyboard" — and this is the only thing that can catch a promise the lowering does not keep.
        //
        // Two ways to keep it: lower to an element that natively takes focus, or generate your own
        // focusable children. `tabs` is the second kind, which is exactly why the theme's focus ring is
        // not driven by this flag (see `is_focusable`).
        const GENERATES_ITS_OWN_CONTROLS: &[&str] = &["tabs"];
        for name in crate::registry().names() {
            let def = crate::registry().get(name).unwrap();
            if !def.a11y.focusable || GENERATES_ITS_OWN_CONTROLS.contains(&name) {
                continue;
            }
            assert!(
                is_focusable(name),
                "`{name}` declares `a11y.focusable` but lowers to {:?}, which does not take focus — \
                 either the claim is wrong or the lowering is",
                crate::element_for(name)
            );
        }
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

#[cfg(test)]
mod hardcoded_colours {
    /// A ratchet on colour literals compiled *into* the backends.
    ///
    /// # The violation this measures
    ///
    /// This module's own docs say it: "a colour literal inside a compiler is a theme nobody can override".
    /// And yet the backends contain 60-odd of them — the error banner is `bg-red-50 text-red-700`, the
    /// loading skeleton is `bg-slate-100`, a `tier` card is `border-slate-200`. Those bypass the theme
    /// entirely, so a host that loads its own palette gets it applied to most of the page and not to those
    /// parts.
    ///
    /// It was invisible for as long as the default theme was `slate`, because the literals *matched* it.
    /// Making shadcn the default is what exposed it: a token-driven page with `bg-red-50` in the middle of
    /// it. So the defect is pre-existing and the change only made it visible, which is the useful kind of
    /// visible.
    ///
    /// # Why a ratchet rather than a fix here
    ///
    /// Routing them through the theme means ~14 pseudo-tag roles across five backends, and doing half of it
    /// would leave the backends disagreeing about the same document — which invariant 8 forbids and which is
    /// worse than the current state, where they are at least consistently wrong. It is tracked in
    /// `ROADMAP.md`.
    ///
    /// Meanwhile this stops the number growing. A count nobody enforces is how 65 of these accumulated.
    #[test]
    fn no_new_colour_literal_enters_a_backend() {
        // Per file, so the message names where to look rather than only that the total moved.
        // The true counts once comments are excluded: 58 in total, not the 65 a naive scan reported. Each is
        // the current number exactly, so the budget only ever ratchets down.
        const BUDGET: &[(&str, &str, usize)] = &[
            ("react", include_str!("react.rs"), 22),
            ("wc", include_str!("wc.rs"), 17),
            ("html", include_str!("html.rs"), 6),
            ("svelte", include_str!("svelte.rs"), 10),
            ("lib", include_str!("lib.rs"), 3),
            ("json", include_str!("json.rs"), 0),
        ];

        // Tailwind's palette shape: `<utility>-<hue>-<shade>`. Deliberately narrow — it looks for a *named
        // hue with a numeric shade*, which is what a literal is, and does not match a token like
        // `bg-primary` or a structural utility like `gap-2`.
        let hues = [
            "red", "orange", "amber", "yellow", "lime", "green", "emerald", "teal", "cyan", "sky",
            "blue", "indigo", "violet", "purple", "fuchsia", "pink", "rose", "slate", "gray",
            "zinc", "neutral", "stone",
        ];
        let mut over = Vec::new();
        for (name, source, budget) in BUDGET {
            // Comment lines are excluded. A colour literal *described* in a doc comment is not one in the
            // output, and counting it made the budget unfalsifiable in the wrong direction: removing four
            // literals from `<body>` and explaining why in a comment left the count unchanged, so the ratchet
            // reported no progress on a real improvement. Line-start only, so a `https://` inside a string is
            // not mistaken for a comment.
            let code: String = source
                .lines()
                .filter(|l| {
                    let t = l.trim_start();
                    !(t.starts_with("//") || t.starts_with("* ") || t == "*")
                })
                .collect::<Vec<_>>()
                .join(
                    "
",
                );
            let found = code
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                .filter(|word| {
                    let Some((prefix, rest)) = word.rsplit_once('-') else { return false };
                    // `bg-slate-900` → prefix `bg-slate`, rest `900`.
                    rest.chars().all(|c| c.is_ascii_digit())
                        && !rest.is_empty()
                        && hues.iter().any(|h| prefix.ends_with(&format!("-{h}")))
                })
                .count();
            if found > *budget {
                over.push(format!("{name}.rs: {found} colour literals, budget {budget}"));
            }
        }
        assert!(
            over.is_empty(),
            "a colour literal was added to a backend instead of to a theme — the theme cannot override it, \
             so a host loading its own palette will not get it applied there.\n{}",
            over.join("\n")
        );
    }
}
