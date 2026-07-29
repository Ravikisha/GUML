//! The GUML component registry.
//!
//! The registry is the *closed, validated tag vocabulary* — the design idea inherited from
//! Markdoc and from A2UI's host-approved component catalog (report §2.1, §2.5). Two
//! consequences that matter more than they look:
//!
//! 1. **An unknown tag is a compile error, not a runtime surprise.** That is most of the
//!    hallucination-resistance claim (report §7.2, H3).
//! 2. **`TagKind` disambiguates prose from structure.** The lexer cannot know whether the
//!    remainder of `p Press the button` is a label plus modifiers or free prose; the registry
//!    decides. See `guml_syntax` module docs.
//!
//! The registry is also the retrieval unit: for a 300-component design system, only the
//! entries a prompt actually needs get loaded into context (report §5.8), so vocabulary size
//! does not linearly inflate the prompt.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    /// Holds children; no prose of its own (`card`, `row`, `section`).
    Container,
    /// Remainder of the line is prose (`p`, `h`, `text`).
    Text,
    /// Interactive leaf (`btn`, `link`, `check`).
    Control,
    /// Bound form field (`input`, `select`).
    Field,
    /// Iterates a resource; its children are the item template (`list`, `table`).
    Repeater,
}

/// Which conformance level a tag belongs to.
///
/// GUML is one language with two levels, in the way CommonMark and GFM are one language with two
/// levels. The split is not cosmetic — it is what makes the language safe to embed:
///
/// * **Core** is markup. No I/O, no state, no behaviour. A host can render a Core document that
///   arrived from an untrusted agent, because there is nothing in it to run.
/// * **App** is the framework layer: `data` resources, actions, mutations, state. Useful, and
///   categorically different — a document that declares network calls is not something a host can
///   render blindly.
///
/// Deciding this after adoption would be impossible, because every document written in the meantime
/// would have to be re-classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Markup only: safe to render from an untrusted source.
    Core,
    /// Needs a runtime, a network, or both.
    App,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Core => "core",
            Level::App => "app",
        }
    }
}

/// The accessibility contract a component must satisfy, stated as data.
///
/// This used to be one `requires_label` flag in the registry with the rest of the behaviour spread
/// through codegen. That is fine while the compiler owns every component and impossible once a host
/// can load its own: a third-party entry has to be able to *declare* what the compiler must
/// guarantee, or the accessibility promise stops at the builtin vocabulary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct A11y {
    /// An accessible name is mandatory when there is no text label. Drives `GUML0050`/`GUML0051`,
    /// which are hard errors by design (report §6.4).
    pub requires_label: bool,
    /// ARIA role the compiler must emit, when the chosen HTML element does not imply it.
    pub role: Option<String>,
    /// Must be reachable and operable from the keyboard.
    pub focusable: bool,
    /// Announces a state that assistive technology has to hear change (`aria-pressed`,
    /// `aria-checked`).
    pub announces_state: bool,
}

/// One entry in the tag vocabulary.
///
/// Owned rather than `&'static`, which is the change that makes the registry *loadable*. While this
/// type borrowed from static memory, every new component was a recompile of the compiler — a
/// requirement no markup language can impose on the applications that embed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentDef {
    pub name: String,
    pub kind: TagKind,
    /// Conformance level. Defaults to `core`, so a hand-written registry entry is safe unless it
    /// says otherwise.
    #[serde(default = "default_level")]
    pub level: Level,
    /// Attribute names accepted beyond the global set. Unknown attrs are diagnosed.
    #[serde(default)]
    pub attrs: Vec<String>,
    #[serde(default)]
    pub a11y: A11y,
    /// One-line description. This text is what goes into the LLM's context, so it is written
    /// for a model, not for a docs site: terse, and states *when* to use the tag.
    pub doc: String,
}

fn default_level() -> Level {
    Level::Core
}

impl ComponentDef {
    /// Kept as a method because it reads better at the call sites in `sema`, and because the flag
    /// now lives one level down in `a11y`.
    pub fn requires_label(&self) -> bool {
        self.a11y.requires_label
    }
}

/// The authored builtin table. `&'static` so the compiled-in vocabulary costs no allocation until
/// a `Registry` is built, and so the table stays readable as a const array.
struct Builtin {
    name: &'static str,
    kind: TagKind,
    level: Level,
    attrs: &'static [&'static str],
    requires_label: bool,
    role: Option<&'static str>,
    focusable: bool,
    announces_state: bool,
    doc: &'static str,
}

