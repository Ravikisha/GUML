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
use std::sync::OnceLock;

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

/// What a component's children may be.
///
/// Stated as data for the same reason [`A11y`] is: while the compiler owned every component, a rule
/// like "a `select`'s children are `option`s" could live in a `match` arm in `sema`. The moment a host
/// can load its own vocabulary, a third-party entry has to be able to *declare* its shape, or child
/// checking stops at the builtins and a loaded component accepts anything.
///
/// The default is permissive — an entry that says nothing accepts any child — because a registry
/// author is more likely to omit this field than to mean "no children".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Children {
    /// Only these tags may appear as direct children. Empty means no restriction.
    pub allow: Vec<String>,
    /// These tags may never appear as direct children, even when `allow` is empty.
    pub deny: Vec<String>,
    /// At least one direct child of each of these tags is required.
    pub require: Vec<String>,
}

impl Children {
    pub fn is_unconstrained(&self) -> bool {
        self.allow.is_empty() && self.deny.is_empty() && self.require.is_empty()
    }

    /// Whether `child` is a legal direct child. `deny` wins over `allow`, so a broad allow-list plus
    /// one exclusion is expressible without enumerating the complement, and `deny: ["*"]` says
    /// "no children at all" without requiring an `allow` list of everything else.
    pub fn admits(&self, child: &str) -> bool {
        if self.deny.iter().any(|d| d == child || d == "*") {
            return false;
        }
        self.allow.is_empty() || self.allow.iter().any(|a| a == child)
    }

    /// True when this component takes no children at all.
    pub fn is_leaf(&self) -> bool {
        self.deny.iter().any(|d| d == "*")
    }
}

/// What a component needs from its host in order to work.
///
/// This is the registry half of the security posture the `core`/`app` split starts. `Level` answers
/// "may an untrusted document contain this at all"; these flags answer the narrower question a
/// *backend* has to ask — the no-JavaScript `html` backend cannot lower a component that needs a
/// runtime, and saying so as data means the backend reports it rather than each backend re-deriving
/// the list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Capabilities {
    /// Needs client-side JavaScript. A backend that emits none must report this rather than emit
    /// markup that silently does nothing.
    pub needs_runtime: bool,
    /// Issues network requests of its own.
    pub network: bool,
    /// Reads or writes host storage.
    pub storage: bool,
    /// Backends known to lower this entry. Empty means every backend — the honest default for a
    /// component whose author has not tested one.
    pub backends: Vec<String>,
}

impl Capabilities {
    /// Whether `backend` is expected to lower this entry.
    pub fn lowers_in(&self, backend: &str) -> bool {
        self.backends.is_empty() || self.backends.iter().any(|b| b == backend)
    }
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
    /// What this component's children may be. See [`Children`].
    #[serde(default)]
    pub children: Children,
    /// Named slots this component's body accepts, beyond the unnamed one. Declared so a `def` or a
    /// third-party component can state its shape; the compiler does not invent slots.
    #[serde(default)]
    pub slots: Vec<String>,
    /// The positional slots this component reads, in order — `["label"]` for a `btn`,
    /// `["name", "price", "blurb"]` for a `tier`.
    ///
    /// # Why the *count* matters
    ///
    /// Without it, `btn Add task primary` parsed as two text positionals plus a modifier, codegen read
    /// only the first, and the emitted button said `Add`. The word `task` was deleted with no
    /// diagnostic and no trace — the same data loss the `p Set x=1` bug caused in prose, and forbidden
    /// by the same invariant. A tag that declares how many positionals it reads makes the extra ones
    /// *countable*, so `GUML0099` can report them and suggest the quoting that fixes it.
    ///
    /// Empty means unspecified, and nothing is checked. Names rather than a number because they are
    /// what a diagnostic and a docs page need to say.
    #[serde(default)]
    pub positionals: Vec<String>,
    /// What this component needs from its host. See [`Capabilities`].
    #[serde(default)]
    pub capabilities: Capabilities,
    /// What this component lowers to, when the compiler does not already know.
    ///
    /// # Why a package has to be able to say this
    ///
    /// Without it a loaded component was only half-usable: `guml check` accepted a document using
    /// `callout`, and `guml build` warned "does not yet lower tag `callout`" and emitted a `TODO`. So a
    /// registry package bought validation and no output — and the compiler was right to refuse, because
    /// nothing had told it what a `callout` *is*.
    ///
    /// Two spellings, distinguished by case, because they mean genuinely different things:
    ///
    /// * **lowercase** (`aside`, `figure`) — an HTML element. The compiler emits it directly with the
    ///   theme's classes, exactly as it does for a builtin.
    /// * **PascalCase** (`Callout`) — the *host's own component*, emitted as `<Callout …>` with an import
    ///   from [`Self::import`]. This is the right answer for a design system: a compiler that tried to
    ///   reimplement someone's component would get it subtly wrong, and the point of a registry package is
    ///   that the host already has the implementation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
    /// Module to import [`Self::element`] from, when it is a host component.
    ///
    /// Required for a PascalCase `element`: emitting `<Callout>` with no import produces code that does
    /// not compile, which is a silent mis-lowering with extra steps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import: Option<String>,
    /// The version of GUML, or of the publishing package, that introduced this entry.
    ///
    /// `None` for the entries that predate the field. Present so a host can diff two registries and
    /// see what a version bump added — the append-only tag promise in `spec/STABILITY.md` is only
    /// auditable if each entry says when it arrived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
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

