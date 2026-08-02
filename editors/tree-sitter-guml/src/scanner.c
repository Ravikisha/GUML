/* External scanner for tree-sitter-guml.
 *
 * Two of GUML's rules cannot be written in a context-free grammar, so they live here. Both mirror
 * `crates/guml-syntax/src/lib.rs`, and where the two disagree, that file is right and this one is a
 * highlighting bug.
 *
 *   1. INDENT / DEDENT from leading-space counts. Children are the following lines with a *strictly
 *      greater* indent, applied recursively: `4` then `5` is a parent and a child, not two ragged
 *      siblings. A scanner that normalised to two-space steps would build a different tree from the
 *      compiler's, on documents that are perfectly legal.
 *
 *   2. PROSE, for a line whose tag is text-kind. `btn Decrement ghost` is a label plus a modifier;
 *      `p Press the buttons` is prose taken verbatim. Only the registry knows which, so the text-kind
 *      tag list is generated into `tags.h` from `guml registry`.
 *
 * A third job falls out of the second: RAW_LINE, for the body of a `js`/`raw` block. That is not GUML at
 * all — not lexed, not checked — so the scanner hands the whole line back untouched.
 *
 * # Bugs recorded so they are not reintroduced
 *
 * * PROSE was emitted for *every* tag, because the text-kind lookup was written and then never called.
 *   `card A` parsed `A` as prose. Nothing but running the parser would have found it.
 * * RAW_LINE was returned before the INDENT that opens its block, leaving the grammar with a body it
 *   had never entered. Order matters: INDENT and DEDENT always come first.
 * * A sibling closed its own block, because the DEDENT test was `<=` instead of `<`.
 * * At depth zero any line opened a block, so top-level siblings nested. Column zero is the implicit base
 *   of the indent stack; see the indent branch.
 * * A text-kind tag with an *empty* remainder returned false rather than falling through to NEWLINE, which
 *   left `divider` and `skeleton` with no possible parse at all.
 *
 * Three of the five passed the whole hand-written corpus. `npm run check:fixtures` — parse every real
 * `.guml` in the repository, fail on any ERROR node — is what found them, and is the check to trust.
 */

#include <string.h>
#include <tree_sitter/parser.h>

#include "tags.h"

/* Order must match the `externals` array in grammar.js exactly. */
enum TokenType {
  NEWLINE,
  INDENT,
  /* The indent opening a body whose lines are verbatim: a `tier`/`faq` or a `js`/`raw` block.
   *
   * A separate token from INDENT, and that separation is the fix for the last two corpus failures. With
   * one token the grammar allowed both an element child and a content line at the same position;
   * tree-sitter resolved the ambiguity at generation time in favour of the element, and RAW_LINE was
   * simply not valid in that state. So a `js` body parsed as GUML and `content_line` was a rule nothing
   * could ever match. The scanner already knows which body it is opening — it looked the tag up in the
   * registry — so saying so in the token removes the ambiguity rather than asking the parser to guess. */
  VERBATIM_INDENT,
  DEDENT,
  PROSE,
  RAW_LINE,
};

/* The indent stack. 32 levels is far past anything a person writes; the compiler's fuzz corpus goes to
 * 200, and beyond the cap the scanner stops pushing rather than overflowing — a pathological document
 * highlights imperfectly instead of crashing the editor. */
#define MAX_DEPTH 32

typedef struct {
  uint16_t depth;
  uint16_t indents[MAX_DEPTH];
  /* Whether the line being lexed has a text-kind tag, so its remainder is prose.
   *
   * The only piece of cross-call state left, and where it is *written* is the whole subtlety. It is set by
   * `peek_next_line` while the scanner is producing the NEWLINE that ends the line above — because
   * tree-sitter persists external-scanner state only for a call that returns a token, and a call that
   * returns `false` throws the mutation away. The line-start branch recomputes the same answer for the
   * case where it is returning an indent token, which agrees by construction: same word, same lookup.
   *
   * Setting it *only* at line start is what the previous version did, and it did not work at top level: a
   * `p Hello world.` at column zero needs no indent token, so that call returned false and the flag was
   * gone by the time PROSE was asked for. It appeared to work for a nested `p One` purely because INDENT
   * happened to be returned on the same call.
   *
   * There used to be two more fields here, `escape_indent` and `content_indent`, remembering which kind
   * of body the line above had opened. They could not work: tree-sitter only persists scanner state for a
   * call that **returns a token**, and a `tier` or `js` header line produces no external token, so the
   * flag was discarded before the body line was reached. The grammar answers the same question through
   * `valid_symbols` instead. */
  bool line_is_text;
} Scanner;