impl Builtin {
    fn to_def(&self) -> ComponentDef {
        ComponentDef {
            name: self.name.to_string(),
            kind: self.kind,
            level: self.level,
            attrs: self.attrs.iter().map(|a| a.to_string()).collect(),
            a11y: A11y {
                requires_label: self.requires_label,
                role: self.role.map(str::to_string),
                focusable: self.focusable,
                announces_state: self.announces_state,
            },
            doc: self.doc.to_string(),
        }
    }
}

/// Modifiers are semantic, never utility classes. Replacing Tailwind class strings with these
/// is the single largest token lever measured in the report (§1.5): class attributes were
/// roughly a third of React's tokens in the landing-page fixture.
pub const MODIFIERS: &[&str] = &[
    // intent
    "primary",
    "secondary",
    "outline",
    "ghost",
    "quiet",
    "danger",
    "featured",
    // size
    "xs",
    "sm",
    "md",
    "lg",
    "xl",
    // layout
    "center",
    "start",
    "end",
    "between",
    "wrap",
    "tight",
    "loose",
    "full",
    // state
    "disabled",
    "loading",
    "readonly",
    "required",
];

/// Attributes accepted on every tag.
///
/// `disabled`/`loading`/`readonly`/`required` appear here *and* in [`MODIFIERS`] on purpose:
/// as a bare word they are a static state (`btn Save disabled`), and as an attribute they take
/// a binding (`btn Save disabled={!draft}`). The parser decides by looking for `=`.
/// Attributes every tag accepts.
///
/// `class` is deliberately absent. It parsed for a while and was then silently discarded by codegen —
/// accepted with no diagnostic, dropped with no trace, which is the one thing invariant 3 forbids.
/// Rejecting it is the right answer rather than honouring it: presentation is the *theme's* to decide
/// (see `guml_codegen::theme`), and a per-element class list in the source would put it back in the
/// document, undoing both the token saving and the guarantee. `raw` covers the genuine exception.
pub const GLOBAL_ATTRS: &[&str] = &[
    "id", "aria", "title", "hidden", "cols", "gap", "w", "if", "disabled", "loading", "readonly",
    "required",
];

/// Tags whose indented children are *content lines*, not nested elements: `tier` perks and
/// `faq` question/answer pairs. Without this, every perk would need its own tag, which is
/// exactly the per-line overhead GUML exists to remove.
pub const TEXT_CHILD_TAGS: &[&str] = &["tier", "faq"];

