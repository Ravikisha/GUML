/**
 * tree-sitter grammar for GUML.
 *
 * # The hard part, stated up front
 *
 * GUML is indentation-significant, and whether a line's remainder is *prose* or *structure* depends on
 * the tag — resolved against the component registry, which tree-sitter cannot consult. Neither fact is
 * expressible in a context-free grammar, so both are handled by an **external scanner**
 * (`src/scanner.c`): it emits INDENT/DEDENT/NEWLINE tokens from leading-space counts, and it decides
 * PROSE by looking up the line's first word in a compiled-in tag list.
 *
 * That tag list is generated from `guml registry` by `scripts/gen-tags.mjs`, for the same reason the
 * TextMate grammar is: a second hand-maintained vocabulary is a second answer waiting to disagree with
 * the compiler. It has happened twice in this project already.
 *
 * # What this is for, and what it is not
 *
 * For: editors that colour and fold with tree-sitter (Neovim, Helix, Zed, GitHub), and structural
 * selection. Not for: deciding what GUML means. The normative definition is `spec/tests/*.txt`, and
 * `spec/grammar.ebnf` is the artifact fed to grammar-constrained decoding. A tree-sitter grammar that
 * disagreed with the compiler would be a highlighting bug, not a language change.
 *
 * # Status: 14 of 14 corpus cases pass, and all 10 real documents parse clean.
 *
 * `npm test` runs both, and the second half is the one that matters. `npm run check:fixtures` parses every
 * `.guml` in `fixtures/` and `bench/phase0/examples/` and fails on a single ERROR or MISSING node — see
 * `scripts/check-fixtures.mjs` for why a hand-written corpus was not enough on its own.
 *
 * **Fixed, with the diagnosis, so none of them is reintroduced.** Seven bugs; the last four were found by
 * the fixtures, not by the corpus, and the corpus passed 12/12 with three of them live:
 *
 * * *Pending DEDENTs at end of file.* The last block in a document was never closed and every parse ended
 *   `(MISSING _dedent)`. Easy to miss because a document ending at column zero parses fine.
 * * *A stale generated tag list.* `src/tags.h` held 8 text tags when the registry had 16, so half the
 *   vocabulary's prose lines lexed as piles of identifiers. `npm run check:tags` fails on staleness now
 *   and CI runs it.
 * * *`_raw_line` gated on remembered scanner state instead of on the grammar.* The scanner refused to
 *   supply a verbatim line unless it believed it was inside a `js` body, which made `content_line` — the
 *   rule for `tier`/`faq` bodies — unreachable by construction. The state could not survive anyway; see
 *   `_verbatim_indent` below.
 * * *A sibling closed its own block.* The DEDENT test was `<=` where it had to be `<`, so a body could
 *   hold only one child. All ten corpus cases passed regardless — each happened to have exactly one child
 *   per level.
 * * *Two top-level siblings nested.* At depth zero the indent branch fired for *any* line, because the
 *   stack was empty and the condition was `s->depth == 0 || indent > ...`. Invisible to the corpus for a
 *   subtle reason: every case had a `page` directive before its first indent, and a directive has no body,
 *   so `valid_symbols[INDENT]` was false and the broken branch was never reached.
 * * *A bare text tag produced no token at all.* `divider` and `skeleton` are text-kind and normally carry
 *   no text; the scanner's PROSE branch returned `false` on an empty remainder rather than falling through
 *   to NEWLINE, and `_newline` is external-only, so the line had no possible parse. Nine ERROR nodes in
 *   `fixtures/e.guml`.
 * * *`identifier` was narrower than the compiler's word rule, and positionals had to precede attributes.*
 *   Both are documented at their rules below. Between them, 14 ERROR nodes in `fixtures/c.guml`.
 *
 * **One known limitation.** A document whose *first* line is a text tag — `p Hello world.` with no `page`
 * directive above it — colours its remainder as words rather than as one `prose` node. The prose decision
 * is made while emitting the NEWLINE that ends the previous line (the only way it persists: tree-sitter
 * discards external-scanner state for a call that returns no token), and the first line of a document has
 * no previous line. It affects only documents the compiler rejects with `GUML0041`, and it degrades to
 * imperfect colour rather than to an error, so it is recorded rather than worked around.
 */

