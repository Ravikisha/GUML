//! Enforces the append-only promises in `spec/STABILITY.md`.
//!
//! A written stability policy nobody checks is the same kind of artifact as an EBNF nobody checks: a
//! promise that quietly stops being true. This file is the record, and it is deliberately tedious.
//!
//! **How to change it.** Adding a tag, a modifier, an attribute or a diagnostic code means adding a
//! line here, which is normal. *Changing* an existing line means deleting one — and that is the point.
//! A breaking change should be a visible deletion in review rather than a one-character diff to a
//! `match` arm that nobody reads.
//!
//! What each assertion protects:
//!
//! * A **diagnostic id** is what the repair loop keys on. Renumbering one silently changes what a model
//!   repairs, and there is no error message that can warn about it.
//! * A tag's **kind** decides prose-versus-structure for every line using it, so changing `p` from
//!   `Text` to `Container` would reinterpret every paragraph in every document ever written.
//! * A tag's **level** decides whether a core document is valid, so moving a tag from `core` to `app`
//!   invalidates documents that were correct when written.
//! * A **modifier** that disappears breaks documents; one re-pointed at a different meaning is worse,
//!   because the document still compiles and simply looks different.

use guml_registry::{Level, Registry, TagKind};

/// Every tag that has shipped, with the two properties a document's meaning depends on.
///
/// Append only. A new tag goes at the end.
const TAGS: &[(&str, TagKind, Level)] = &[
    ("card", TagKind::Container, Level::Core),
    ("row", TagKind::Container, Level::Core),
    ("col", TagKind::Container, Level::Core),
    ("section", TagKind::Container, Level::Core),
    ("nav", TagKind::Container, Level::Core),
    ("hero", TagKind::Container, Level::Core),
    ("footer", TagKind::Container, Level::Core),
    ("form", TagKind::Container, Level::Core),
    ("tier", TagKind::Container, Level::Core),
    ("faq", TagKind::Container, Level::Core),
    ("tabs", TagKind::Container, Level::Core),
    ("h", TagKind::Text, Level::Core),
    ("h1", TagKind::Text, Level::Core),
    ("h2", TagKind::Text, Level::Core),
    ("p", TagKind::Text, Level::Core),
    ("text", TagKind::Text, Level::Core),
    ("metric", TagKind::Text, Level::Core),
    ("head", TagKind::Text, Level::Core),
    ("empty", TagKind::Text, Level::Core),
    ("btn", TagKind::Control, Level::Core),
    ("link", TagKind::Control, Level::Core),
    ("check", TagKind::Control, Level::Core),
    ("toggle", TagKind::Control, Level::Core),
    ("input", TagKind::Field, Level::Core),
    ("select", TagKind::Field, Level::Core),
    ("list", TagKind::Repeater, Level::App),
    ("table", TagKind::Repeater, Level::App),
    // Meaningful only inside a `def` body, where it marks the insertion point for a call's children.
    ("slot", TagKind::Container, Level::Core),
    // ---- 0.2: the production vocabulary ----
    //
    // Adding these was *not* free, and the cost is worth recording where the policy lives. A document
    // that had `def stat` in it stops compiling, because a def may not shadow a tag (`GUML0093`). Two
    // cases in this repo hit exactly that and were renamed to `kpi`. The failure mode is the acceptable
    // one — compile time, loud, with the name in the message — but "a tag may be added" is only
    // additive for documents that did not already use the name.
    ("option", TagKind::Text, Level::Core),
    ("note", TagKind::Text, Level::Core),
    // `Container`, not `Text`, and this line is the visible edit the policy asks for rather than a
    // one-character diff somewhere else.
    //
    // As a text tag `badge` took its remainder as prose, so `badge danger Breaking` rendered the string
    // "danger Breaking" — while this tag's own registry doc said "use `danger`/`primary`/`quiet` for
    // tone" and `themes/slate.json` carried three tone rules keyed on those exact modifiers. All three
    // were unreachable. Nothing failed, because no fixture used them.
    //
    // Permissible *only* because 0.2 is unreleased: the workspace is 0.1.0 and `badge` is `since: 0.2`,
    // so no published document can contain one. After a release this same change would be forbidden by
    // the assertion below, and the answer would have had to be a second tag.
    ("badge", TagKind::Container, Level::Core),
    ("divider", TagKind::Text, Level::Core),
    ("avatar", TagKind::Text, Level::Core),
    ("img", TagKind::Text, Level::Core),
    ("skeleton", TagKind::Text, Level::Core),
    ("step", TagKind::Text, Level::Core),
    ("alert", TagKind::Container, Level::Core),
    ("grid", TagKind::Container, Level::Core),
    ("sidebar", TagKind::Container, Level::Core),
    ("toolbar", TagKind::Container, Level::Core),
    ("breadcrumb", TagKind::Container, Level::Core),
    ("pagination", TagKind::Container, Level::Core),
    ("stepper", TagKind::Container, Level::Core),
    ("menu", TagKind::Container, Level::Core),
    ("stat", TagKind::Container, Level::Core),
    ("progress", TagKind::Control, Level::Core),
    // These three need a runtime to be anything but dead markup, so they are app-level and say so in
    // `capabilities.needs_runtime` as well. A core host renders documents from untrusted agents; a
    // dialog that traps focus is not something to hand such a document.
    ("modal", TagKind::Container, Level::App),
    ("drawer", TagKind::Container, Level::App),
    ("toast", TagKind::Container, Level::App),
];

