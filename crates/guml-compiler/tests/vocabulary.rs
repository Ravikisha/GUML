//! The 0.2 production vocabulary, and the cross-backend agreement it exposed.
//!
//! Two separate jobs here.
//!
//! **Every new tag actually lowers.** A tag in the registry with no lowering is worse than no tag: the
//! model is *told* it exists by the prompt slice, uses it, and gets a warning plus a `TODO` comment.
//! `every_registry_tag_lowers_in_the_react_backend` is the check that makes adding a registry entry
//! without a lowering fail immediately.
//!
//! **The backends agree on the element.** They already shared `classes()`, with a test holding them to
//! producing identical class strings on the argument that "GUML is an IR" must be a claim about the
//! language rather than about one emitter. Nothing made the same demand of the *element*, and the three
//! tables had drifted badly:
//!
//! * `nav`, `hero`, `footer` → `<nav>`/`<header>`/`<footer>` in React, `<div>` in the HTML backend. The
//!   no-JavaScript build had **no landmarks**, so a screen-reader user could not jump to the navigation
//!   on a page where the React build let them.
//! * `metric` → `<div>` in React, `<p>` in HTML.
//! * `text` → `<span>` in React, `<p>` in Svelte and HTML.
//! * `hero` → `<div>` in Svelte.
//!
//! Every one of those is a semantic difference between two representations of the same document, and
//! every one survived because each backend's snapshot only ever agreed with itself.

use guml_compiler::check;
use guml_registry::Registry;

fn compile(src: &str, backend: &str) -> (String, Vec<String>) {
    let (program, diags) = check(src);
    assert!(!diags.has_errors(), "source did not compile: {:?}", diags.items);
    let b = guml_codegen::backend(backend).expect("backend exists");
    let out = b.emit(&program);
    let messages = out.diagnostics.items.iter().map(|d| d.message.clone()).collect();
    (out.files[0].contents.clone(), messages)
}

/// One document exercising every markup-level tag added in 0.2, so a single compile covers them all.
const GALLERY: &str = r#"page Gallery

state tone=all|urgent

alert danger
  p Something needs attention.

breadcrumb
  link /home Home
  text Current

toolbar
  btn Export
  badge danger Overdue

grid cols=3
  stat "Revenue" "12,400" delta="+8%"
  stat "Signups" "310"
  card
    note Fine print goes here.

stepper
  step Collect done
  step Review current
  step Publish

menu
  link /a First
  divider
  link /b Second

sidebar
  avatar Ada Lovelace
  img src="/logo.png" alt="The company logo"
  skeleton
  progress value=40 max=100 aria="Upload progress"

pagination
  btn Previous
  text Page 2
  btn Next

select tone aria="Filter by tone"
"#;

#[test]
fn the_whole_new_vocabulary_lowers_without_a_warning() {
    // The point of the gallery: if any 0.2 tag had no lowering, this is where it surfaces — as a
    // warning naming the tag, rather than as a `TODO` comment in someone's emitted output.
    for backend in ["react", "json", "svelte"] {
        let (out, warnings) = compile(GALLERY, backend);
        assert!(warnings.is_empty(), "`{backend}` warned on the 0.2 vocabulary: {warnings:?}");
        assert!(
            !out.contains("TODO(guml)"),
            "`{backend}` left an unlowered construct in its output:\n{out}"
        );
    }
}

#[test]
fn every_registry_tag_lowers_in_the_react_backend() {
    // Directly against the registry rather than against a hand-listed set, so adding an entry with no
    // lowering fails here on the next run instead of the next time a model happens to use it.
    //
    // `raw`/`js` are escape hatches rather than components and are not in the registry; `slot` is
    // meaningful only inside a `def` body and is consumed by expansion before codegen sees it.
    const NOT_AN_ELEMENT: &[&str] = &["slot"];
    let reg = Registry::builtin();
    let mut missing = Vec::new();
    for name in reg.names() {
        if NOT_AN_ELEMENT.contains(&name) {
            continue;
        }
        if !lowers(name) {
            missing.push(name.to_string());
        }
    }
    assert!(
        missing.is_empty(),
        "these registry tags have no lowering, so a prompt offers the model a tag the compiler \
         cannot emit: {missing:?}"
    );
}