const COMPONENTS: &[Builtin] = &[
    // ---- containers ----
    Builtin {
        name: "card",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Bordered surface grouping related content. Optional title as first positional.",
    },
    Builtin {
        name: "row",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Horizontal layout. Use `center`/`between` to align instead of flex utilities.",
    },
    Builtin {
        name: "col",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Vertical layout.",
    },
    Builtin {
        name: "section",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Page section. Takes an optional #anchor and a title positional.",
    },
    Builtin {
        name: "nav",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Top navigation bar. First positional is the brand name.",
    },
    Builtin {
        name: "hero",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Above-the-fold block: headline, subcopy, calls to action.",
    },
    Builtin {
        name: "footer",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Page footer. First positional is the copyright line.",
    },
    Builtin {
        name: "form",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Submits on enter and on its primary button; its action runs on submit.",
    },
    Builtin {
        name: "tier",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &["cta"],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Pricing tier. Positionals: name, price, blurb. Children are perk lines.",
    },
    Builtin {
        name: "faq",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &["open"],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Accordion. Each child line is `question | answer`.",
    },
    Builtin {
        name: "tabs",
        level: Level::Core,
        kind: TagKind::Container,
        attrs: &[],
        role: Some("tablist"),
        focusable: true,
        announces_state: true,
        requires_label: false,
        doc: "Segmented control bound to an enumerated state; options come from its domain.",
    },
    // ---- text ----
    Builtin {
        name: "h",
        level: Level::Core,
        kind: TagKind::Text,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Section heading.",
    },
    Builtin {
        name: "h1",
        level: Level::Core,
        kind: TagKind::Text,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Page headline. One per page.",
    },
    Builtin {
        name: "h2",
        level: Level::Core,
        kind: TagKind::Text,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Subheading.",
    },
    Builtin {
        name: "p",
        level: Level::Core,
        kind: TagKind::Text,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Paragraph of prose.",
    },
    Builtin {
        name: "text",
        level: Level::Core,
        kind: TagKind::Text,
        attrs: &["strike"],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Inline text, usually a binding inside a repeater.",
    },
    Builtin {
        name: "metric",
        level: Level::Core,
        kind: TagKind::Text,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Large single number, for counters and KPI tiles.",
    },
    Builtin {
        name: "head",
        level: Level::Core,
        kind: TagKind::Text,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Page header line; may include a binding for a live count.",
    },
    Builtin {
        name: "empty",
        level: Level::Core,
        kind: TagKind::Text,
        attrs: &[],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Empty-state message for the enclosing repeater.",
    },
    // ---- controls ----
    Builtin {
        name: "btn",
        level: Level::Core,
        kind: TagKind::Control,
        attrs: &["busy", "type"],
        role: None,
        focusable: true,
        announces_state: false,
        requires_label: true,
        doc: "Button. First positional is the label; `>` gives the action.",
    },
    Builtin {
        name: "link",
        level: Level::Core,
        kind: TagKind::Control,
        attrs: &[],
        role: None,
        focusable: true,
        announces_state: false,
        requires_label: true,
        doc: "Navigational link to a /route or #anchor.",
    },
    Builtin {
        name: "check",
        level: Level::Core,
        kind: TagKind::Control,
        attrs: &[],
        role: None,
        focusable: true,
        announces_state: true,
        requires_label: true,
        doc: "Checkbox bound to a boolean; its action runs on change.",
    },
    Builtin {
        name: "toggle",
        level: Level::Core,
        kind: TagKind::Control,
        attrs: &[],
        role: Some("switch"),
        focusable: true,
        announces_state: true,
        requires_label: true,
        doc: "On/off switch bound to a boolean.",
    },
    // ---- fields ----
    Builtin {
        name: "input",
        level: Level::Core,
        kind: TagKind::Field,
        attrs: &["placeholder", "kind", "min", "max"],
        role: None,
        focusable: true,
        announces_state: false,
        requires_label: true,
        doc: "Text field bound to a state name given as the first positional.",
    },
    Builtin {
        name: "select",
        level: Level::Core,
        kind: TagKind::Field,
        attrs: &["placeholder"],
        role: None,
        focusable: true,
        announces_state: false,
        requires_label: true,
        doc: "Dropdown bound to an enumerated state.",
    },
    // ---- repeaters ----
    Builtin {
        name: "list",
        level: Level::App,
        kind: TagKind::Repeater,
        attrs: &["where", "sort", "of"],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Renders one child template per item of a resource. Loading, empty and error states are compiled in.",
    },
    Builtin {
        name: "table",
        level: Level::App,
        kind: TagKind::Repeater,
        attrs: &["where", "sort", "of"],
        role: None,
        focusable: false,
        announces_state: false,
        requires_label: false,
        doc: "Tabular repeater; children describe one row.",
    },
];

#[derive(Debug, Clone)]
pub struct Registry {
    by_name: BTreeMap<String, ComponentDef>,
    /// Highest conformance level this registry admits. `Core` is the safety mode: an `App`-level
    /// tag is simply not in the vocabulary, so it fails as an unknown tag with a message saying why.
    level: Level,
}