    /// The line this entry contributes to a prompt, which is the only form in which it costs a model
    /// anything.
    pub fn prompt_line(&self) -> String {
        format!("{} ({:?}) — {}\n", self.name, self.kind, self.doc)
    }

    /// Whether [`Self::element`] names a host component rather than an HTML element.
    ///
    /// Case is the signal, matching the rule JSX itself uses: a lowercase tag is an element, a
    /// capitalised one is a component. Reusing that convention means a registry author does not have to
    /// learn a second one.
    pub fn is_host_component(&self) -> bool {
        self.element.as_deref().is_some_and(|e| e.starts_with(char::is_uppercase))
    }

    /// **Estimated** prompt cost of this entry, at ~3.6 chars/token.
    ///
    /// An estimate, and named so. The project's measurement rule is that a published number comes
    /// from the target model's own tokenizer (`CLAUDE.md`, "Claim discipline"); this exists for the
    /// two jobs an estimate is legitimate for — ordering entries by cost, and budgeting a slice
    /// before spending an API call to measure it exactly.
    pub fn approx_prompt_tokens(&self) -> usize {
        (self.prompt_line().len() as f64 / 3.6).ceil() as usize
    }
}

/// The builtin vocabulary, parsed from `components.json` once per process.
///
/// # Why the table is a data file
///
/// It was a `const COMPONENTS: &[Builtin]` compiled into the binary. That is the same mistake
/// `guml_codegen::theme` documents about the class table: a vocabulary written into a compiler is one
/// nobody outside can inspect, diff, or publish. Two concrete gains from moving it:
///
/// * The **builtin registry now travels the same load path a third-party package does.** If
///   `from_json` regresses, every builtin tag vanishes and the whole suite says so — a far stronger
///   test of that path than any hand-written fixture.
/// * A per-entry `children` / `capabilities` / `since` record is *readable* here. In the const-array
///   form the same data was fourteen lines of repetitive Rust per tag.
///
/// Parsed once rather than per `builtin()` call. `builtin()` is called by every `check`, and
/// invariant 6 puts `check` on a 2 ms budget it is already close to; re-parsing ~50 entries per
/// keystroke would be a real regression. Cloning the `BTreeMap` is what the old code did anyway.
static BUILTIN: OnceLock<Registry> = OnceLock::new();

const BUILTIN_JSON: &str = include_str!("../components.json");

fn builtin_registry() -> &'static Registry {
    BUILTIN.get_or_init(|| {
        #[derive(Deserialize)]
        struct Doc {
            components: Vec<ComponentDef>,
        }
        // `expect` is honest here: the file ships with the crate, and
        // `the_builtin_vocabulary_is_a_valid_registry_document` is what makes it so.
        let doc: Doc = serde_json::from_str(BUILTIN_JSON)
            .expect("crates/guml-registry/components.json is checked by a test");
        let mut by_name = BTreeMap::new();
        for def in doc.components {
            by_name.insert(def.name.clone(), def);
        }
        Registry { by_name, level: Level::App }
    })
}

/// Names a loaded registry may not reuse. Derived from the parsed table rather than a second list,
/// so the shadow check cannot drift from the vocabulary it protects.
fn builtin_names() -> &'static BTreeMap<String, ComponentDef> {
    &builtin_registry().by_name
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

