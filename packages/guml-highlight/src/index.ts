/**
 * Syntax highlighting for GUML.
 *
 * **No WebAssembly, and that is the entire point of this package.** The compiler ships its own
 * classifier — `guml_fmt::highlight`, reachable from `@guml/core` — and it is the authoritative one.
 * But reaching it means loading 787 KB of compiler wasm, asynchronously, in a browser. Highlighting a
 * snippet in a static page needs none of that: it has to run *synchronously during server rendering*,
 * and it has to work in Node, where the wasm build cannot load at all.
 *
 * So this is a hand-written tokeniser, ~15 KB, zero dependencies. What makes that safe rather than a
 * second source of truth is the parity gate: `pnpm check:highlight` runs this and the compiler's own
 * classifier over every fixture and fails on any disagreement. 936 spans across 10 documents currently
 * agree. A hand-maintained highlighter drifts silently, and this one already had — it listed `h3`,
 * which the registry does not define.
 *
 * The vocabulary is generated from `guml registry` into `vocabulary.generated.ts`, never retyped. Both
 * halves matter: the parity check catches tokenising that disagrees, and the generated vocabulary means
 * a tag added in Rust reaches this package without a second edit.
 *
 * TSX / bash / JSON are ordinary regex grammars. Nothing in the compiler describes them, so there is
 * nothing for them to drift from.
 */

import { CONTENT_TAGS, DIRECTIVES, MODIFIERS, TEXT_TAGS } from "./vocabulary.generated.ts";

/**
 * `cls` is the compiler's class name (`guml_fmt::highlight::Class::name`), not a CSS class.
 * Mapping to colour happens in `CLASS_STYLE` so the parity check can compare names.
 */
export type Tok = { text: string; cls: string };
export type Lang = "guml" | "tsx" | "python" | "bash" | "json" | "text";

/** Compiler class names. Keep in step with `Class::name` in `crates/guml-fmt`. */
const C = {
  tag: "tag",
  directive: "directive",
  mod: "modifier",
  bind: "binding",
  str: "string",
  num: "number",
  attr: "attr",
  action: "action",
  prose: "prose",
  comment: "comment",
  route: "route",
  anchor: "anchor",
  punct: "punct",
  text: "text",
  plain: "plain",
} as const;

/** The only place a class name becomes a colour. */
export const CLASS_STYLE: Record<string, string> = {
  tag: "text-syn-tag",
  directive: "text-syn-mod",
  modifier: "text-syn-mod",
  binding: "text-syn-bind",
  string: "text-syn-str",
  number: "text-syn-num",
  attr: "text-syn-key",
  action: "text-ember",
  prose: "text-chalk/70",
  comment: "text-syn-comment italic",
  route: "text-syn-key",
  anchor: "text-syn-key",
  punct: "text-syn-punct",
  text: "text-chalk/90",
  plain: "text-chalk/90",
};