/// Whether the React backend has an element or a custom lowering for a tag.
///
/// Asked through the public surface — compile a one-element document and look for the `TODO` marker
/// `unsupported` emits — because `element_for` is crate-private and a test that reached inside it would
/// be checking the table against itself.
fn lowers(tag: &str) -> bool {
    // Repeaters need a resource, `option` needs a `select` parent, and `step` needs a `stepper`; each is
    // covered by the gallery or by a dedicated test below. What is checked here is the flat case.
    let src = match tag {
        "list" | "table" => format!(
            "page P\ntype Row {{id, name}}\ndata rows:Row[] GET /api/rows\n{tag} rows\n  text {{name}}\n  empty None yet.\n"
        ),
        "option" => "page P\nstate s=a|b\nselect s aria=\"S\"\n  option a\n".to_string(),
        "step" => "page P\nstepper\n  step One\n".to_string(),
        "stepper" => "page P\nstepper\n  step One\n".to_string(),
        "tabs" => "page P\nstate view=all|open\ntabs view\n".to_string(),
        "faq" => "page P\nfaq\n  Q | A\n".to_string(),
        "tier" => "page P\ntier \"Pro\" \"$9\" \"For teams\"\n  Everything\n".to_string(),
        "select" => "page P\nstate s=a|b\nselect s aria=\"S\"\n".to_string(),
        "input" => "page P\nstate s=\"\"\ninput s aria=\"S\"\n".to_string(),
        "img" => "page P\nimg src=\"/a.png\" alt=\"A\"\n".to_string(),
        "progress" => "page P\nprogress value=1 max=2 aria=\"P\"\n".to_string(),
        "check" | "toggle" => format!("page P\nstate on=false\n{tag} {{on}} aria=\"On\"\n"),
        "btn" => "page P\nbtn Go\n".to_string(),
        "link" => "page P\nlink /a Home\n".to_string(),
        "modal" | "drawer" | "toast" => {
            format!("page P\nstate open=false\n{tag} \"Title\" if={{open}}\n  p Body.\n")
        }
        "empty" => "page P\ncol\n  empty Nothing.\n".to_string(),
        _ => format!("page P\n{tag} Something\n"),
    };
    let (program, diags) = check(&src);
    assert!(!diags.has_errors(), "`{tag}` probe did not compile: {:?}", diags.items);
    let out = guml_codegen::backend("react").unwrap().emit(&program);
    !out.files[0].contents.contains("TODO(guml)")
}

#[test]
fn every_tag_lowers_to_the_same_element_in_every_backend() {
    // The regression this pins. Checked through emitted output rather than through the shared table,
    // because the table agreeing with itself is not the property that matters — what matters is that
    // the *documents* agree.
    for tag in ["nav", "hero", "footer", "section", "metric", "text", "p", "h1", "sidebar"] {
        // Quoted, because a container reads one positional and two bare words is now `GUML0099`.
        let src = format!("page P\n{tag} \"Words here\"\n");
        let (react, _) = compile(&src, "react");
        let (html, _) = compile(&src, "html");
        let (svelte, _) = compile(&src, "svelte");
        let element = first_element(&react);
        assert!(
            html.contains(&format!("<{element} ")) || html.contains(&format!("<{element}>")),
            "`{tag}` is <{element}> in React but not in the html backend:\n{html}"
        );
        assert!(
            svelte.contains(&format!("<{element} ")) || svelte.contains(&format!("<{element}>")),
            "`{tag}` is <{element}> in React but not in Svelte:\n{svelte}"
        );
    }
}

/// The tag name of the first JSX element in emitted React.
fn first_element(src: &str) -> String {
    let body = src.split("return (").nth(1).expect("a component body");
    let at = body.find('<').expect("an element");
    body[at + 1..].chars().take_while(|c| c.is_alphanumeric()).collect::<String>()
}