void *tree_sitter_guml_external_scanner_create(void) { return calloc(1, sizeof(Scanner)); }

void tree_sitter_guml_external_scanner_destroy(void *payload) { free(payload); }

unsigned tree_sitter_guml_external_scanner_serialize(void *payload, char *buffer) {
  memcpy(buffer, payload, sizeof(Scanner));
  return sizeof(Scanner);
}

void tree_sitter_guml_external_scanner_deserialize(void *payload, const char *buffer,
                                                   unsigned length) {
  if (length == sizeof(Scanner)) {
    memcpy(payload, buffer, length);
  } else {
    memset(payload, 0, sizeof(Scanner));
  }
}

static bool is_word(int32_t c) {
  return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' ||
         c == '-';
}

static void consume_to_eol(TSLexer *lexer) {
  while (lexer->lookahead != '\n' && !lexer->eof(lexer)) {
    lexer->advance(lexer, false);
  }
}


/* Decide whether the line starting at the current position has a text-kind tag.
 *
 * # Why this is called while producing NEWLINE rather than at line start
 *
 * `line_is_text` has to be set by one call and read by another — the tag is behind the cursor by the time
 * PROSE is asked for. And tree-sitter **only persists external-scanner state for a call that returns a
 * token**, so setting it from a call that returns `false` throws it away.
 *
 * At line start there is often no token due: a top-level `p Hello world.` needs no INDENT, so that call
 * returned false and the flag vanished. It worked for a *nested* `p One` purely because INDENT happened to
 * be returned on the same call — which is why the corpus passed while the feature was broken.
 *
 * NEWLINE always returns a token. So the decision is made there, looking one line ahead past the newline
 * the scanner has just consumed. Nothing is marked consumed by the peek: `mark_end` has already fixed the
 * token's end at the newline. */
static void peek_next_line(TSLexer *lexer, Scanner *s) {
  s->line_is_text = false;
  while (lexer->lookahead == ' ' || lexer->lookahead == '	') {
    lexer->advance(lexer, false);
  }
  char word[32];
  unsigned n = 0;
  while (n + 1 < sizeof(word) && is_word(lexer->lookahead)) {
    word[n++] = (char)lexer->lookahead;
    lexer->advance(lexer, false);
  }
  word[n] = 0;
  for (unsigned i = 0; i < GUML_TEXT_TAGS_COUNT; i++) {
    if (strcmp(word, GUML_TEXT_TAGS[i]) == 0) {
      s->line_is_text = true;
      return;
    }
  }
}