function gumlLine(line: string): Tok[] {
  const out: Tok[] = [];
  const indentLen = line.length - line.trimStart().length;
  if (indentLen) out.push({ text: line.slice(0, indentLen), cls: C.plain });

  const body = line.slice(indentLen).replace(/\s+$/, "");
  if (!body) return out;
  if (body.startsWith("//")) {
    out.push({ text: body, cls: C.comment });
    return out;
  }

  /**
   * Prose keeps its bindings: the compiler interpolates them, so they are code.
   *
   * The leading gap is emitted separately because the compiler's prose span starts at the
   * first non-space byte, and the parity check compares span boundaries.
   */
  const pushProse = (text: string) => {
    const lead = text.length - text.trimStart().length;
    if (lead) out.push({ text: text.slice(0, lead), cls: C.plain });
    for (const piece of text.slice(lead).split(/(\{[^}]*\})/g)) {
      if (piece) out.push({ text: piece, cls: piece.startsWith("{") ? C.bind : C.prose });
    }
  };

  let i = 0;
  let wordIndex = 0;
  let headWord = "";

  const isWordChar = (ch: string) =>
    !(ch === " " || ch === '"' || ch === "{" || ch === "}" || ch === "|" || ch === "=" || ch === ":" || ch === "," || ch === ">" || ch === "#");

  while (i < body.length) {
    const ch = body[i];

    if (ch === " ") {
      let j = i;
      while (j < body.length && body[j] === " ") j++;
      out.push({ text: body.slice(i, j), cls: C.plain });
      i = j;
      continue;
    }

    // `>` takes the rest of the line: actions terminate a line by construction.
    if (ch === ">") {
      out.push({ text: body.slice(i), cls: C.action });
      break;
    }

    if (ch === '"') {
      let j = i + 1;
      while (j < body.length && body[j] !== '"') {
        if (body[j] === "\\") j++;
        j++;
      }
      out.push({ text: body.slice(i, Math.min(j + 1, body.length)), cls: C.str });
      i = j + 1;
      wordIndex++;
      continue;
    }

    if (ch === "{") {
      let depth = 0;
      let j = i;
      while (j < body.length) {
        if (body[j] === "{") depth++;
        else if (body[j] === "}") {
          depth--;
          if (!depth) {
            j++;
            break;
          }
        }
        j++;
      }
      out.push({ text: body.slice(i, j), cls: C.bind });
      i = j;
      wordIndex++;
      continue;
    }

    if (ch === "#" || ch === "/") {
      let j = i;
      while (j < body.length && body[j] !== " ") j++;
      out.push({ text: body.slice(i, j), cls: ch === "#" ? C.anchor : C.route });
      i = j;
      wordIndex++;
      continue;
    }

    if (ch === "|" || ch === "=" || ch === ":" || ch === ",") {
      out.push({ text: ch, cls: C.punct });
      i++;
      // On an element line everything past the bar is content, so it is never
      // re-tokenised — `full` in a sentence is a word, not a modifier.
      if (ch === "|" && !DIRECTIVES.has(headWord)) {
        pushProse(body.slice(i));
        break;
      }
      continue;
    }

    let j = i;
    while (j < body.length && isWordChar(body[j])) j++;
    const word = body.slice(i, j);

    if (wordIndex === 0) {
      headWord = word;
      // An unknown first word is still in tag position: colouring it as prose would hide
      // the typo the diagnostic is about to report.
      out.push({ text: word, cls: DIRECTIVES.has(word) ? C.directive : C.tag });

      if (TEXT_TAGS.has(word)) {
        pushProse(body.slice(j));
        return out;
      }
    } else if (body[j] === "=") {
      out.push({ text: word, cls: C.attr });
    } else if (MODIFIERS.has(word)) {
      out.push({ text: word, cls: C.mod });
    } else if (/^\d[\d.]*$/.test(word)) {
      out.push({ text: word, cls: C.num });
    } else {
      out.push({ text: word, cls: C.text });
    }

    i = j;
    wordIndex++;
  }

  return out;
}

/**
 * GUML is not line-independent: below a `tier` or `faq`, every deeper line is a *content
 * line* — raw text, however much it looks like an element. `3 projects` is prose, not the
 * tag `3` followed by a word. The compiler learns this from the registry via its nesting
 * analysis; here it needs only the one rule, tracked with a single open-block indent.
 *
 * A `js`/`raw` body is the same kind of region with one extra rule: `//` inside it is the host
 * language's comment, not GUML's, so the line is body text rather than a comment. The Rust
 * highlighter reaches the same answer through `Info::raw_text_child`, and
 * `scripts/check-highlight-parity.mjs` holds the two to a span-for-span match.
 */
function gumlDocument(lines: string[]): Tok[][] {
  let contentIndent: number | null = null;
  let inEscape = false;

  return lines.map((line) => {
    const indent = line.length - line.trimStart().length;
    const body = line.trim();

    if (contentIndent !== null && body && indent <= contentIndent) {
      contentIndent = null;
      inEscape = false;
    }

    if (contentIndent !== null && body && (inEscape || !body.startsWith("//"))) {
      const out: Tok[] = [];
      if (indent) out.push({ text: line.slice(0, indent), cls: C.plain });
      for (const piece of line.slice(indent).replace(/\s+$/, "").split(/(\{[^}]*\})/g)) {
        if (piece) out.push({ text: piece, cls: piece.startsWith("{") ? C.bind : C.prose });
      }
      return out;
    }

    if (body && !body.startsWith("//")) {
      const head = body.split(/[\s|=:]/)[0];
      // `js`/`raw` are not registry components — they are the way out of the vocabulary — so they
      // are matched by name, exactly as the parser matches them before its registry lookup.
      const escape = head === "js" || head === "raw";
      if (CONTENT_TAGS.has(head) || escape) {
        contentIndent = indent;
        inEscape = escape;
      }
    }
    return gumlLine(line);
  });
}