/// Why a registry document was rejected.
#[derive(Debug)]
pub enum RegistryError {
    Parse(serde_json::Error),
    /// A user entry would shadow a builtin. Rejected rather than merged: silently replacing `btn`
    /// would mean two documents using the same tag render differently with no diagnostic, which is
    /// the failure mode a closed vocabulary exists to prevent.
    Shadows(String),
    /// A name that cannot be written in GUML, so no document could ever reference it.
    BadName(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Parse(e) => write!(f, "registry is not valid JSON: {e}"),
            RegistryError::Shadows(n) => {
                write!(f, "`{n}` is a builtin tag; a registry may add tags but not redefine them")
            }
            RegistryError::BadName(n) => write!(
                f,
                "`{n}` is not a usable tag name: use lowercase letters, digits and `-`, starting with a letter"
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

impl Default for Registry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl Registry {
    pub fn builtin() -> Self {
        let mut by_name = BTreeMap::new();
        for c in COMPONENTS {
            by_name.insert(c.name.to_string(), c.to_def());
        }
        Self { by_name, level: Level::App }
    }

    /// The Core vocabulary only: markup, no I/O, no state.
    ///
    /// This is the switch a host embedding untrusted documents needs. It is enforced by *absence* —
    /// an `App` tag is not in the map, so it fails as an unknown tag — rather than by a check
    /// somewhere downstream that could be forgotten.
    pub fn core() -> Self {
        let mut r = Self::builtin();
        r.by_name.retain(|_, c| c.level == Level::Core);
        r.level = Level::Core;
        r
    }

    pub fn level(&self) -> Level {
        self.level
    }

    /// Load extra components from a JSON registry document.
    ///
    /// The shape is `{"components": [ … ]}`, matching `ComponentDef`. Builtins always win: a user
    /// entry that shadows one is an error, not an override.
    pub fn from_json(json: &str) -> Result<Self, RegistryError> {
        Self::builtin().extend_from_json(json)
    }

    pub fn extend_from_json(mut self, json: &str) -> Result<Self, RegistryError> {
        // Both shapes are accepted: `{"components": [ … ]}`, and a bare `[ … ]` array, which is how
        // someone writing a registry by hand tends to start. `untagged` picks whichever matches.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Doc {
            Wrapped { components: Vec<ComponentDef> },
            Bare(Vec<ComponentDef>),
        }
        let components = match serde_json::from_str::<Doc>(json).map_err(RegistryError::Parse)? {
            Doc::Wrapped { components } => components,
            Doc::Bare(components) => components,
        };
        let builtin: std::collections::BTreeSet<&str> = COMPONENTS.iter().map(|c| c.name).collect();

        for def in components {
            if builtin.contains(def.name.as_str()) {
                return Err(RegistryError::Shadows(def.name));
            }
            if !is_usable_tag_name(&def.name) {
                return Err(RegistryError::BadName(def.name));
            }
            // A registry loaded into a Core host may not smuggle in an app-level component.
            if self.level == Level::Core && def.level == Level::App {
                continue;
            }
            self.by_name.insert(def.name.clone(), def);
        }
        Ok(self)
    }

    /// Serialise this registry, so a host can publish the vocabulary it accepts.
    pub fn to_json(&self) -> String {
        let components: Vec<&ComponentDef> = self.by_name.values().collect();
        serde_json::json!({ "components": components }).to_string()
    }

    pub fn get(&self, tag: &str) -> Option<&ComponentDef> {
        self.by_name.get(tag)
    }

    pub fn kind(&self, tag: &str) -> Option<TagKind> {
        self.get(tag).map(|c| c.kind)
    }

    pub fn is_modifier(&self, word: &str) -> bool {
        MODIFIERS.contains(&word)
    }

    pub fn children_are_text(&self, tag: &str) -> bool {
        TEXT_CHILD_TAGS.contains(&tag)
    }

    pub fn accepts_attr(&self, tag: &str, attr: &str) -> bool {
        if GLOBAL_ATTRS.contains(&attr) {
            return true;
        }
        self.get(tag).map(|c| c.attrs.iter().any(|a| a == attr)).unwrap_or(false)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> + '_ {
        self.by_name.keys().map(String::as_str)
    }

    /// Nearest known tag within a small edit distance — the `did you mean` suggestion that
    /// lets the repair loop fix a typo without another model call.
    pub fn suggest(&self, unknown: &str) -> Option<&str> {
        let max = if unknown.len() <= 4 { 1 } else { 2 };
        self.by_name
            .keys()
            .map(|n| (n.as_str(), edit_distance(unknown, n)))
            .filter(|(_, d)| *d <= max)
            .min_by_key(|(_, d)| *d)
            .map(|(n, _)| n)
    }

    /// Strict variant (distance 1 only). Used where a false positive would be noisy — e.g.
    /// deciding whether a bare lowercase word is a mistyped modifier or just a label.
    pub fn suggest_modifier_close(unknown: &str) -> Option<&'static str> {
        MODIFIERS
            .iter()
            .map(|m| (*m, edit_distance(unknown, m)))
            .filter(|(_, d)| *d == 1)
            .min_by_key(|(_, d)| *d)
            .map(|(m, _)| m)
    }

    pub fn suggest_modifier(unknown: &str) -> Option<&'static str> {
        let max = if unknown.len() <= 4 { 1 } else { 2 };
        MODIFIERS
            .iter()
            .map(|m| (*m, edit_distance(unknown, m)))
            .filter(|(_, d)| *d <= max)
            .min_by_key(|(_, d)| *d)
            .map(|(m, _)| m)
    }

    /// Context block for the LLM prompt. Only the requested tags are emitted, which is what
    /// keeps prompt cost sublinear in vocabulary size.
    pub fn prompt_context(&self, tags: &[&str]) -> String {
        let mut out = String::new();
        for t in tags {
            if let Some(c) = self.get(t) {
                out.push_str(&format!("{} ({:?}) — {}\n", c.name, c.kind, c.doc));
            }
        }
        out
    }
}