/// HTML elements a model reaches for out of habit, and the GUML tag that does the job.
///
/// # Why this is not left to edit distance
///
/// The most common wrong tag in generated GUML is not a *misspelling*, it is an HTML element. Every
/// model has seen far more HTML than GUML, so `div`, `span` and `button` appear constantly — and edit
/// distance cannot help with any of them: `button` → `btn` is three edits, past any threshold that does
/// not also produce nonsense, and `div` → `col` is three. So the diagnostic said only "see the component
/// registry for the available tags", with **no suggestion at all**, which means the repair loop had to
/// spend a full model generation on a rename the compiler could have named exactly.
///
/// Every entry here is a rename that `guml fix` can apply unattended, which is the point: the whole
/// class of failure moves from "costs a round trip" to "costs nothing".
///
/// The third column says *why* the language does not have the element, because a bare rename teaches
/// nothing and the next document will make the same mistake. Where the honest answer is "this needs a
/// different construct, not a different tag" — `ul` for data is a `list` over a resource — the note says
/// so rather than pretending the rename is sufficient.
pub const HTML_HABITS: &[(&str, &str, &str)] = &[
    // Layout. A bare `div` is almost always a vertical stack.
    (
        "div",
        "col",
        "GUML has no generic box: `col` stacks vertically, `row` horizontally, `card` is a bordered surface, `grid` takes `cols=`",
    ),
    ("span", "text", "`text` is the inline text tag"),
    ("main", "section", "`section` is the page-section container"),
    ("article", "card", "`card` is the bordered content surface"),
    ("aside", "sidebar", "`sidebar` is the secondary column"),
    ("header", "hero", "`hero` is the above-the-fold block; `nav` is the navigation bar"),
    // Controls.
    ("button", "btn", "`btn` takes its label as the first positional and its behaviour after `>`"),
    ("a", "link", "`link` takes a `/route` or `#anchor` positional"),
    ("anchor", "link", "`link` takes a `/route` or `#anchor` positional"),
    ("textarea", "input", "`input` covers every text field; `kind=` selects the type"),
    ("checkbox", "check", "`check` binds a boolean"),
    ("switch", "toggle", "`toggle` binds a boolean"),
    (
        "label",
        "text",
        "a control's accessible name is its label positional or `aria=`, never a separate element — that is what makes the name impossible to omit",
    ),
    // Text.
    (
        "h3",
        "h2",
        "headings are `h1` for the page and `h2`/`h` for sections; deeper levels are a sign the page wants splitting",
    ),
    ("h4", "h2", "headings are `h1` for the page and `h2`/`h` for sections"),
    ("h5", "h2", "headings are `h1` for the page and `h2`/`h` for sections"),
    ("h6", "h2", "headings are `h1` for the page and `h2`/`h` for sections"),
    ("strong", "text", "emphasis is presentation, which the theme owns — `text` plus a modifier"),
    ("em", "text", "emphasis is presentation, which the theme owns — `text` plus a modifier"),
    ("b", "text", "emphasis is presentation, which the theme owns — `text` plus a modifier"),
    ("i", "text", "emphasis is presentation, which the theme owns — `text` plus a modifier"),
    ("small", "note", "`note` is the secondary-prose tag"),
    ("hr", "divider", "`divider` is the separator"),
    ("image", "img", "`img` needs `src` and `alt`"),
    // Structures where the rename alone is not the whole answer.
    (
        "ul",
        "menu",
        "for a list of links or actions use `menu`; for data use `list <resource>` with an item template, which also gives you loading, empty and error states",
    ),
    (
        "ol",
        "stepper",
        "for ordered stages use `stepper` with `step` children; for data use `list <resource>`",
    ),
    (
        "li",
        "text",
        "a repeater's children *are* the row template — there is no item wrapper to write",
    ),
    ("tr", "text", "a `table`'s children are one row's cells; there is no row element"),
    ("td", "text", "a `table`'s children are one row's cells"),
    ("th", "text", "a `table` builds its own header from the row template"),
    ("thead", "text", "a `table` builds its own header from the row template"),
    ("tbody", "text", "a `table`'s children are one row's cells"),
    ("dialog", "modal", "`modal` is shown while its `if=` is true"),
    ("details", "faq", "`faq` lowers to `<details>`; each child line is `question | answer`"),
    ("summary", "faq", "`faq` lowers to `<details>`; each child line is `question | answer`"),
    ("nav-bar", "nav", "`nav` is the navigation bar"),
    ("navbar", "nav", "`nav` is the navigation bar"),
    (
        "spinner",
        "skeleton",
        "a resource's loading state is generated — declare the `data` and the `empty` message rather than a spinner",
    ),
    (
        "loader",
        "skeleton",
        "a resource's loading state is generated — declare the `data` and the `empty` message rather than a spinner",
    ),
];

/// The GUML tag an HTML element maps to, if any.
fn html_habit(unknown: &str) -> Option<&'static str> {
    HTML_HABITS.iter().find(|(html, _, _)| *html == unknown).map(|(_, guml, _)| *guml)
}

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

/// The result of inspecting a registry package.
///
/// Every problem at once, rather than the first one. A package author fixing five entries should not
/// need five runs, and `guml add` should be able to say "this is why I will not install it" completely.
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageAudit {
    /// Declared package name, if the document has one.
    pub name: Option<String>,
    /// Declared package version, if the document has one.
    pub version: Option<String>,
    /// Component names the package provides.
    pub components: Vec<String>,
    /// Problems that make the package unusable.
    pub errors: Vec<String>,
    /// Problems that make it work badly.
    pub warnings: Vec<String>,
    /// **Estimated** total prompt cost of every entry, at ~3.6 chars/token. An estimate, and named so:
    /// a published figure comes from the target model's own tokenizer. Useful for the question this is
    /// actually asked — "will adding this package blow my prompt budget" — where the retrieval layer
    /// means the real answer is per-slice anyway.
    pub approx_prompt_tokens: usize,
}

impl PackageAudit {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl Registry {
    pub fn builtin() -> Self {
        builtin_registry().clone()
    }

    /// The version the builtin vocabulary declares. A host pinning tags beyond a given release can
    /// compare against this rather than against the compiler's own version, since the two move
    /// independently.
    pub fn builtin_version() -> &'static str {
        #[derive(Deserialize)]
        struct Doc {
            version: String,
        }
        static V: OnceLock<String> = OnceLock::new();
        V.get_or_init(|| {
            serde_json::from_str::<Doc>(BUILTIN_JSON).map(|d| d.version).unwrap_or_default()
        })
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
        let builtin = builtin_names();