const TSX_RULES: Array<[RegExp, string]> = [
  [/^\/\/[^\n]*/, C.comment],
  [/^\/\*[\s\S]*?\*\//, C.comment],
  [/^"(?:[^"\\]|\\.)*"|^'(?:[^'\\]|\\.)*'|^`(?:[^`\\]|\\.)*`/, C.str],
  [
    /^\b(?:import|from|export|default|function|return|const|let|var|if|else|for|while|await|async|try|catch|finally|new|type|interface|as|useState|useEffect|useMemo)\b/,
    C.mod,
  ],
  [/^\b(?:true|false|null|undefined)\b/, C.num],
  [/^\b\d[\d_.]*\b/, C.num],
  [/^<\/?[A-Za-z][\w.-]*/, C.tag],
  [/^\b[a-zA-Z-]+(?==)/, C.attr],
  [/^[{}()[\].,;:=<>/+\-*!?&|]+/, C.punct],
  [/^\s+/, C.plain],
  [/^[^\s<>{}()[\].,;:="'`]+/, C.plain],
];

/**
 * Python. Added when the docs grew a Python page — a language the site shows must be a language it can
 * colour, and the alternative was rendering every snippet as undifferentiated text.
 *
 * Ordering carries the meaning, as in every rule list here. Strings precede identifiers, so a keyword
 * inside a string stays a string; triple-quoted precedes single, or a docstring's opening `"""` would
 * lex as an empty string followed by loose tokens; and the decorator rule precedes punctuation, so
 * `@app.get` is one token rather than an `@` and a name. `self` and `cls` sit with the keywords
 * because that is how they read, even though Python treats them as ordinary parameters.
 */
const PYTHON_RULES: Array<[RegExp, string]> = [
  [/^#[^\n]*/, C.comment],
  [/^"""[\s\S]*?"""|^'''[\s\S]*?'''/, C.str],
  [/^[rbfu]{0,2}"(?:[^"\\]|\\.)*"|^[rbfu]{0,2}'(?:[^'\\]|\\.)*'/, C.str],
  [
    /^\b(?:import|from|as|def|class|return|yield|if|elif|else|for|while|in|not|and|or|is|with|await|async|try|except|finally|raise|lambda|pass|break|continue|global|nonlocal|assert|del|self|cls|match|case)\b/,
    C.mod,
  ],
  [/^\b(?:True|False|None)\b/, C.num],
  [/^\b\d[\d_.]*(?:[eE][+-]?\d+)?\b/, C.num],
  [/^@[A-Za-z_][\w.]*/, C.attr],
  [/^\b[A-Z]\w*\b/, C.tag],
  [/^\b[a-z_]\w*(?=\s*\()/, C.attr],
  [/^[{}()[\].,;:=<>/+\-*!?&|%~^]+/, C.punct],
  [/^\s+/, C.plain],
  [/^[^\s<>{}()[\].,;:="'#@]+/, C.plain],
];

function ruleLine(line: string, rules: Array<[RegExp, string]>): Tok[] {
  const out: Tok[] = [];
  let rest = line;
  let guard = 0;
  while (rest.length && guard++ < 4000) {
    let matched = false;
    for (const [re, cls] of rules) {
      const m = re.exec(rest);
      if (m && m[0].length) {
        out.push({ text: m[0], cls });
        rest = rest.slice(m[0].length);
        matched = true;
        break;
      }
    }
    if (!matched) {
      out.push({ text: rest[0], cls: C.plain });
      rest = rest.slice(1);
    }
  }
  return out;
}

const BASH_RULES: Array<[RegExp, string]> = [
  [/^#[^\n]*/, C.comment],
  [/^"(?:[^"\\]|\\.)*"|^'(?:[^'\\]|\\.)*'/, C.str],
  [/^\b(?:cargo|guml|pnpm|npm|git|just|rustup|cd|ls)\b/, C.tag],
  [/^--?[\w-]+/, C.mod],
  [/^\|+|^&&|^>|^;/, C.punct],
  [/^\s+/, C.plain],
  [/^[^\s"'|&>;#]+/, C.plain],
];

const JSON_RULES: Array<[RegExp, string]> = [
  [/^"(?:[^"\\]|\\.)*"(?=\s*:)/, C.attr],
  [/^"(?:[^"\\]|\\.)*"/, C.str],
  [/^\b(?:true|false|null)\b/, C.mod],
  [/^-?\d[\d.eE+-]*/, C.num],
  [/^[{}[\],:]/, C.punct],
  [/^\s+/, C.plain],
  [/^[^\s{}[\],:"]+/, C.plain],
];

/** Tokenize source into lines of coloured spans. */
export function highlight(code: string, lang: Lang): Tok[][] {
  const lines = code.replace(/\n$/, "").split("\n");
  switch (lang) {
    case "guml":
      return gumlDocument(lines);
    case "tsx":
      return lines.map((l) => ruleLine(l, TSX_RULES));
    case "python":
      return lines.map((l) => ruleLine(l, PYTHON_RULES));
    case "bash":
      return lines.map((l) => ruleLine(l, BASH_RULES));
    case "json":
      return lines.map((l) => ruleLine(l, JSON_RULES));
    default:
      return lines.map((l) => [{ text: l, cls: C.plain }]);
  }
}

/**
 * Rough token estimate, matching the compiler's own `guml tokens` heuristic
 * (~3.6 chars/token). Estimates only — the real figures in the research report
 * were measured with a real tokenizer.
 */
export function approxTokens(code: string) {
  return Math.ceil(code.length / 3.6);
}
