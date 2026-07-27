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

// `ComponentDef` holds `&'static` slices so the builtin registry costs nothing at runtime;
// that makes it Serialize-only. External registries load through an owned mirror type when
// JSON registry packages land (ROADMAP Phase 4).
#[derive(Debug, Clone, Serialize)]
pub struct ComponentDef {
    pub name: &'static str,
    pub kind: TagKind,
    /// Attribute names accepted beyond the global set. Unknown attrs are diagnosed.
    pub attrs: &'static [&'static str],
    /// Whether an accessible name is mandatory when there is no text label. Drives
    /// `GUML0050`, which is a hard error by design (report §6.4).
    pub requires_label: bool,
    /// One-line description. This text is what goes into the LLM's context, so it is written
    /// for a model, not for a docs site: terse, and states *when* to use the tag.
    pub doc: &'static str,
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
pub const GLOBAL_ATTRS: &[&str] = &[
    "id", "class", "aria", "title", "hidden", "cols", "gap", "w", "if", "disabled", "loading",
    "readonly", "required",
];

/// Tags whose indented children are *content lines*, not nested elements: `tier` perks and
/// `faq` question/answer pairs. Without this, every perk would need its own tag, which is
/// exactly the per-line overhead GUML exists to remove.
pub const TEXT_CHILD_TAGS: &[&str] = &["tier", "faq"];

const COMPONENTS: &[ComponentDef] = &[
    // ---- containers ----
    ComponentDef {
        name: "card",
        kind: TagKind::Container,
        attrs: &[],
        requires_label: false,
        doc: "Bordered surface grouping related content. Optional title as first positional.",
    },
    ComponentDef {
        name: "row",
        kind: TagKind::Container,
        attrs: &[],
        requires_label: false,
        doc: "Horizontal layout. Use `center`/`between` to align instead of flex utilities.",
    },
    ComponentDef {
        name: "col",
        kind: TagKind::Container,
        attrs: &[],
        requires_label: false,
        doc: "Vertical layout.",
    },
    ComponentDef {
        name: "section",
        kind: TagKind::Container,
        attrs: &[],
        requires_label: false,
        doc: "Page section. Takes an optional #anchor and a title positional.",
    },
    ComponentDef {
        name: "nav",
        kind: TagKind::Container,
        attrs: &[],
        requires_label: false,
        doc: "Top navigation bar. First positional is the brand name.",
    },
    ComponentDef {
        name: "hero",
        kind: TagKind::Container,
        attrs: &[],
        requires_label: false,
        doc: "Above-the-fold block: headline, subcopy, calls to action.",
    },
    ComponentDef {
        name: "footer",
        kind: TagKind::Container,
        attrs: &[],
        requires_label: false,
        doc: "Page footer. First positional is the copyright line.",
    },
    ComponentDef {
        name: "form",
        kind: TagKind::Container,
        attrs: &[],
        requires_label: false,
        doc: "Submits on enter and on its primary button; its action runs on submit.",
    },
    ComponentDef {
        name: "tier",
        kind: TagKind::Container,
        attrs: &["cta"],
        requires_label: false,
        doc: "Pricing tier. Positionals: name, price, blurb. Children are perk lines.",
    },
    ComponentDef {
        name: "faq",
        kind: TagKind::Container,
        attrs: &["open"],
        requires_label: false,
        doc: "Accordion. Each child line is `question | answer`.",
    },
    ComponentDef {
        name: "tabs",
        kind: TagKind::Container,
        attrs: &[],
        requires_label: false,
        doc: "Segmented control bound to an enumerated state; options come from its domain.",
    },
    // ---- text ----
    ComponentDef {
        name: "h",
        kind: TagKind::Text,
        attrs: &[],
        requires_label: false,
        doc: "Section heading.",
    },
    ComponentDef {
        name: "h1",
        kind: TagKind::Text,
        attrs: &[],
        requires_label: false,
        doc: "Page headline. One per page.",
    },
    ComponentDef {
        name: "h2",
        kind: TagKind::Text,
        attrs: &[],
        requires_label: false,
        doc: "Subheading.",
    },
    ComponentDef {
        name: "p",
        kind: TagKind::Text,
        attrs: &[],
        requires_label: false,
        doc: "Paragraph of prose.",
    },
    ComponentDef {
        name: "text",
        kind: TagKind::Text,
        attrs: &["strike"],
        requires_label: false,
        doc: "Inline text, usually a binding inside a repeater.",
    },
    ComponentDef {
        name: "metric",
        kind: TagKind::Text,
        attrs: &[],
        requires_label: false,
        doc: "Large single number, for counters and KPI tiles.",
    },
    ComponentDef {
        name: "head",
        kind: TagKind::Text,
        attrs: &[],
        requires_label: false,
        doc: "Page header line; may include a binding for a live count.",
    },
    ComponentDef {
        name: "empty",
        kind: TagKind::Text,
        attrs: &[],
        requires_label: false,
        doc: "Empty-state message for the enclosing repeater.",
    },
    // ---- controls ----
    ComponentDef {
        name: "btn",
        kind: TagKind::Control,
        attrs: &["busy", "type"],
        requires_label: true,
        doc: "Button. First positional is the label; `>` gives the action.",
    },
    ComponentDef {
        name: "link",
        kind: TagKind::Control,
        attrs: &[],
        requires_label: true,
        doc: "Navigational link to a /route or #anchor.",
    },
    ComponentDef {
        name: "check",
        kind: TagKind::Control,
        attrs: &[],
        requires_label: true,
        doc: "Checkbox bound to a boolean; its action runs on change.",
    },
    ComponentDef {
        name: "toggle",
        kind: TagKind::Control,
        attrs: &[],
        requires_label: true,
        doc: "On/off switch bound to a boolean.",
    },
    // ---- fields ----
    ComponentDef {
        name: "input",
        kind: TagKind::Field,
        attrs: &["placeholder", "kind", "min", "max"],
        requires_label: true,
        doc: "Text field bound to a state name given as the first positional.",
    },
    ComponentDef {
        name: "select",
        kind: TagKind::Field,
        attrs: &["placeholder"],
        requires_label: true,
        doc: "Dropdown bound to an enumerated state.",
    },
    // ---- repeaters ----
    ComponentDef {
        name: "list",
        kind: TagKind::Repeater,
        attrs: &["where", "sort", "of"],
        requires_label: false,
        doc: "Renders one child template per item of a resource. Loading, empty and error states are compiled in.",
    },
    ComponentDef {
        name: "table",
        kind: TagKind::Repeater,
        attrs: &["where", "sort", "of"],
        requires_label: false,
        doc: "Tabular repeater; children describe one row.",
    },
];

#[derive(Debug, Clone)]
pub struct Registry {
    by_name: BTreeMap<&'static str, &'static ComponentDef>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl Registry {
    pub fn builtin() -> Self {
        let mut by_name = BTreeMap::new();
        for c in COMPONENTS {
            by_name.insert(c.name, c);
        }
        Self { by_name }
    }

    pub fn get(&self, tag: &str) -> Option<&'static ComponentDef> {
        self.by_name.get(tag).copied()
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
        self.get(tag).map(|c| c.attrs.contains(&attr)).unwrap_or(false)
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.by_name.keys().copied()
    }

    /// Nearest known tag within a small edit distance — the `did you mean` suggestion that
    /// lets the repair loop fix a typo without another model call.
    pub fn suggest(&self, unknown: &str) -> Option<&'static str> {
        let max = if unknown.len() <= 4 { 1 } else { 2 };
        self.by_name
            .keys()
            .map(|n| (*n, edit_distance(unknown, n)))
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
