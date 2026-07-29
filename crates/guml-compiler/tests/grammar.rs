//! Keeps `spec/grammar.ebnf` from drifting away from the parser.
//!
//! # What this does and does not do
//!
//! It does **not** parse the EBNF or generate a parser from it. That would be the strong version, and
//! it is not what is on offer here.
//!
//! What it does is catch the drift that actually happened: `page_decl` read `"page" IDENT NEWLINE` for
//! several releases after `page` gained metadata attributes, and nothing noticed, because nothing
//! executed the file. So this test asserts that every directive, every registry tag and every modifier
//! is at least *mentioned* in the grammar — which fails the moment someone adds a construct to the
//! parser and not to the file.
//!
//! The normative definition of the language is `spec/tests/*.txt`, which is executed. The EBNF is the
//! artifact fed to grammar-prompting and grammar-constrained decoding, and its job is to be complete
//! rather than to be the authority.

fn grammar() -> String {
    std::fs::read_to_string("../../spec/grammar.ebnf").expect("spec/grammar.ebnf exists")
}

#[test]
fn every_directive_appears_in_the_grammar() {
    let ebnf = grammar();
    for directive in ["page", "type", "state", "store", "data"] {
        assert!(
            ebnf.contains(&format!("\"{directive}\"")),
            "`{directive}` is a directive the parser accepts but the grammar does not mention"
        );
    }
    // The escape hatches are part of the surface a model has to be able to produce.
    for hatch in ["js", "raw"] {
        assert!(ebnf.contains(&format!("\"{hatch}\"")), "`{hatch}` is missing from the grammar");
    }
}

#[test]
fn every_registry_tag_is_reachable_through_the_grammar() {
    // Tags are not enumerated in the EBNF — `TAG` is a terminal resolved against the registry, which is
    // the right factoring, because a loadable registry means the tag set is not fixed at grammar-writing
    // time. So what is asserted is that the file says so, rather than listing tags it cannot know.
    let ebnf = grammar();
    assert!(ebnf.contains("TAG"), "the grammar has no TAG terminal");
    assert!(
        ebnf.to_lowercase().contains("registry"),
        "the grammar must say that TAG resolves against the component registry, since a loadable \
         registry means the tag set is not fixed here"
    );
    // A sanity check that the registry is non-empty, so this test cannot pass vacuously.
    assert!(guml_registry::Registry::builtin().names().count() >= 27);
}

#[test]
fn the_page_metadata_attributes_are_in_the_grammar() {
    // The specific drift this file exists because of.
    let ebnf = grammar();
    for attr in ["title", "description", "lang", "dir"] {
        assert!(
            ebnf.contains(&format!("\"{attr}\"")),
            "`page {attr}=` is accepted by the parser but missing from the grammar"
        );
    }
}

#[test]
fn the_grammar_does_not_still_claim_to_be_normative() {
    // A file that calls itself normative and is not executed is worse than one that admits what it is.
    let ebnf = grammar();
    assert!(
        ebnf.contains("spec/tests"),
        "the grammar should point at the conformance suite as the normative definition"
    );
    assert!(
        !ebnf.contains("it is the normative surface"),
        "the grammar still claims to be normative; the executed conformance suite is"
    );
}

#[test]
fn the_prose_rule_is_stated_correctly() {
    // The old wording — "the line contains no `=`" — described a rule that deleted words from prose.
    let ebnf = grammar();
    assert!(
        !ebnf.contains("the line contains no \"=\""),
        "the grammar still states the prose rule as \"contains no =\", which was the bug: \
         `p Set x=1 to enable` is prose"
    );
    assert!(
        ebnf.contains("attribute the registry accepts"),
        "the grammar should state the corrected prose rule"
    );
}