        for def in components {
            if builtin.contains_key(def.name.as_str()) {
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

    /// Inspect a registry *package* without loading it into a vocabulary.
    ///
    /// `extend_from_json` answers "can I use this"; this answers "is this well-formed, and what is in
    /// it" — which is what `guml add` needs before writing a path into a project's config, and what a
    /// package author needs from `guml registry --validate`. Loading and auditing are different jobs:
    /// loading stops at the first fatal problem, and an audit should report everything at once, for the
    /// same reason the parser collects every error in one pass (invariant 1).
    pub fn audit_package(json: &str) -> PackageAudit {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Doc {
            Wrapped {
                components: Vec<ComponentDef>,
                #[serde(default)]
                version: Option<String>,
                #[serde(default)]
                name: Option<String>,
            },
            Bare(Vec<ComponentDef>),
        }

        let mut audit = PackageAudit::default();
        let (components, version, name) = match serde_json::from_str::<Doc>(json) {
            Ok(Doc::Wrapped { components, version, name }) => (components, version, name),
            Ok(Doc::Bare(components)) => (components, None, None),
            Err(e) => {
                audit.errors.push(format!("not a valid registry document: {e}"));
                return audit;
            }
        };
        audit.name = name;
        audit.version = version;

        let builtin = builtin_names();
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();

        for def in &components {
            let n = def.name.as_str();
            if builtin.contains_key(n) {
                audit.errors.push(format!(
                    "`{n}` is a builtin tag; a package may add tags but not redefine them"
                ));
            }
            if !is_usable_tag_name(n) {
                audit.errors.push(format!(
                    "`{n}` is not a usable tag name: lowercase letters, digits and `-`, starting with a letter"
                ));
            }
            if !seen.insert(n) {
                audit.errors.push(format!("`{n}` is declared twice in this package"));
            }
            if def.doc.trim().is_empty() {
                // An error, not a warning. The doc line *is* the entry's prompt representation, so a
                // component without one is invisible to the retrieval layer — present in the vocabulary
                // and impossible for a model to be told about.
                audit.errors.push(format!("`{n}` has no `doc` line, so no prompt can offer it"));
            }
            if def.level == Level::App
                && !(def.capabilities.needs_runtime
                    || def.capabilities.network
                    || def.capabilities.storage)
            {
                audit.warnings.push(format!(
                    "`{n}` is app-level but declares no capability that justifies it — a core host will refuse it with no stated reason"
                ));
            }
            for child in def.children.allow.iter().chain(&def.children.require) {
                if child != "*"
                    && !builtin.contains_key(child.as_str())
                    && !components.iter().any(|c| &c.name == child)
                {
                    audit.warnings.push(format!(
                        "`{n}` names `{child}` as a child, but no builtin or package component has that name"
                    ));
                }
            }
            // A host component with no import emits `<Callout>` into a file that does not import it.
            // That is code which does not compile, so it is an error rather than a warning.
            if def.is_host_component() && def.import.as_deref().unwrap_or("").trim().is_empty() {
                audit.errors.push(format!(
                    "`{n}` lowers to the host component `{}` but declares no `import`; the emitted file would reference an undefined name",
                    def.element.as_deref().unwrap_or("")
                ));
            }
            // A component with no `element` at all is validation-only: documents using it check, and no
            // backend can emit it. Legitimate for a host that only wants the vocabulary closed, and worth
            // saying out loud, because the alternative is discovering it at build time.
            if def.element.is_none() {
                audit.warnings.push(format!(
                    "`{n}` declares no `element`, so it validates but no backend can lower it — add `\"element\": \"aside\"` for an HTML element, or `\"element\": \"Callout\"` with an `import` for your own component"
                ));
            }
            if def.a11y.requires_label && def.kind == TagKind::Container {
                audit.warnings.push(format!(
                    "`{n}` is a container that requires a label; a container's accessible name comes from its title positional, which is easy to omit"
                ));
            }
            audit.approx_prompt_tokens += def.approx_prompt_tokens();
        }

        audit.components = components.into_iter().map(|c| c.name).collect();
        audit
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
    ///
    /// Consults [`HTML_HABITS`] first, because the most common wrong tag is not a typo at all.
    pub fn suggest(&self, unknown: &str) -> Option<&str> {
        if let Some(mapped) = html_habit(unknown)
            && self.get(mapped).is_some()
        {
            // Return the registry's own key, so the lifetime is the registry's rather than `'static`
            // mixed with borrowed — and so a host that removed the tag gets no suggestion for it.
            return self.by_name.get_key_value(mapped).map(|(k, _)| k.as_str());
        }
        let max = if unknown.len() <= 4 { 1 } else { 2 };
        self.by_name
            .keys()
            .map(|n| (n.as_str(), edit_distance(unknown, n)))
            .filter(|(_, d)| *d <= max)
            .min_by_key(|(_, d)| *d)
            .map(|(n, _)| n)
    }

    /// Extra guidance when the unknown tag is an HTML element rather than a misspelling.
    ///
    /// Separate from the suggestion because the two answer different questions: the suggestion is the
    /// edit to apply, and this is why the language does not have the tag the author reached for.
    pub fn habit_note(&self, unknown: &str) -> Option<&'static str> {
        HTML_HABITS.iter().find(|(html, _, _)| *html == unknown).map(|(_, _, note)| *note)
    }

    /// Strict variant (distance 1 only). Used where a false positive would be noisy — e.g.
    /// deciding whether a bare lowercase word is a mistyped modifier or just a label.
    ///
    /// # The length floor
    ///
    /// Short words are excluded, and this is not a tuning knob — it fixes real data loss. `btn Click me`
    /// warned that `me` was a mistyped `md` and attached that as an *applicable* suggestion, so
    /// `guml fix` rewrote the line to `btn Click md`: the word `me` was deleted from the label and a
    /// modifier the author never wrote was added, unattended.
    ///
    /// The vocabulary has five two-character modifiers (`xs`, `sm`, `md`, `lg`, `xl`), and at distance 1
    /// a two-letter word matches one of them whenever it shares a single character — so `me`, `my`, `so`,
    /// `us` are all "typos". That is not a heuristic that is slightly too eager; for words this short it
    /// carries no signal at all. Four characters is where a distance-1 match starts meaning something:
    /// `primry` → `primary`, `ghots` → `ghost`, `outlien` → `outline` all survive.
    ///
    /// A short word that really is a mistyped modifier now surfaces as `GUML0099` instead — quote the
    /// label or fix the word — which is a safe suggestion rather than a destructive one.
    pub fn suggest_modifier_close(unknown: &str) -> Option<&'static str> {
        if unknown.chars().count() < 4 {
            return None;
        }
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
    /// Pick the tags a task description implies, for a prompt slice nobody hand-wrote.
    ///
    /// # Why this is not embeddings
    ///
    /// The obvious approach is a vector search over the doc lines. It is the wrong tool here: the
    /// vocabulary is ~30 short entries, the queries are one-sentence task descriptions, and the cost of
    /// a wrong answer is asymmetric — a *missing* tag makes the task impossible to express, while an
    /// extra tag costs about eight tokens. So this is lexical and deliberately generous, and the
    /// interesting engineering is in the recall side rather than the ranking.
    ///
    /// Three signals, in order of trust:
    ///
    /// 1. **The tag name appears in the prompt.** "a table of invoices" wants `table`.
    /// 2. **A synonym appears.** "button", "form", "list of" — the words a task description actually
    ///    uses, which are not always the tag. This is the table that matters, and it is small enough to
    ///    read.
    /// 3. **A word from the entry's own `doc` appears.** Free recall, since the doc line is already
    ///    written for a model; filtered against a stop list so "the" does not match everything.
    ///
    /// Structural tags are always included, because almost every document needs a container and prose
    /// and no task description says the word "card".
    pub fn tags_for_prompt(&self, prompt: &str) -> Vec<&str> {
        /// Words a task description uses for a tag it does not name.
        ///
        /// Hand-written on purpose: it is the one piece of judgement here, it is short enough to review,
        /// and a generated version would need a corpus this project does not have yet.
        const SYNONYMS: &[(&str, &[&str])] = &[
            ("btn", &["button", "click", "submit", "action", "cta"]),
            ("link", &["link", "navigate", "anchor", "href"]),
            ("input", &["input", "field", "type", "enter", "search", "email", "password", "text"]),
            ("select", &["select", "dropdown", "choose", "picker"]),
            ("check", &["checkbox", "check", "tick", "complete", "completed", "done", "mark"]),
            ("toggle", &["toggle", "switch", "enable", "disable"]),
            ("list", &["list", "items", "rows", "feed", "results", "each"]),
            ("table", &["table", "column", "grid of", "spreadsheet"]),
            ("form", &["form", "submit", "add", "create", "sign up", "sign in", "login"]),
            ("tabs", &["tab", "filter", "segment", "switch between"]),
            ("faq", &["faq", "question", "accordion", "collapse", "disclosure"]),
            ("tier", &["pricing", "plan", "tier", "subscription"]),
            ("metric", &["count", "total", "metric", "kpi", "number", "stat"]),
            ("head", &["heading", "header", "title"]),
            ("nav", &["nav", "navigation", "menu"]),
            ("hero", &["hero", "landing", "above the fold"]),
            ("footer", &["footer"]),
            ("section", &["section"]),
            ("card", &["card", "panel", "tile"]),
            ("empty", &["empty", "no results", "nothing"]),
        ];

        /// Words too common to carry a signal. Short list — the doc lines are terse, so most words in
        /// them are already meaningful.
        const STOP: &[&str] = &[
            "a", "an", "the", "and", "or", "of", "for", "to", "in", "on", "as", "is", "its",
            "with", "first", "one", "per", "use", "used", "optional", "content", "children",
            "line", "instead", "gives", "takes", "renders", "when", "no", "that", "this", "it",
        ];

        let lower = prompt.to_lowercase();
        let words: std::collections::BTreeSet<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2 && !STOP.contains(w))
            .collect();

        let mut picked: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();

        // Always available: a document needs somewhere to put things and something to say, and no task
        // description says the word "card". (`page` is a directive, not a component, so it is not here.)
        for always in ["card", "row", "col", "h", "h1", "p", "text"] {
            if self.get(always).is_some() {
                picked.insert(self.by_name.get_key_value(always).map(|(k, _)| k.as_str()).unwrap());
            }
        }

        for (name, def) in &self.by_name {
            let name = name.as_str();
            // 1. Named outright.
            if words.contains(name) || lower.contains(name) {
                picked.insert(name);
                continue;
            }
            // 2. A synonym. Multi-word entries are matched against the raw prompt, since splitting on
            //    non-alphanumerics would have destroyed them.
            if let Some((_, syns)) = SYNONYMS.iter().find(|(t, _)| *t == name)
                && syns
                    .iter()
                    .any(|s| if s.contains(' ') { lower.contains(s) } else { words.contains(s) })
            {
                picked.insert(name);
                continue;
            }
            // 3. A word from the entry's own doc line.
            let doc = def.doc.to_lowercase();
            if doc
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.len() > 3 && !STOP.contains(w))
                .any(|w| words.contains(w))
            {
                picked.insert(name);
            }
        }

        // Structural implications. More robust than extending the synonym table for every phrasing:
        // "add a task" implies a form, and a form without a field and a submit control is not a form.
        // Applied transitively until nothing new appears, so `table -> empty` picks up too.
        const IMPLIES: &[(&str, &[&str])] = &[
            ("form", &["input", "btn"]),
            ("list", &["text", "empty"]),
            ("table", &["text", "empty"]),
            ("tabs", &["btn"]),
            ("tier", &["btn", "link"]),
            ("nav", &["link"]),
            ("hero", &["h1", "p", "btn"]),
        ];
        loop {
            let mut added = false;
            for (tag, implied) in IMPLIES {
                if !picked.contains(tag) {
                    continue;
                }
                for name in *implied {
                    if let Some((key, _)) = self.by_name.get_key_value(*name)
                        && picked.insert(key.as_str())
                    {
                        added = true;
                    }
                }
            }
            if !added {
                break;
            }
        }

        picked.into_iter().collect()
    }

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
mod package_tests {
    use super::*;