/// A tag name a document could actually reference.
///
/// The lexer reads a tag as a bare word, so a name with a space or a `=` in it could be registered
/// and then never matched — the registry would accept an entry that is unusable by construction.
fn is_usable_tag_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Optimal string alignment (Damerau-Levenshtein restricted to adjacent transpositions).
///
/// Plain Levenshtein scores `crad` -> `card` as distance 2, which pushes the most common typo
/// class — a swapped pair — outside a tight threshold. Counting a transposition as one edit
/// keeps suggestions useful without loosening the threshold and inviting false positives.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }

    // Three rolling rows rather than a full matrix: `prev2` is only needed for the
    // transposition case, so the whole table never has to be allocated. This runs on every
    // unknown tag, inside the compiler's hot path.
    let mut prev2 = vec![0usize; m + 1];
    let mut prev1: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0usize; m + 1];

    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (prev1[j] + 1).min(cur[j - 1] + 1).min(prev1[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(prev2[j - 2] + 1);
            }
            cur[j] = best;
        }
        std::mem::swap(&mut prev2, &mut prev1);
        std::mem::swap(&mut prev1, &mut cur);
    }
    prev1[m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_tags_resolve() {
        let r = Registry::builtin();
        assert_eq!(r.kind("btn"), Some(TagKind::Control));
        assert_eq!(r.kind("p"), Some(TagKind::Text));
        assert_eq!(r.kind("list"), Some(TagKind::Repeater));
        assert_eq!(r.kind("nope"), None);
    }

    #[test]
    fn typos_get_a_suggestion() {
        let r = Registry::builtin();
        assert_eq!(r.suggest("buton"), Some("btn"));
        assert_eq!(r.suggest("crad"), Some("card"));
        assert_eq!(r.suggest("secton"), Some("section"));
        assert_eq!(r.suggest("completely-different"), None);
    }

    #[test]
    fn modifier_vocabulary_is_closed() {
        let r = Registry::builtin();
        assert!(r.is_modifier("primary"));
        assert!(!r.is_modifier("Decrement"));
        assert_eq!(Registry::suggest_modifier("primry"), Some("primary"));
    }

    #[test]
    fn attrs_are_validated_per_tag() {
        let r = Registry::builtin();
        assert!(r.accepts_attr("btn", "busy"));
        assert!(r.accepts_attr("btn", "aria")); // global
        assert!(!r.accepts_attr("btn", "where")); // list-only
        assert!(r.accepts_attr("list", "where"));
    }

    #[test]
    fn prompt_context_is_terse() {
        let r = Registry::builtin();
        let ctx = r.prompt_context(&["btn", "card"]);
        assert!(ctx.contains("btn"));
        assert!(ctx.contains("card"));
        assert_eq!(ctx.lines().count(), 2);
    }
}

#[cfg(test)]
mod loadable_tests {
    use super::*;