fn errors(src: &str) -> Vec<(String, String, Option<String>)> {
    let (_, diags) = check(src);
    diags
        .items
        .iter()
        .filter(|d| d.severity == guml_diagnostics::Severity::Error)
        .map(|d| (d.id.clone(), d.message.clone(), d.suggestion.clone()))
        .collect()
}

#[test]
fn a_bare_word_past_the_last_positional_slot_is_reported() {
    // The regression. `btn Add task primary` compiled with **zero diagnostics** and emitted
    // `<button>Add</button>` — `task` deleted from the output with no trace. Same class as the older
    // `p Set x=1 to enable` bug, and forbidden by the same rule: a word the author wrote does not
    // silently leave the document.
    let found = errors("page P\nbtn Add task primary\n");
    let (id, message, suggestion) = found.first().expect("the dropped word should be reported");
    assert_eq!(id, "GUML0099", "{found:?}");
    assert!(message.contains("1 positional slot (label)"), "{message}");
    assert_eq!(suggestion.as_deref(), Some(r#"btn "Add task" primary"#));

    // Quoted is fine, and so is a label that fits.
    assert!(errors("page P\nbtn \"Add task\" primary\n").is_empty());
    assert!(errors("page P\nbtn Add primary\n").is_empty());
}

#[test]
fn the_suggestion_carries_the_rest_of_the_line() {
    // A suggestion that fixed a dropped word by dropping an attribute would be the same defect wearing
    // a different hat — and `guml fix` applies these unattended, with no human reading the diff.
    let found = errors("page P\nsection #work Selected work cols=3\n");
    let (_, _, suggestion) = found.first().expect("reported");
    assert_eq!(suggestion.as_deref(), Some(r#"section #work "Selected work" cols=3"#));

    // Actions and `|` content too, for the same reason.
    let found = errors("page P\nstate n=0\nbtn Add one more >n = n + 1\n");
    let (_, _, suggestion) = found.first().expect("reported");
    assert_eq!(suggestion.as_deref(), Some(r#"btn "Add one more" >n = n + 1"#));
}

#[test]
fn a_multi_slot_tag_still_takes_all_of_its_slots() {
    // Why the arity is per-entry registry data rather than one rule for every tag: joining bare words
    // into a single label is right for a `btn` and destroys a `tier`, whose three positionals are name,
    // price and blurb.
    assert!(errors("page P\ntier \"Pro\" \"$9\" \"For teams\"\n  Everything\n").is_empty());
    assert!(errors("page P\nstat \"Revenue\" \"12,400\"\n").is_empty());
    // And one word too many is still caught there.
    let found = errors("page P\ntier \"Pro\" \"$9\" \"For teams\" extra\n  Everything\n");
    assert_eq!(found.first().map(|f| f.0.as_str()), Some("GUML0099"), "{found:?}");
}

#[test]
fn a_child_constraint_from_the_registry_is_enforced() {
    // `select` accepts only `option`, and says so in its own registry entry rather than in a `match`
    // arm — which is what makes a loaded third-party component get the same checking.
    assert!(
        errors("page P\nstate s=a|b\nselect s aria=\"S\"\n  option a\n  option b\n").is_empty()
    );

    let found = errors("page P\nstate s=a|b\nselect s aria=\"S\"\n  card \"Nope\"\n");
    let (id, message, _) = found.first().expect("a card inside a select should be reported");
    assert_eq!(id, "GUML0100", "{found:?}");
    assert!(message.contains("`card` is not a valid child of `select`"), "{message}");
}

#[test]
fn a_required_child_that_is_absent_is_reported() {
    // A `stepper` with no `step` renders an empty container — visibly nothing, with the author's intent
    // lost. `children.require` makes that countable.
    assert!(errors("page P\nstepper\n  step One\n").is_empty());
    let found = errors("page P\nstepper\n  p Not a step.\n");
    assert!(
        found.iter().any(|(id, m, _)| id == "GUML0100" && m.contains("needs at least one `step`")),
        "{found:?}"
    );
}

#[test]
fn a_leaf_component_rejects_children() {
    // `children.deny: ["*"]` is how an entry says "no children" without listing the complement.
    let found = errors("page P\ndivider\n  p Inside a rule?\n");
    assert!(found.iter().any(|(id, _, _)| id == "GUML0100"), "{found:?}");
}

#[test]
fn a_conditional_element_is_wrapped_rather_than_given_a_dom_property() {
    // `if` had been a declared global attribute lowered by *nothing*: it fell through to the generic
    // attribute arm and came out as `<div if={open}>`, which React forwards to the DOM and `tsc`
    // rejects. The document said "show this when open" and the output showed it always.
    let src = "page P\nstate open=false\nbtn Show > open = true\ncard \"Details\" if={open}\n  p Hidden.\n";
    let (react, warnings) = compile(src, "react");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(react.contains("{open && ("), "the condition did not become a guard:\n{react}");
    assert!(!react.contains("if={open}"), "`if` leaked out as a DOM property:\n{react}");

    // The no-JavaScript backend cannot re-evaluate it, so it renders the document as it first loads —
    // the same rule it already applies to a binding in prose.
    let (html, _) = compile(src, "html");
    assert!(
        html.contains("hidden — `if=` is false at initial state"),
        "a false-at-load element was rendered anyway:\n{html}"
    );
}

#[test]
fn a_select_offers_its_choices() {
    // A `select` emitted no options at all in every backend, and its bound state name leaked out as the
    // element's text. Both sources of choices are checked, in every backend, because the bug was that
    // each backend had its own idea of what a dropdown contained.
    let from_domain = "page P\nstate tone=all|urgent\nselect tone aria=\"Tone\"\n";
    let from_children =
        "page P\nstate tone=all|urgent\nselect tone aria=\"Tone\"\n  option all\n  option urgent\n";
    for src in [from_domain, from_children] {
        for backend in ["react", "html", "svelte"] {
            let (out, _) = compile(src, backend);
            assert!(
                out.contains("<option value=\"all\"") && out.contains("<option value=\"urgent\""),
                "`{backend}` emitted a dropdown with no choices:\n{out}"
            );
        }
    }
    // A `select` over a plain string state has nothing to offer. The front end catches this earlier and
    // better than codegen can — `GUML0080` names the fix — so codegen's own "nothing to choose from"
    // warning is a backstop that a well-formed document never reaches, and is kept only because a
    // backend should not assume the analyser ran.
    let found = errors("page P\nstate q=\"\"\nselect q aria=\"Q\"\n");
    assert!(found.iter().any(|(id, _, _)| id == "GUML0080"), "{found:?}");
}

#[test]
fn the_no_javascript_backend_reports_a_component_that_needs_one() {
    // Driven by `capabilities.needs_runtime` in the registry, not by a list inside the backend — which
    // is what makes it extend to a component the backend has never heard of.
    let src = "page P\nstate open=true\nmodal \"Edit\" if={open}\n  p Body.\n";
    let (out, warnings) = compile(src, "html");
    assert!(
        warnings.iter().any(|w| w.contains("needs a JavaScript runtime")),
        "a dialog was emitted into a no-JavaScript page with no warning: {warnings:?}"
    );
    // Marked in the output too, so a reader of the file sees the gap and not only a build log.
    assert!(out.contains("data-guml-inert=\"needs runtime\""), "{out}");
    // And it is a `<template>`, which browsers do not render — rather than a visible dialog that
    // cannot be dismissed, which is the worst possible version of this.
    assert!(out.contains("<template data-guml-inert"), "{out}");
}

/// Every diagnostic on a document, error or warning, as `(id, message)`.
fn diagnostics(src: &str) -> Vec<(String, String)> {
    let (_, diags) = check(src);
    diags.items.iter().map(|d| (d.id.clone(), d.message.clone())).collect()
}

#[test]
fn a_badge_takes_a_modifier_for_its_tone() {
    // `badge` was `TagKind::Text`, so its remainder was prose taken verbatim and `badge danger Breaking`
    // rendered the literal string "danger Breaking" — with no diagnostic, while the tag's own registry
    // doc said to use those modifiers for tone and `themes/tailwind.json` carried three tone rules keyed on
    // them. Two thirds of the compiler advertised a feature the third could not deliver.
    //
    // Asserted through the emitted class rather than through the kind, because the kind is not the claim:
    // the claim is that a red badge comes out.
    // The *element's* line, not the whole file: the static-HTML backend inlines the entire theme
    // stylesheet, so `bg-red-600` is present in every document as a CSS rule whether or not anything uses
    // it. Asserting against the file was the first version of this test, and it passed for that reason.
    let span = |src: &str| -> String {
        compile(src, "html")
            .0
            .lines()
            .find(|l| l.contains("<span"))
            .unwrap_or_else(|| panic!("no badge in the output of {src:?}"))
            .to_string()
    };
    let danger = span("page P\nbadge Breaking danger\n");
    let plain = span("page P\nbadge Feature\n");
    assert!(danger.contains(">Breaking<"), "the label is the content, not a modifier: {danger}");
    assert!(!danger.contains("danger Breaking"), "the modifier leaked into the text: {danger}");
    // Named neither palette. This asserted `bg-red-600`, then `bg-destructive`, and broke on each
    // change of default theme — a test about whether a modifier *reaches* the theme cannot also be a
    // test of which colour that theme picked.
    //
    // The claim is that `danger` produces a different badge from an untoned one. That holds under any
    // theme, and still fails for the right reason if the tone rule stops firing.
    assert_ne!(
        danger.split_once('>').map(|(head, _)| head),
        plain.split_once('>').map(|(head, _)| head),
        "the tone rule did not fire: {danger}"
    );

    // And a two-word label still has to be quoted, which is `GUML0099` doing its job rather than a word
    // vanishing — the whole reason the positional arity check exists.
    let found = errors("page P\nbadge Breaking change danger\n");
    assert!(found.iter().any(|(id, _, _)| id == "GUML0099"), "{found:?}");
}

#[test]
fn a_modifier_at_the_start_of_prose_is_reported() {
    // `GUML0102`. The verbatim rule does not bend for a text tag — reclassifying a leading word would
    // silently delete one from prose — so the compiler says what will render instead.
    let found = diagnostics("page P\nnote danger Card declined.\n");
    assert!(
        found.iter().any(|(id, m)| id == "GUML0102" && m.contains("danger Card declined.")),
        "{found:?}"
    );

    // A remainder that is *only* a modifier is the same mistake and reports too.
    let alone = diagnostics("page P\ncol\n  p quiet\n");
    assert!(alone.iter().any(|(id, _)| id == "GUML0102"), "{alone:?}");
}

#[test]
fn ordinary_prose_beginning_with_a_modifier_word_is_left_alone() {
    // The first version of `GUML0102` fired on both of these, which would have made it noise — one of
    // them is the kind of sentence `fixtures/c.guml` is full of. Case is the discriminator: the mistake
    // is a lowercase modifier followed by capitalised content, and sentence prose continues lowercase.
    for src in [
        "page P\np center the label under the field\n",
        "page P\np Start free today, no card needed.\n",
        "page P\np loose leaf tea, sold by weight\n",
    ] {
        let found = diagnostics(src);
        assert!(
            !found.iter().any(|(id, _)| id == "GUML0102"),
            "warned on legitimate prose: {src:?} -> {found:?}"
        );
    }
}

#[test]
fn a_table_lowers_to_a_real_table_in_every_backend() {
    // `table` emitted a `<ul>` in *every* backend, so a document asking for tabular data got a list of rows
    // with no columns, no headers and no header association for a screen reader. `render-emitted.mjs` had
    // asserted "table without header cells" since it was written and the assertion had never once run,
    // because no `<table>` was ever produced for it to check.
    //
    // Pinned here rather than there, and the reason is worth recording: a repeater emits
    // `{rowsLoading ? <skeleton> : …}` with `rowsLoading` starting `true`, so a server render with no data
    // reaches the skeleton and never the table. The structure is only visible in the *source*.
    let src = "page P\n\
               type Inv {id, client, amount:number, paid:bool}\n\
               data invoices:Inv[] GET /api/invoices\n\
               table invoices cols=\"Client, Amount, Paid\"\n\
               \x20 text {client}\n\
               \x20 text {amount}\n\
               \x20 check {paid} readonly aria=\"Paid\"\n\
               \x20 empty None yet.\n";

    // Every backend that emits interactive markup. `html` is excluded on purpose: with no runtime it cannot
    // fetch, so its repeater renders the empty state and there is no table to check — which it says.
    for backend in ["react", "svelte", "wc"] {
        let (out, _) = compile(src, backend);
        assert!(out.contains("<table"), "{backend}: no <table>\n{out}");
        assert!(out.contains("<thead>"), "{backend}: no <thead>\n{out}");
        assert!(out.contains("<tbody"), "{backend}: no <tbody>\n{out}");
        // The association itself: without `scope`, a header cell is decoration. A lower bound rather than an
        // exact count, because the header row is rendered *twice* — once in the loading skeleton and once
        // with the data. That is deliberate: the headers are static, so showing them while loading removes
        // the layout shift instead of merely shrinking it.
        let scoped = out.matches("<th scope=\"col\"").count();
        assert!(scoped >= 3, "{backend}: expected at least three scoped header cells\n{out}");
        assert_eq!(scoped % 3, 0, "{backend}: a partial header row somewhere\n{out}");
        for header in ["Client", "Amount", "Paid"] {
            assert!(out.contains(header), "{backend}: header {header} missing\n{out}");
        }
        // At least one cell per child of the row template. Deliberately a lower bound: the loading skeleton
        // is a table too, and the wc backend emits its whole row on one line, so there is no line-based or
        // exact count that is right for all three. The *exact* arity property — headers matching the row
        // template — is `check_columns`, tested on its own below.
        assert!(out.matches("<td").count() >= 3, "{backend}: fewer than three cells\n{out}");
        // The skeleton takes the table's shape too, so no list item survives anywhere. A `<ul>` placeholder
        // followed by a `<table>` is a visible layout shift the moment the data lands.
        assert!(!out.contains("<li"), "{backend}: still emitting list items\n{out}");
    }
}

#[test]
fn a_table_without_cols_is_reported_and_a_mismatch_too() {
    // A data table with no header row is an accessibility defect, not a style choice. The compiler cannot
    // invent the names — only the author knows what the columns mean — so it reports.
    let src = "page P\n\
               type Inv {id, client}\n\
               data invoices:Inv[] GET /api/invoices\n\
               table invoices\n\
               \x20 text {client}\n\
               \x20 empty None.\n";
    let (_, warnings) = compile(src, "react");
    assert!(
        warnings.iter().any(|w| w.contains("no `cols=`")),
        "an unlabelled table was accepted in silence: {warnings:?}"
    );

    // And a count that does not match the row template, which is the nastier failure: every header would sit
    // one column left of its data, and that reads as correct at a glance.
    let mismatched = src.replace("table invoices", "table invoices cols=\"Client, Amount\"");
    let (_, warnings) = compile(&mismatched, "react");
    assert!(
        warnings.iter().any(|w| w.contains("names 2 column(s) but the row template renders 1")),
        "{warnings:?}"
    );
}

#[test]
fn cols_is_a_number_on_a_grid_and_a_header_list_on_a_repeater() {
    // One attribute name, two types, decided by the tag. A grid's columns are a count the compiler
    // generates; a table's are names only the author knows.
    let grid = compile("page P\ngrid cols=3\n  card A\n  card B\n", "html").0;
    assert!(grid.contains("md:grid-cols-3"), "{grid}");

    // And the numeric rule still applies where it should: `grid cols="Client, Amount"` is an error.
    let bad = errors("page P\ngrid cols=\"Client, Amount\"\n  card A\n");
    assert!(bad.iter().any(|(id, _, _)| id == "GUML0081"), "{bad:?}");
}

mod a_repeater_over_a_derived_array {
    use super::*;

    /// Three filters composed client-side, which is the thing that had no expressible form.
    const COMPOSED: &str = "page P\n\
                            type Event {id, name, channel, country}\n\
                            data events:Event[] GET /api/events\n\
                            state channel=all|web\n\
                            state region=all|GB\n\
                            js\n\
                            \x20 const matches = events.filter((e) => e.channel === channel && e.country === region);\n\
                            list matches of=Event\n\
                            \x20 text {name}\n\
                            \x20 note {country}\n\
                            \x20 empty Nothing matches.\n";

    #[test]
    fn it_compiles_and_the_row_fields_resolve() {
        // `GUML0090` for the hatch is expected — the escape is counted, not waved through. What must not
        // appear is `GUML0033`: `{name}` and `{country}` resolve against `Event` because `of=` said so.
        assert_eq!(errors(COMPOSED).len(), 0, "{:?}", errors(COMPOSED));
        let out = compile(COMPOSED, "react").0;
        assert!(out.contains("item.name"), "row field did not resolve:\n{out}");
        assert!(out.contains("item.country"), "row field did not resolve:\n{out}");
    }

    #[test]
    fn the_js_block_is_declared_before_the_value_that_reads_it() {
        // The bug this pins: `const visibleMatches = matches;` was emitted *above* `const matches = …`, a
        // temporal dead zone error that throws on first render. Ordering, not a type error, so neither the
        // Rust tests nor `tsc` would have caught it — only running the component.
        for backend in ["react", "svelte"] {
            let out = compile(COMPOSED, backend).0;
            let decl = out.find("const matches =").expect("the js block is emitted");
            let read = out.find("Matches = matches").or_else(|| out.find("matches.length"));
            if let Some(read) = read {
                assert!(
                    decl < read,
                    "{backend}: `matches` is read at {read} and declared at {decl}\n{out}"
                );
            }
        }
    }

    #[test]
    fn a_derived_source_gets_no_fetch_scaffolding() {
        // There is no request, so there is no `matchesLoading` and no `matchesError`. Emitting either would
        // reference a name that does not exist — in React that is a compile error, and in Svelte it is a
        // silent `undefined`, which is worse.
        let out = compile(COMPOSED, "react").0;
        assert!(!out.contains("matchesLoading"), "{out}");
        assert!(!out.contains("matchesError"), "{out}");
        // The empty state still applies: "no rows matched" is a real thing to say about a derived array.
        assert!(out.contains("Nothing matches."), "{out}");
    }

    #[test]
    fn a_derived_source_with_no_of_is_an_error_that_names_the_fix() {
        // Nothing can infer the row type: the compiler does not read a `js` body. `GUML0104` says so, and
        // the message carries the fix rather than leaving a backend to warn about an empty list later.
        let src = COMPOSED.replace("list matches of=Event", "list matches");
        let found = errors(&src);
        assert!(found.iter().any(|(id, _, _)| id == "GUML0104"), "{found:?}");
        assert!(
            found.iter().any(|(_, m, _)| m.contains("not a resource")),
            "the message should say why: {found:?}"
        );
    }

    #[test]
    fn an_of_naming_an_undeclared_type_is_reported_at_the_repeater() {
        // Otherwise the row scope is empty and every field read inside becomes `GUML0033`, pointing at the
        // wrong line — the author would go looking at the row template instead of at `of=`.
        let src = COMPOSED.replace("of=Event", "of=Eveny");
        let found = errors(&src);
        assert!(found.iter().any(|(id, _, _)| id == "GUML0062"), "{found:?}");
        // And the message names `of=` rather than just the type, so the reader looks in the right place.
        assert!(found.iter().any(|(_, m, _)| m.contains("of=")), "{found:?}");
    }

    #[test]
    fn the_wc_backend_refuses_it_rather_than_emitting_a_dead_read() {
        // That backend emits a class body, so a `js` block has nowhere to live — it reports one already. The
        // first attempt here emitted `const rows = s.matches`, a read of `#state.matches` that is never
        // assigned, so the list would have rendered its empty state forever with no diagnostic.
        let (_, warnings) = compile(COMPOSED, "wc");
        assert!(
            warnings.iter().any(|w| w.contains("derived array") && w.contains("raw wc")),
            "wc should refuse and name the escape: {warnings:?}"
        );
    }
}

/// The two errors that stopped model-generated GUML from compiling more than any others.
///
/// `bench/gen` ran six applications through an 8B and a 70B model. One of six compiled, while
/// **32 of 37 functional requirements were met** — the models understood the applications and failed
/// on surface rules. The two rules below were the surface, and in both cases the compiler already held
/// the information it was refusing to use.
mod what_a_model_actually_writes {
    use super::*;

    /// The obvious spelling: options written where they are used.
    const AS_A_MODEL_WRITES_IT: &str = "page P
state c: a

select c
  option a
  option b
";

    #[test]
    fn options_may_be_written_as_children_rather_than_as_a_domain() {
        // `GUML0080` checked only the bound state's domain and never looked at the `option` children,
        // while `guml_codegen::select_options` had reconciled both for a while — so codegen accepted
        // a spelling validation rejected. Two halves of one compiler disagreeing about one document.
        let found = errors(AS_A_MODEL_WRITES_IT);
        assert!(
            !found.iter().any(|(id, _, _)| id == "GUML0080"),
            "options written as children are still refused: {found:?}"
        );
    }

    #[test]
    fn a_field_is_named_from_the_state_it_binds() {
        // `GUML0051` refused a field with no `aria`, while the state name sat in the same line.
        let found = errors(AS_A_MODEL_WRITES_IT);
        assert!(
            !found.iter().any(|(id, _, _)| id == "GUML0051"),
            "a derivable name is still an error: {found:?}"
        );
    }

    #[test]
    fn the_document_a_model_writes_now_compiles() {
        assert!(errors(AS_A_MODEL_WRITES_IT).is_empty(), "{:?}", errors(AS_A_MODEL_WRITES_IT));
    }

    /// The derived name must actually be **emitted**, in every backend that can emit one.
    ///
    /// This is the half that makes the warning honest. Warning "named from the state it binds" beside
    /// output carrying no name would be the silent mis-lowering invariant 3 forbids — and the HTML
    /// backend was already emitting `aria-label="select"`, the *tag name*, which is present and tells
    /// a screen reader nothing.
    #[test]
    fn every_backend_emits_the_derived_name() {
        for backend in ["react", "html", "svelte"] {
            let (out, _) = compile(AS_A_MODEL_WRITES_IT, backend);
            assert!(
                out.contains(r#"aria-label="c""#),
                "`{backend}` does not emit the derived accessible name:\n{out}"
            );
            assert!(
                !out.contains(r#"aria-label="select""#),
                "`{backend}` named the field after its tag, which announces nothing"
            );
        }
    }

    /// Derived, not invented: an explicit name always wins.
    #[test]
    fn an_explicit_name_is_never_overridden() {
        let src = "page P
state c: a

select c aria=\"Colour\"
  option a
  option b
";
        assert!(errors(src).is_empty(), "{:?}", errors(src));
        let (out, _) = compile(src, "react");
        assert!(out.contains(r#"aria-label="Colour""#), "{out}");
        assert!(!out.contains(r#"aria-label="c""#), "the derived name overrode an explicit one");
    }

    /// A warning, not silence. A state name is a variable name — usually a real word, occasionally
    /// `x1` — and the compiler cannot tell which, so the author is told a better one may exist.
    #[test]
    fn deriving_a_name_is_reported_as_a_warning() {
        let (_, diags) = guml_compiler::check(AS_A_MODEL_WRITES_IT);
        let warned = diags
            .items
            .iter()
            .any(|d| d.id == "GUML0051" && d.severity == guml_diagnostics::Severity::Warning);
        assert!(
            warned,
            "{:?}",
            diags.items.iter().map(|d| (&d.id, &d.message)).collect::<Vec<_>>()
        );
    }

    /// And a `select` with genuinely nothing to choose from is still an error.
    #[test]
    fn a_select_with_no_options_at_all_is_still_refused() {
        let src = "page P
state c: a

select c aria=\"Colour\"
";
        assert!(
            errors(src).iter().any(|(id, _, _)| id == "GUML0080"),
            "a control the reader cannot operate must still be reported: {:?}",
            errors(src)
        );
    }
}