    #[test]
    fn a_good_package_audits_clean() {
        let json = r#"{
          "name": "@acme/design-system", "version": "2.1.0",
          "components": [
            { "name": "callout", "kind": "container", "doc": "Highlighted aside.",
              "element": "Callout", "import": "@acme/design-system" },
            { "name": "figure-block", "kind": "container", "doc": "Figure with a caption.",
              "element": "figure" }
          ]
        }"#;
        let audit = Registry::audit_package(json);
        assert!(audit.ok(), "{:?}", audit.errors);
        assert!(audit.warnings.is_empty(), "{:?}", audit.warnings);
        assert_eq!(audit.name.as_deref(), Some("@acme/design-system"));
        assert_eq!(audit.version.as_deref(), Some("2.1.0"));
        assert_eq!(audit.components, vec!["callout", "figure-block"]);
        assert!(audit.approx_prompt_tokens > 0);
    }

    #[test]
    fn an_audit_reports_every_problem_at_once() {
        // One run, every problem — the same reasoning as invariant 1 for the parser. A package author
        // fixing five entries should not need five runs, and `guml add` should be able to say completely
        // why it will not install something.
        let json = r#"{
          "components": [
            { "name": "dup", "kind": "container", "doc": "One.", "element": "div" },
            { "name": "dup", "kind": "container", "doc": "Two.", "element": "div" },
            { "name": "card", "kind": "container", "doc": "Shadows a builtin.", "element": "div" },
            { "name": "Bad Name", "kind": "text", "doc": "Unusable.", "element": "span" },
            { "name": "noimport", "kind": "container", "doc": "No import.", "element": "Widget" },
            { "name": "nodoc", "kind": "text", "doc": "   ", "element": "span" }
          ]
        }"#;
        let audit = Registry::audit_package(json);
        assert!(!audit.ok());
        let all = audit.errors.join("\n");
        for expected in
            ["declared twice", "builtin tag", "not a usable tag name", "no `import`", "no `doc`"]
        {
            assert!(all.contains(expected), "`{expected}` not reported in:\n{all}");
        }
        assert_eq!(audit.errors.len(), 5, "expected exactly five errors: {:?}", audit.errors);
    }

