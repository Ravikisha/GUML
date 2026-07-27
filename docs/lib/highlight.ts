/**
 * Syntax highlighting for the docs.
 *
 * The GUML highlighter is a direct port of the rules in `crates/guml-syntax`
 * (line-oriented, `>` swallows the rest of the line, `{…}` is a balanced brace
 * group, prose is never quoted). Using the compiler's own rules rather than a
 * generic highlighter means the code samples on this site tokenize exactly the
 * way the compiler does — and it keeps the site dependency-free.
 *
 * The Rust crate is the source of truth for the vocabularies below. If a tag or
 * modifier is added there, mirror it here.
 */

export type Tok = { text: string; cls: string };
export type Lang = "guml" | "tsx" | "bash" | "json" | "text";

const DIRECTIVES = new Set([
  "page",
  "type",
  "state",
  "store",
  "data",
  "route",
  "auth",
  "def",
  "js",
  "raw",
]);

const METHODS = new Set(["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"]);

const TAGS = new Set([
  "card", "row", "col", "section", "nav", "hero", "footer", "form", "tabs", "tier", "faq",
  "h", "h1", "h2", "h3", "p", "text", "metric", "head", "empty",
  "btn", "link", "check", "toggle",
  "input", "select",
  "list", "table",
]);

const MODIFIERS = new Set([
  "primary", "secondary", "outline", "ghost", "quiet", "danger", "featured",
  "xs", "sm", "md", "lg", "xl",
  "center", "start", "end", "between", "wrap", "tight", "loose", "full",
  "disabled", "loading", "readonly", "required",
]);

/** Tags whose line remainder is prose (TagKind::Text in the registry). */
const TEXT_TAGS = new Set(["h", "h1", "h2", "h3", "p", "text", "metric", "head", "empty"]);

const C = {
  tag: "text-syn-tag",
  mod: "text-syn-mod",
  bind: "text-syn-bind",
  str: "text-syn-str",
  num: "text-syn-num",
  key: "text-syn-key",
  comment: "text-syn-comment italic",
  punct: "text-syn-punct",
  action: "text-ember",
  prose: "text-chalk/70",
  plain: "text-chalk/90",
} as const;

function gumlLine(line: string): Tok[] {
  const out: Tok[] = [];
  const indentLen = line.length - line.trimStart().length;
  if (indentLen) out.push({ text: line.slice(0, indentLen), cls: C.plain });

  const body = line.slice(indentLen);
  if (!body) return out;
  if (body.startsWith("//")) {
    out.push({ text: body, cls: C.comment });
    return out;
  }

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
      out.push({ text: body.slice(i, j), cls: C.key });
      i = j;
      wordIndex++;
      continue;
    }

    if (ch === "|" || ch === "=" || ch === ":" || ch === ",") {
      out.push({ text: ch, cls: C.punct });
      i++;
      // Prose after `|` is taken raw.
      if (ch === "|") {
        out.push({ text: body.slice(i), cls: C.prose });
        break;
      }
      continue;
    }

    let j = i;
    while (j < body.length && isWordChar(body[j])) j++;
    const word = body.slice(i, j);

    if (wordIndex === 0) {
      headWord = word;
      if (DIRECTIVES.has(word)) out.push({ text: word, cls: C.mod });
      else if (TAGS.has(word)) out.push({ text: word, cls: C.tag });
      else out.push({ text: word, cls: C.tag });

      // A text tag with no `=` on the line takes the whole remainder as prose.
      if (TEXT_TAGS.has(word) && !body.slice(j).includes("=")) {
        const rest = body.slice(j);
        if (rest) {
          // Bindings inside prose still highlight.
          for (const piece of rest.split(/(\{[^}]*\})/g)) {
            if (!piece) continue;
            out.push({ text: piece, cls: piece.startsWith("{") ? C.bind : C.prose });
          }
        }
        return out;
      }
    } else if (METHODS.has(word)) {
      out.push({ text: word, cls: C.key });
    } else if (MODIFIERS.has(word) && body[j] !== "=") {
      out.push({ text: word, cls: C.mod });
    } else if (/^\$?[\d.]+/.test(word)) {
      out.push({ text: word, cls: C.num });
    } else if (body[j] === "=") {
      out.push({ text: word, cls: C.key });
    } else if (DIRECTIVES.has(headWord)) {
      out.push({ text: word, cls: C.plain });
    } else {
      out.push({ text: word, cls: C.prose });
    }

    i = j;
    wordIndex++;
  }

  return out;
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
  [/^\b[a-zA-Z-]+(?==)/, C.key],
  [/^[{}()[\].,;:=<>/+\-*!?&|]+/, C.punct],
  [/^\s+/, C.plain],
  [/^[^\s<>{}()[\].,;:="'`]+/, C.plain],
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
  [/^"(?:[^"\\]|\\.)*"(?=\s*:)/, C.key],
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
      return lines.map(gumlLine);
    case "tsx":
      return lines.map((l) => ruleLine(l, TSX_RULES));
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