    const USER: &str = r#"{
      "components": [
        { "name": "callout", "kind": "container", "doc": "Highlighted aside.", "attrs": ["tone"] },
        { "name": "avatar", "kind": "control", "doc": "Round user image.",
          "a11y": { "requires_label": true } }
      ]
    }"#;

    #[test]
    fn a_host_can_add_components_without_recompiling_the_compiler() {
        // The point of the whole change. While `ComponentDef` borrowed from static memory, this was
        // impossible — every new tag meant rebuilding the compiler, which no markup language can ask
        // of the applications embedding it.
        let reg = Registry::from_json(USER).expect("loads");
        let callout = reg.get("callout").expect("callout is in the vocabulary");
        assert_eq!(callout.kind, TagKind::Container);
        assert!(reg.accepts_attr("callout", "tone"));
        assert!(!reg.accepts_attr("callout", "nope"));
        // Builtins survive the merge.
        assert!(reg.get("card").is_some());
    }

    #[test]
    fn a_loaded_component_declares_its_own_accessibility_contract() {
        // What makes the a11y promise extend past the builtin vocabulary: a third-party entry states
        // what the compiler must guarantee, instead of the guarantee stopping at tags we shipped.
        let reg = Registry::from_json(USER).expect("loads");
        assert!(reg.get("avatar").unwrap().requires_label());
        assert!(!reg.get("callout").unwrap().requires_label());
    }

    #[test]
    fn a_user_entry_may_not_shadow_a_builtin() {
        // Merging silently would mean the same document renders differently depending on which
        // registry was loaded, with no diagnostic — the exact failure a closed vocabulary prevents.
        let json = r#"{"components":[{"name":"btn","kind":"control","doc":"mine"}]}"#;
        match Registry::from_json(json) {
            Err(RegistryError::Shadows(name)) => assert_eq!(name, "btn"),
            other => panic!("expected a shadow rejection, got {other:?}"),
        }
    }

    #[test]
    fn an_unusable_tag_name_is_rejected_at_load_time() {
        // The lexer reads a tag as a bare lowercase word, so these could be registered and then
        // never matched by any document. Better to fail loading than to accept a dead entry.
        for bad in ["My Tag", "Card", "3d", "with_underscore", ""] {
            let json = format!(r#"{{"components":[{{"name":"{bad}","kind":"text","doc":"d"}}]}}"#);
            assert!(
                matches!(Registry::from_json(&json), Err(RegistryError::BadName(_))),
                "`{bad}` should not be a usable tag name"
            );
        }
        // And a legal one still loads.
        let json = r#"{"components":[{"name":"call-out","kind":"text","doc":"d"}]}"#;
        assert!(Registry::from_json(json).is_ok());
    }

    #[test]
    fn the_core_level_holds_only_markup() {
        let core = Registry::core();
        assert_eq!(core.level(), Level::Core);
        // Markup survives.
        for tag in ["card", "p", "h1", "btn", "tier", "faq"] {
            assert!(core.get(tag).is_some(), "`{tag}` is markup and belongs in core");
        }
        // A repeater iterates a `data` resource, so it cannot render without the app layer.
        for tag in ["list", "table"] {
            assert!(core.get(tag).is_none(), "`{tag}` needs the app level");
        }
        assert!(Registry::builtin().get("list").is_some(), "the app level still has it");
    }

    #[test]
    fn a_registry_cannot_smuggle_behaviour_into_a_core_host() {
        // A host that asked for markup only gets markup only, even if the registry document it was
        // handed says otherwise. Skipped rather than merged, because the host's decision wins.
        let json = r#"{"components":[
            {"name":"feed","kind":"repeater","level":"app","doc":"app-level"},
            {"name":"aside","kind":"container","doc":"markup"}
        ]}"#;
        let reg = Registry::core().extend_from_json(json).expect("loads");
        assert!(reg.get("feed").is_none(), "an app-level entry reached a core host");
        assert!(reg.get("aside").is_some(), "a markup entry should still load");
    }

    #[test]
    fn a_component_defaults_to_the_core_level() {
        // A hand-written entry that says nothing about level is markup. The safe default matters
        // because `level` is the field a registry author is most likely to omit.
        let json = r#"{"components":[{"name":"aside","kind":"container","doc":"d"}]}"#;
        let reg = Registry::from_json(json).unwrap();
        assert_eq!(reg.get("aside").unwrap().level, Level::Core);
    }

    #[test]
    fn a_registry_round_trips_through_json() {
        // A host publishing the vocabulary it accepts has to be able to serialise it, and reading it
        // back has to produce the same thing — otherwise the published document is not the contract.
        let reg = Registry::from_json(USER).expect("loads");
        let json = reg.to_json();
        // `to_json` includes the builtins, which `from_json` would reject as shadowing, so the
        // round trip is checked against a fresh map rather than through `from_json`.
        #[derive(serde::Deserialize)]
        struct Doc {
            components: Vec<ComponentDef>,
        }
        let doc: Doc = serde_json::from_str(&json).expect("valid json");
        assert_eq!(doc.components.len(), reg.names().count());
        let callout = doc.components.iter().find(|c| c.name == "callout").expect("callout");
        assert_eq!(callout, reg.get("callout").unwrap());
    }

    #[test]
    fn malformed_json_is_an_error_not_a_panic() {
        assert!(matches!(Registry::from_json("{"), Err(RegistryError::Parse(_))));
        assert!(matches!(Registry::from_json("3"), Err(RegistryError::Parse(_))));
        // Both container shapes are legal, and either may be empty.
        assert!(Registry::from_json("[]").is_ok());
        assert!(Registry::from_json(r#"{"components":[]}"#).is_ok());
        // A bare array is the shape a hand-written registry usually starts as.
        let bare = r#"[{"name":"aside","kind":"container","doc":"d"}]"#;
        assert!(Registry::from_json(bare).unwrap().get("aside").is_some());
    }
}