    #[test]
    fn a_host_component_must_declare_where_it_comes_from() {
        // An error rather than a warning: `<Widget>` with no import is a file that does not compile,
        // which is a silent mis-lowering with extra steps.
        let json = r#"{"components":[
            {"name":"w","kind":"container","doc":"d","element":"Widget"}
        ]}"#;
        let audit = Registry::audit_package(json);
        assert!(!audit.ok());
        assert!(audit.errors[0].contains("declares no `import`"), "{:?}", audit.errors);

        // With one, it is fine — and `is_host_component` reads the case, the same signal JSX uses.
        let json = r#"{"components":[
            {"name":"w","kind":"container","doc":"d","element":"Widget","import":"@acme/ds"}
        ]}"#;
        let reg = Registry::from_json(json).expect("loads");
        assert!(reg.get("w").unwrap().is_host_component());
        assert!(Registry::audit_package(json).ok());
    }

    #[test]
    fn a_lowercase_element_is_an_html_element_not_a_component() {
        let json = r#"{"components":[
            {"name":"figure-block","kind":"container","doc":"d","element":"figure"}
        ]}"#;
        let reg = Registry::from_json(json).expect("loads");
        let def = reg.get("figure-block").unwrap();
        assert!(!def.is_host_component(), "a lowercase element needs no import");
        assert_eq!(def.element.as_deref(), Some("figure"));
        assert!(Registry::audit_package(json).ok());
    }

    #[test]
    fn a_component_with_no_element_is_flagged_as_validation_only() {
        // Legitimate — a host may only want the vocabulary closed — and worth saying out loud, because
        // the alternative is finding out at build time that nothing can emit it.
        let json = r#"{"components":[{"name":"aside-thing","kind":"container","doc":"d"}]}"#;
        let audit = Registry::audit_package(json);
        assert!(audit.ok(), "{:?}", audit.errors);
        assert!(
            audit.warnings.iter().any(|w| w.contains("no backend can lower it")),
            "{:?}",
            audit.warnings
        );
    }

    #[test]
    fn malformed_json_is_one_error_rather_than_a_panic() {
        let audit = Registry::audit_package("{ not json");
        assert!(!audit.ok());
        assert_eq!(audit.errors.len(), 1);
        assert!(audit.errors[0].contains("not a valid registry document"));
    }
}

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn the_builtin_vocabulary_is_a_valid_registry_document() {
        // `builtin_registry` unwraps the parse. This test is what makes that unwrap honest — and it
        // is the reason the vocabulary can live in a data file at all.
        let reg = Registry::builtin();
        assert!(
            reg.names().count() >= 40,
            "expected the 0.2 vocabulary, got {}",
            reg.names().count()
        );
        assert!(!Registry::builtin_version().is_empty(), "the table must declare a version");
        // Every entry is usable by a document, and says what it is for.
        for name in reg.names() {
            assert!(is_usable_tag_name(name), "`{name}` cannot be written in GUML");
            let def = reg.get(name).unwrap();
            assert!(
                !def.doc.trim().is_empty(),
                "`{name}` has no doc line, so a prompt cannot use it"
            );
            assert_eq!(def.name, name, "`{name}` disagrees with its own key");
        }
    }

    #[test]
    fn a_child_contract_is_declared_rather_than_hardcoded() {
        // The point of moving this into the registry: `select` accepts `option` because its *entry*
        // says so, which is the only version of the rule a loaded third-party component can use too.
        let reg = Registry::builtin();
        let select = reg.get("select").unwrap();
        assert!(select.children.admits("option"));
        assert!(!select.children.admits("card"), "an allow-list must exclude what it omits");

        // A component that says nothing accepts anything — the permissive default, because a
        // registry author is likelier to omit the field than to mean "no children".
        assert!(reg.get("card").unwrap().children.is_unconstrained());
        assert!(reg.get("card").unwrap().children.admits("p"));

        // `deny: ["*"]` is how an entry says "leaf" without listing the complement of nothing.
        assert!(reg.get("divider").unwrap().children.is_leaf());
        assert!(!reg.get("divider").unwrap().children.admits("p"));

        // A required child is recorded, so its absence is checkable.
        assert_eq!(reg.get("stepper").unwrap().children.require, vec!["step".to_string()]);
    }

    #[test]
    fn a_component_declares_what_it_needs_from_its_host() {
        let reg = Registry::builtin();
        // A dialog cannot work without JavaScript, and says so rather than leaving each backend to
        // re-derive the list.
        assert!(reg.get("modal").unwrap().capabilities.needs_runtime);
        assert!(reg.get("list").unwrap().capabilities.network);
        // Pure markup needs nothing.
        assert!(!reg.get("badge").unwrap().capabilities.needs_runtime);
        assert!(!reg.get("badge").unwrap().capabilities.network);
        // An empty backend list means "every backend", which is the honest default.
        assert!(reg.get("badge").unwrap().capabilities.lowers_in("html"));
    }

    #[test]
    fn every_app_level_tag_needs_a_runtime_or_a_network() {
        // The two fields answer different questions, but they cannot disagree in this direction: an
        // `app`-level tag is app-level *because* it needs something a core host will not give it. An
        // entry that claims neither is mislabelled, and a core host would be refusing it for no
        // stated reason.
        let reg = Registry::builtin();
        for name in reg.names() {
            let def = reg.get(name).unwrap();
            if def.level == Level::App {
                let c = &def.capabilities;
                assert!(
                    c.needs_runtime || c.network || c.storage,
                    "`{name}` is app-level but declares no capability that justifies it"
                );
            }
        }
    }

    #[test]
    fn token_cost_is_an_estimate_and_tracks_the_doc_line() {
        // Two jobs an estimate is legitimate for: ordering entries by cost, and budgeting a slice
        // before paying for an exact count. Neither needs the number to be exact; both need it to
        // move with the text.
        let reg = Registry::builtin();
        let terse = reg.get("col").unwrap();
        let wordy = reg.get("list").unwrap();
        assert!(
            terse.approx_prompt_tokens() < wordy.approx_prompt_tokens(),
            "a one-line doc should cost less than a three-clause one"
        );
        assert!(terse.approx_prompt_tokens() > 0);
        // And it is the cost of the line that actually enters a prompt.
        assert!(terse.prompt_line().contains("col"));
        assert_eq!(reg.prompt_context(&["col"]), terse.prompt_line());
    }

    #[test]
    fn the_slice_cost_is_the_sum_of_its_entries() {
        // What makes the retrieval claim checkable: the cost of a prompt block is per-entry, so
        // vocabulary size does not inflate a prompt that does not use it.
        let reg = Registry::builtin();
        let tags = reg.tags_for_prompt("A pricing page with three plans.");
        let summed: usize = tags.iter().map(|t| reg.get(t).unwrap().approx_prompt_tokens()).sum();
        let whole: usize = reg.names().map(|n| reg.get(n).unwrap().approx_prompt_tokens()).sum();
        assert!(summed < whole, "a slice that costs the whole vocabulary is not a slice");
    }

    #[test]
    fn a_loaded_entry_may_declare_the_same_metadata_as_a_builtin() {
        // The whole reason this is registry data: a third-party component has to be able to state its
        // shape and its needs, or the guarantees stop at the tags we shipped.
        let json = r#"{"components":[{
            "name": "combobox", "kind": "field", "level": "app",
            "doc": "Filterable dropdown.",
            "children": { "allow": ["option"], "require": ["option"] },
            "slots": ["trigger"],
            "capabilities": { "needs_runtime": true, "backends": ["react"] },
            "since": "1.4.0"
        }]}"#;
        let reg = Registry::from_json(json).expect("loads");
        let def = reg.get("combobox").expect("combobox is in the vocabulary");
        assert!(def.children.admits("option"));
        assert!(!def.children.admits("btn"));
        assert_eq!(def.slots, vec!["trigger".to_string()]);
        assert!(def.capabilities.needs_runtime);
        assert!(def.capabilities.lowers_in("react"));
        assert!(!def.capabilities.lowers_in("html"), "a declared backend list is exhaustive");
        assert_eq!(def.since.as_deref(), Some("1.4.0"));
    }

    #[test]
    fn metadata_survives_a_json_round_trip() {
        // A host publishing the vocabulary it accepts publishes the *contracts* too, or the document
        // it hands out is not the contract it enforces.
        let reg = Registry::builtin();
        let reloaded: BTreeMap<String, ComponentDef> = {
            #[derive(Deserialize)]
            struct Doc {
                components: Vec<ComponentDef>,
            }
            serde_json::from_str::<Doc>(&reg.to_json())
                .expect("valid json")
                .components
                .into_iter()
                .map(|c| (c.name.clone(), c))
                .collect()
        };
        for name in reg.names() {
            assert_eq!(
                reloaded.get(name),
                reg.get(name),
                "`{name}` did not survive the round trip"
            );
        }
    }
}