/// Modifiers that have shipped. Append only.
const MODIFIERS: &[&str] = &[
    "primary",
    "secondary",
    "outline",
    "ghost",
    "quiet",
    "danger",
    "featured",
    "xs",
    "sm",
    "md",
    "lg",
    "xl",
    "center",
    "start",
    "end",
    "between",
    "wrap",
    "tight",
    "loose",
    "full",
    "disabled",
    "loading",
    "readonly",
    "required",
];

/// Global attributes that have shipped. Append only.
///
/// `class` is *not* here and never was, in the sense that matters: it parsed for a while and was
/// silently discarded, which is not shipping a feature. It is now rejected, and rejecting it is the
/// stable behaviour — presentation belongs to the theme.
const GLOBAL_ATTRS: &[&str] = &[
    "id", "aria", "title", "hidden", "cols", "gap", "w", "if", "disabled", "loading", "readonly",
    "required",
];

#[test]
fn no_tag_has_changed_its_kind_or_level() {
    let reg = Registry::builtin();
    for (name, kind, level) in TAGS {
        let def = reg
            .get(name)
            .unwrap_or_else(|| panic!("tag `{name}` has shipped and may not be removed"));
        assert_eq!(
            def.kind, *kind,
            "`{name}` changed kind — that reinterprets every line using it, in every document ever written"
        );
        assert_eq!(
            def.level, *level,
            "`{name}` changed conformance level — that can invalidate a document which was correct when written"
        );
    }
}

#[test]
fn the_shipped_vocabulary_is_still_present() {
    for m in MODIFIERS {
        assert!(
            guml_registry::MODIFIERS.contains(m),
            "modifier `{m}` has shipped and may not be removed"
        );
    }
    for a in GLOBAL_ATTRS {
        assert!(
            guml_registry::GLOBAL_ATTRS.contains(a),
            "global attribute `{a}` has shipped and may not be removed"
        );
    }
}

#[test]
fn new_vocabulary_has_been_recorded_here() {
    // The other direction: a tag added without a line in `TAGS` would be unprotected, and the omission
    // would only surface the day somebody changed its kind.
    let reg = Registry::builtin();
    let recorded: Vec<&str> = TAGS.iter().map(|(n, _, _)| *n).collect();
    let missing: Vec<&str> = reg.names().filter(|n| !recorded.contains(n)).collect();
    assert!(
        missing.is_empty(),
        "these tags are in the registry but not recorded in spec/STABILITY.md's list: {missing:?}"
    );

    let unrecorded: Vec<&&str> =
        guml_registry::MODIFIERS.iter().filter(|m| !MODIFIERS.contains(m)).collect();
    assert!(unrecorded.is_empty(), "unrecorded modifiers: {unrecorded:?}");
}

#[test]
fn diagnostic_ids_are_append_only() {
    use guml_diagnostics::Code;
    // Every id that has shipped, in order. A gap is fine; a reused or renumbered id is not.
    let ids: Vec<String> = Code::ALL.iter().map(|c| c.id().to_string()).collect();

    // Ids are unique.
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "two codes share an id");

    // Every id parses back to the same code, which is what `guml explain` and the repair loop rely on.
    for code in Code::ALL {
        let id = code.id();
        assert_eq!(Code::from_id(id), Some(*code), "`{id}` does not round-trip through `from_id`");
    }

    // The codes the repair loop and the docs reference by number. Spot-checked rather than exhaustive:
    // the point is that these specific strings cannot move.
    for (id, what) in [
        ("GUML0001", "tab indentation"),
        ("GUML0030", "unknown tag"),
        ("GUML0033", "unknown reference"),
        ("GUML0050", "missing accessible name"),
        ("GUML0074", "unused state"),
        ("GUML0090", "escape hatch"),
        ("GUML0091", "app-level construct at the core level"),
        ("GUML0093", "a def may not shadow an existing tag"),
        ("GUML0094", "def arity"),
        ("GUML0095", "recursive def"),
        ("GUML0099", "a bare word past the last positional slot"),
        ("GUML0100", "a child the component does not accept"),
    ] {
        assert!(Code::from_id(id).is_some(), "`{id}` ({what}) has shipped and may not disappear");
    }
}

#[test]
fn the_frozen_syntax_rules_still_hold() {
    // Not a substitute for the conformance suite — a short list of the properties `spec/STABILITY.md`
    // calls frozen, so a change to any of them fails here with that word in the message.
    use guml_compiler::check;

    // Prose is verbatim.
    let (program, _) = check("page P\np Two  spaces   and x=1 inside\n");
    assert_eq!(
        program.tree[0].content.as_deref(),
        Some("Two  spaces   and x=1 inside"),
        "frozen: a text tag takes its line remainder as prose, verbatim"
    );

    // A tab is an error.
    let (_, diags) = check("page P\ncard A\n\tp One\n");
    assert!(diags.items.iter().any(|d| d.id == "GUML0001"), "frozen: a tab is not an indent unit");

    // An unknown tag is an error, not a passthrough.
    let (_, diags) = check("page P\nnosuchtag x\n");
    assert!(
        diags
            .items
            .iter()
            .any(|d| d.id == "GUML0030" && d.severity == guml_diagnostics::Severity::Error),
        "frozen: an unknown tag is a compile error"
    );

    // A comment never affects layout.
    let (with, _) = check("page P\ncard A\n  // note\n  p One\n");
    let (without, _) = check("page P\ncard A\n  p One\n");
    assert_eq!(
        with.tree[0].children.len(),
        without.tree[0].children.len(),
        "frozen: comments never affect layout"
    );
}