bool tree_sitter_guml_external_scanner_scan(void *payload, TSLexer *lexer,
                                            const bool *valid_symbols) {
  Scanner *s = (Scanner *)payload;

  /* End of file closes every open block.
   *
   * Without this the last block in a document is never closed and the parse ends with
   * `(MISSING _dedent)` — the grammar is still inside a body it has no way to leave. Every
   * indentation-sensitive grammar needs this and it is easy to miss, because a document that happens to
   * end at column zero parses fine.
   *
   * Zero-width: nothing is consumed, so the same EOF position serves each level in turn. */
  if (lexer->eof(lexer)) {
    if (valid_symbols[DEDENT] && s->depth > 0) {
      s->depth--;
      lexer->result_symbol = DEDENT;
      return true;
    }
    return false;
  }

  if (lexer->get_column(lexer) == 0) {
    /* Leading whitespace. A tab is `GUML0001` in the compiler; here it counts as two spaces so the rest
     * of the line still highlights, which is the same recovery the real lexer performs. */
    uint16_t indent = 0;
    for (;;) {
      if (lexer->lookahead == ' ') {
        indent++;
        lexer->advance(lexer, true);
      } else if (lexer->lookahead == '\t') {
        indent += 2;
        lexer->advance(lexer, true);
      } else {
        break;
      }
    }

    /* A blank line neither opens nor closes a block, and must not clear the escape state — which is why
     * a blank line inside a `js` body does not end it. */
    if (lexer->lookahead == '\r' || lexer->lookahead == '\n' || lexer->eof(lexer)) {
      if (valid_symbols[NEWLINE] && !lexer->eof(lexer)) {
        while (lexer->lookahead == '\r') {
          lexer->advance(lexer, true);
        }
        if (lexer->lookahead == '\n') {
          lexer->advance(lexer, false);
        }
        lexer->mark_end(lexer);
        peek_next_line(lexer, s);
        lexer->result_symbol = NEWLINE;
        return true;
      }
      return false;
    }

    /* The token this call may return is zero-width, so its end is fixed here — before the first word is
     * peeked at. Without this, peeking would extend the INDENT over the tag.
     *
     * The order below matters and cost two debugging rounds: an early `return INDENT` meant the first
     * word was never inspected, so `line_is_text` stayed false and `p One` never produced PROSE. State
     * is updated first; the token decision comes after. */
    lexer->mark_end(lexer);

    char word[32];
    unsigned n = 0;
    while (n + 1 < sizeof(word) && is_word(lexer->lookahead)) {
      word[n++] = (char)lexer->lookahead;
      lexer->advance(lexer, false);
    }
    word[n] = 0;

    /* Is this line's remainder prose? The one decision that genuinely needs the registry.
     *
     * Unconditional, and it has to be. Gating it on `valid_symbols[PROSE]` looks tidier and is wrong: at
     * *line start* the parser is expecting an indent token, so PROSE is not yet valid, and the flag would
     * be false by the time the mid-line call needs it. `card A` / `p One` regressed to an identifier that
     * way.
     *
     * Nothing is lost by asking early. Inside a verbatim body the grammar does not accept PROSE at all, so
     * the mid-line branch checks `valid_symbols[PROSE]` and a `p` inside a `js` block never becomes prose —
     * the grammar refuses it, rather than the scanner having to remember where it is. */
    s->line_is_text = false;
    for (unsigned i = 0; i < GUML_TEXT_TAGS_COUNT; i++) {
      if (strcmp(word, GUML_TEXT_TAGS[i]) == 0) {
        s->line_is_text = true;
        break;
      }
    }

    /* DEDENT and INDENT first, one level per call, so blocks close in order.
     *
     * Strictly `<`, not `<=`. With `<=` a *sibling* at the same indent closed the block it was in, so a
     * body could only ever hold one child — `tier Pro` with two perk lines lexed the first as a content
     * line and the second as a top-level element. Every corpus case passed anyway, because each of them
     * happened to have exactly one child at each level. */
    if (valid_symbols[DEDENT] && s->depth > 0 && indent < s->indents[s->depth - 1]) {
      s->depth--;
      lexer->result_symbol = DEDENT;
      return true;
    }

    /* Which indent does the *grammar* want here?
     *
     * # Why this is read from `valid_symbols` and not remembered
     *
     * The scanner used to decide by looking the line above's tag up in the registry and storing a flag.
     * That cannot work, and it is worth writing down why, because the failure is silent and the mechanism
     * is not obvious: **tree-sitter only persists external-scanner state for a call that returns a
     * token.** A `tier` or `js` header line produces no external token — the internal lexer reads the tag
     * — so the flag set while reading it was discarded before the body line was ever reached.
     *
     * The grammar already knows. After a `js`/`raw` or `tier`/`faq` header only `_verbatim_indent` is
     * valid; after any other element only `_indent` is. So the dependency runs the other way: the scanner
     * asks which one is acceptable and emits that. No cross-line state, and nothing to lose. */
    enum TokenType indent_token = valid_symbols[VERBATIM_INDENT] ? VERBATIM_INDENT : INDENT;
    /* Column zero is the enclosing indent at depth zero — an implicit base of the stack.
     *
     * The condition used to be `s->depth == 0 || indent > ...`, which emitted an indent token for *any*
     * top-level line whenever the stack was empty. Two top-level siblings therefore nested: in
     * `p One.` / `btn Go` / `p Three.` the two later lines became children of the first. Every corpus case
     * passed regardless, because in each one the line before the first indent was a `page` directive —
     * which has no body, so `valid_symbols[INDENT]` was false and the branch was never reached. */
    uint16_t enclosing = s->depth > 0 ? s->indents[s->depth - 1] : 0;
    if (valid_symbols[indent_token] && indent > enclosing) {
      if (s->depth < MAX_DEPTH) {
        s->indents[s->depth++] = indent;
      }
      lexer->result_symbol = indent_token;
      return true;
    }

    /* A verbatim line reached at line start rather than mid-line: the second and later lines of a body,
     * where no indent token is due. */
    if (valid_symbols[RAW_LINE]) {
      consume_to_eol(lexer);
      lexer->mark_end(lexer);
      lexer->result_symbol = RAW_LINE;
      return true;
    }

    return false;
  }

  /* Mid-line, in a body whose lines are taken verbatim. Reached because returning an indent token moved
   * the column past the indent, so the line-start branch does not run again for this line.
   *
   * `valid_symbols[RAW_LINE]` is the whole condition, and getting there took three attempts. It is the
   * *grammar* saying "a verbatim line is legal here", and the grammar is the authority on where — the
   * scanner only has to supply "the rest of this line, untouched". Every version that tried to
   * second-guess it from remembered state was wrong, because the state could not survive: tree-sitter
   * persists scanner state only for a call that returns a token, and a `tier` or `js` header line produces
   * none. */
  if (valid_symbols[RAW_LINE] && lexer->lookahead != '\n' && !lexer->eof(lexer)) {
    consume_to_eol(lexer);
    lexer->mark_end(lexer);
    lexer->result_symbol = RAW_LINE;
    return true;
  }

  /* Mid-line. PROSE only for a text-kind tag — the decision made above.
   *
   * Note what happens when the remainder is *empty*: this branch declines and control falls through to
   * NEWLINE below. It must not `return false` there, which is what it used to do — and that bug cost
   * `fixtures/e.guml` nine ERROR nodes. `divider` and `skeleton` are text-kind tags that normally carry
   * no text, so on `divider\n` the scanner refused to produce anything at all; `_newline` is
   * external-only, so the internal lexer had nothing to fall back on and the line became an error.
   * `prose` is `optional` in the grammar precisely so a bare text tag is legal. */
  if (valid_symbols[PROSE] && s->line_is_text) {
    while (lexer->lookahead == ' ') {
      lexer->advance(lexer, true);
    }
    if (lexer->lookahead != '\n' && !lexer->eof(lexer)) {
      consume_to_eol(lexer);
      lexer->mark_end(lexer);
      lexer->result_symbol = PROSE;
      return true;
    }
  }

  if (valid_symbols[NEWLINE]) {
    while (lexer->lookahead == ' ' || lexer->lookahead == '\r') {
      lexer->advance(lexer, true);
    }
    if (lexer->lookahead == '\n') {
      lexer->advance(lexer, false);
      lexer->mark_end(lexer);
      peek_next_line(lexer, s);
      lexer->result_symbol = NEWLINE;
      return true;
    }
    /* End of file closes the last line, so a document without a trailing newline still parses. */
    if (lexer->eof(lexer)) {
      lexer->result_symbol = NEWLINE;
      return true;
    }
  }

  /* And every block still open at end of file has to be closed, or the tree ends with a MISSING dedent
   * for each level the document never dedented out of. */
  if (valid_symbols[DEDENT] && lexer->eof(lexer) && s->depth > 0) {
    s->depth--;
    lexer->result_symbol = DEDENT;
    return true;
  }

  return false;
}