#[cfg(test)]
mod loadable_tests {
    use super::*;

    // `user-chip` rather than `avatar` deliberately: `avatar` became a builtin in 0.2, and a fixture
    // that shadows one is now rejected — which is the shadow rule working, not a test to loosen.
    const USER: &str = r#"{
      "components": [
        { "name": "callout", "kind": "container", "doc": "Highlighted aside.", "attrs": ["tone"] },
        { "name": "user-chip", "kind": "control", "doc": "Round user image.",
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
        assert!(reg.get("user-chip").unwrap().requires_label());
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

#[cfg(test)]
mod retrieval_tests {
    use super::*;

    fn slice(prompt: &str) -> Vec<String> {
        Registry::builtin().tags_for_prompt(prompt).into_iter().map(str::to_string).collect()
    }

    #[test]
    fn a_task_description_selects_the_tags_it_implies() {
        // The point of the whole thing: nobody hand-writes the tag list for a task.
        let tags = slice("A task list where I can add a task, tick it off, and filter by status.");
        for want in ["list", "form", "input", "btn", "check", "tabs"] {
            assert!(tags.iter().any(|t| t == want), "`{want}` missing from {tags:?}");
        }
    }

    #[test]
    fn a_synonym_selects_a_tag_the_prompt_never_names() {
        // "pricing" is not a tag. `tier` is. A description almost never uses the tag's own name.
        assert!(slice("A pricing page with three plans.").iter().any(|t| t == "tier"));
        assert!(slice("An accordion of common questions.").iter().any(|t| t == "faq"));
        assert!(slice("Show the total number of open items.").iter().any(|t| t == "metric"));
        assert!(slice("A dropdown to choose a country.").iter().any(|t| t == "select"));
    }

    #[test]
    fn structural_tags_are_always_available() {
        // No task description says the word "card", and almost every document needs one. Omitting
        // these would make the slice unusable in a way that looks like a model failure.
        let tags = slice("Anything at all.");
        // `page` is a directive rather than a component, so it is not part of the tag slice.
        for always in ["card", "p", "h1"] {
            assert!(tags.iter().any(|t| t == always), "`{always}` should always be offered");
        }
    }

    #[test]
    fn the_slice_is_smaller_than_the_full_vocabulary() {
        // If it were not, the retrieval layer would be costing tokens rather than saving them.
        let full = Registry::builtin().names().count();
        let tags = slice("A pricing page with three plans and an FAQ.");
        assert!(tags.len() < full, "slice {} vs full {full}", tags.len());
        // And it should not collapse to only the structural set either.
        assert!(tags.len() >= 6, "suspiciously narrow: {tags:?}");
    }

    #[test]
    fn recall_is_favoured_over_precision() {
        // The costs are asymmetric: a *missing* tag makes the task impossible to express, an extra one
        // costs about eight tokens. This pins the direction of the trade rather than a exact set.
        let tags = slice("A dashboard with a table of invoices and a filter.");
        assert!(tags.iter().any(|t| t == "table"), "{tags:?}");
        assert!(tags.iter().any(|t| t == "tabs"), "{tags:?}");
        // `list` is a plausible near-miss for "table of"; offering it too is the right kind of error.
        assert!(tags.len() >= 8, "expected a generous slice, got {tags:?}");
    }

    #[test]
    fn the_slice_only_contains_real_tags() {
        // A hallucinated tag in the prompt would be worse than a missing one: the model would use it.
        let reg = Registry::builtin();
        for tag in reg.tags_for_prompt("A form, a table, a chart, a carousel, a datepicker.") {
            assert!(reg.get(tag).is_some(), "`{tag}` is not in the registry");
        }
    }

    #[test]
    fn a_loaded_component_is_selectable_too() {
        // Retrieval has to work over the *host's* vocabulary, not just the builtin one, or a loaded
        // registry would be invisible to the prompt path.
        let json = r#"{"components":[{"name":"callout","kind":"container","doc":"Highlighted aside for a warning."}]}"#;
        let reg = Registry::from_json(json).expect("loads");
        let tags = reg.tags_for_prompt("A page with a warning callout at the top.");
        assert!(tags.contains(&"callout"), "{tags:?}");
    }

    #[test]
    fn the_prompt_slice_renders_the_selected_entries() {
        // End to end: prompt in, prompt block out, with no hand-written list in between.
        let reg = Registry::builtin();
        let tags = reg.tags_for_prompt("A pricing page with three plans.");
        let block = reg.prompt_context(&tags);
        assert!(block.contains("tier"), "{block}");
        assert!(block.lines().count() == tags.len(), "one line per tag:\n{block}");
    }
}