module.exports = grammar({
  name: "guml",

  externals: ($) => [
    $._newline,
    $._indent,
    // A body whose lines are verbatim: the indent opening a `tier`/`faq` or a `js`/`raw` block.
    //
    // A *separate* token from `_indent`, and that is the whole fix. With one token the grammar allowed
    // both an element child and a content line in the same body, tree-sitter resolved the ambiguity at
    // generation time in favour of the element, and `_raw_line` was simply not valid in that state — so
    // `tier` perks and `js` bodies both parsed as GUML. The scanner already knows which kind of body it
    // is opening (it looked the tag up in the registry), so saying so in the token removes the ambiguity
    // instead of asking the parser to guess.
    $._verbatim_indent,
    $._dedent,
    // The whole remainder of a line, taken verbatim. The scanner emits this only for a tag whose
    // registry kind is Text, which is the one decision a CFG cannot make here.
    $._prose,
    // A line inside a `js`/`raw` body, or under `tier`/`faq`: not GUML at all.
    $._raw_line,
  ],

  extras: () => [/[ \t]/],

  // A comment line is dropped by the lexer in the real compiler, so it never affects layout. Here it
  // is a node so an editor can colour it, but it is deliberately not part of any rule.
  word: ($) => $.identifier,

  rules: {
    document: ($) => repeat(choice($.comment, $._blank, $._statement)),

    _blank: ($) => $._newline,

    // `content_element` before `element`, so `tier`/`faq` take the rule that knows their body is verbatim.
    _statement: ($) => choice($.directive, $.escape_block, $.content_element, $.element),

    comment: () => token(seq("//", /[^\n]*/)),

    /* ------------------------------------------------------------ directives */

    directive: ($) =>
      choice($.page, $.type_decl, $.state_decl, $.data_decl, $.def_decl),

    page: ($) =>
      seq("page", field("name", $.identifier), repeat($.attribute), $._newline),

    type_decl: ($) =>
      seq("type", field("name", $.identifier), $.brace_group, $._newline),

    state_decl: ($) =>
      seq(
        choice("state", "store"),
        field("name", $.identifier),
        "=",
        // `all|open|done` is an enumerated domain; a single value is just an initial value.
        seq($._value, repeat(seq("|", $._value))),
        $._newline,
      ),

    data_decl: ($) =>
      seq(
        "data",
        field("name", $.identifier),
        optional(seq(":", $.type_ref)),
        field("method", $.method),
        field("url", $.route),
        $._newline,
        optional(seq($._indent, repeat1(choice($.mutation, $._blank)), $._dedent)),
      ),

    mutation: ($) =>
      seq(
        field("name", $.identifier),
        field("method", $.method),
        field("url", $.route),
        optional($.brace_group),
        optional(seq("optimistic", optional(seq(":", $.identifier)))),
        $._newline,
      ),

    def_decl: ($) =>
      seq(
        "def",
        field("name", $.identifier),
        field("params", repeat($.identifier)),
        $._newline,
        // A body is required; an empty `def` is GUML0096.
        seq($._indent, repeat1(choice($.comment, $._blank, $._statement)), $._dedent),
      ),

    method: () => choice("GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"),
    type_ref: ($) => seq($.identifier, optional("[]")),

    /* -------------------------------------------------------- escape hatches */

    // Matched before any tag, because `js`/`raw` are the way *out* of the vocabulary rather than part
    // of it. The body is `_raw_line`, so nothing inside is parsed as GUML.
    escape_block: ($) =>
      seq(
        choice("js", seq("raw", optional(field("target", $.identifier)))),
        $._newline,
        optional(seq($._verbatim_indent, repeat1(choice($.raw_line, $._blank)), $._dedent)),
      ),

    raw_line: ($) => seq($._raw_line, $._newline),

    /* -------------------------------------------------------------- elements */

    // The line remainder below is duplicated between `element` and `content_element` on purpose.
    //
    // It was extracted into a shared `_line_rest` rule, which tree-sitter rejects outright: every part of
    // it is optional, so the rule can match the empty string, and a syntactic rule that can do that is not
    // allowed anywhere but the start rule. Six lines twice is the cost of that constraint.
    //
    // `identifier` covers both a bare label and a modifier. They are the *same token*, and only the
    // registry can tell them apart — `ghost` is a modifier, `Decrement` is a label, and nothing lexical
    // distinguishes them. A separate `modifier` rule made the grammar ambiguous, which is the correct
    // answer: colouring a bare word as a modifier is the language server's job, from `guml highlight`.
    element: ($) =>
      seq(
        field("tag", $.tag),
        // Positionals and attributes in ONE repeat, interleaved, because that is what GUML allows:
        // `tier Hobby $0/mo "For side projects" cta="Start free" /signup` puts a positional route after a
        // keyed attribute. Two consecutive repeats — all positionals, then all attributes — rejected that
        // line, and it is a line in `fixtures/c.guml`. `attribute` first so the one-token lookahead on
        // `=` is resolved toward it.
        repeat(choice($.attribute, $.binding, $.route, $.anchor, $.string, $.number, $.identifier)),
        optional($.action),
        optional(choice(seq("|", $.content), $.prose)),
        $._newline,
        // Element children only. `content_line` used to be a `choice` alternative here, which is what
        // made the whole thing unparseable — see `content_element` below.
        optional(seq($._indent, repeat1(choice($.comment, $._blank, $._statement)), $._dedent)),
      ),

    // `tier` and `faq`: the two tags whose indented children are *content lines* rather than elements.
    //
    // # Why this is its own rule with literal tag names
    //
    // The body shape used to be a `choice` inside `element`, which meant an element child and a content
    // line were both valid at the same position. tree-sitter resolved that at generation time in favour
    // of the element, so `_raw_line` was not in the state's valid set at all — a `js` body parsed as GUML
    // and `content_line` was a rule nothing could ever match.
    //
    // Splitting the *token* into `_indent`/`_verbatim_indent` was necessary and not sufficient: the
    // scanner still had to know which body it was opening, and it tried to remember that from the line
    // above. It cannot. tree-sitter only persists external-scanner state for a call that **returns a
    // token**, and the `tier` header line returns none — so the flag set while reading `tier` was
    // discarded before the body line was reached.
    //
    // Naming the two tags here removes the need to remember anything: after a `tier` header only
    // `_verbatim_indent` is valid, so the scanner reads which indent the *grammar* wants out of
    // `valid_symbols` and emits that. The dependency runs the right way round, and the scanner keeps no
    // cross-line state.
    //
    // These two names are the one place this grammar hardcodes vocabulary. That is a real cost, and the
    // alternative — remembering the parent tag across lines — is not available. `guml registry` prints
    // `content-children:` so `scripts/gen-tags.mjs` can assert this list has not grown.
    content_element: ($) =>
      seq(
        field("tag", alias(choice("tier", "faq"), $.tag)),
        // Positionals and attributes in ONE repeat, interleaved, because that is what GUML allows:
        // `tier Hobby $0/mo "For side projects" cta="Start free" /signup` puts a positional route after a
        // keyed attribute. Two consecutive repeats — all positionals, then all attributes — rejected that
        // line, and it is a line in `fixtures/c.guml`. `attribute` first so the one-token lookahead on
        // `=` is resolved toward it.
        repeat(choice($.attribute, $.binding, $.route, $.anchor, $.string, $.number, $.identifier)),
        optional($.action),
        $._newline,
        optional(seq($._verbatim_indent, repeat1(choice($.content_line, $._blank)), $._dedent)),
      ),

    content_line: ($) => seq($._raw_line, $._newline),

    // Not enumerated. The tag set is not fixed at grammar-writing time: a host can load its own
    // registry, and a document can declare components with `def`. Deciding whether a tag *resolves* is
    // the language server's job, not this grammar's.
    tag: ($) => $.identifier,

    attribute: ($) => seq(field("name", $.identifier), "=", field("value", $._value)),

    _value: ($) => choice($.string, $.number, $.boolean, $.binding, $.identifier),

    // `>` consumes the rest of the line by construction, which is what makes an action lexable in one
    // pass — and why it must be last.
    action: ($) => seq(">", $.action_body),
    action_body: () => token.immediate(/[^\n]*/),

    prose: ($) => $._prose,
    content: () => token.immediate(/[^\n]*/),

    binding: ($) => seq("{", $.expression, "}"),

    // Deliberately shallow. The compiler has a real precedence-climbing parser for expressions
    // (`guml-syntax::expr`); re-deriving its grammar here would be a third implementation of it, and
    // an editor needs to know where an expression *is*, not what it evaluates to.
    expression: () => token.immediate(/[^}\n]*/),

    brace_group: () => token(seq("{", /[^}]*/, "}")),

    string: () => token(seq('"', repeat(choice(/[^"\\]/, seq("\\", /./))), '"')),
    // Declared *before* `identifier`, and that ordering is what disambiguates them.
    //
    // `identifier` below is as permissive as the compiler's word rule, so it matches `24` and `/signup`
    // too. tree-sitter resolves a token conflict by match length first and then by declaration order, so
    // `24` (equal length) comes back as a `number` and `2026-05` (longer) as an identifier — which is
    // exactly the compiler's rule: "a digit run followed immediately by word bytes is a word".
    //
    // These carried `token(prec(1, …))` for one revision, and that is a different mechanism with a
    // different answer: *lexical* precedence is checked before length, so `number` won even when shorter
    // and `state cohort=2026-05|…` lexed as the number `2026` followed by an error. `3px` and `2xl` — both
    // named in `guml_syntax::is_word_byte`'s own doc comment — were broken the same way. Found by pointing
    // `check:fixtures` at the reference corpus, not by the hand-written cases.
    number: () => /-?\d+(\.\d+)?/,
    boolean: () => choice("true", "false"),
    route: () => token(seq("/", /[^\s]*/)),
    anchor: () => token(seq("#", /[A-Za-z][\w-]*/)),

    // Deliberately as permissive as `guml_syntax::is_word_byte`, which it mirrors delimiter for
    // delimiter: a bare word is any run that is neither whitespace nor one of the eleven delimiters.
    //
    // It used to be `/[A-Za-z_][\w-]*/`, and that is where 14 of `fixtures/c.guml`'s ERROR nodes came
    // from — a price `$0/mo` is a single `Word` to the compiler ("deliberately permissive so that things
    // like `$24/mo`, `text-sm`, `tasks.open.count` and `Task[]` lex as one token") and matched nothing at
    // all here. By the rule at the top of this file, a grammar that rejects a document the compiler
    // accepts is the thing that is wrong.
    //
    // Yes, this lets `tag` match `$0/mo`. So does the compiler's lexer — a tag position holds a `Word`,
    // and deciding whether it *resolves* is the language server's job, not this grammar's.
    identifier: () => /[^\s"{}|=:,>#]+/,
  },
});
