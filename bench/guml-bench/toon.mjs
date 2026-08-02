/**
 * A TOON-shaped encoder, for GUML-Bench arm **B4**.
 *
 * # Why this arm exists, and why it is the one that could embarrass the project
 *
 * The first objection any reviewer raises to a UI IR is "why not just emit JSON", and arm B3 answers it
 * with a real A2UI-shaped payload the compiler emits itself. The *second* objection is sharper: JSON is a
 * verbose serialisation, so maybe the win is not the language at all but the encoding — use a compact
 * serialisation of the same structure and the gap closes. TOON (Token-Oriented Object Notation) is the
 * strongest version of that objection, which is why the report names it as an arm rather than a footnote.
 *
 * Answering it honestly means encoding **the same IR** as B3 and nothing else. If B4 were given a
 * hand-tuned structure the other arms did not get, the comparison would measure the tuning.
 *
 * # What "TOON-shaped" means, and why the hedge is in the name
 *
 * Implemented from the format's design, not validated against a reference implementation, so it carries
 * the same hedge as the compiler's own `"a2ui-shaped"` output. The four properties that matter for a token
 * count are all here:
 *
 *   1. **Indentation instead of braces.** Every `{`, `}` and the comma after every field is gone.
 *   2. **Tabular uniform arrays.** An array of objects sharing a key set declares those keys *once* in a
 *      header and then emits one delimited row per element. This is where the format wins, and it is the
 *      whole reason the arm is a serious rival rather than a straw man.
 *   3. **Declared lengths.** `[N]` after a key, which is what lets a decoder — or a model — know a row
 *      count without scanning, and is TOON's own argument for why it is *more* reliable than JSON rather
 *      than merely smaller.
 *   4. **Minimal quoting.** A scalar is bare unless it would be ambiguous.
 *
 * What is *not* here: key folding, and any alternate delimiter. Both are real TOON features and both would
 * make these numbers smaller. So this encoder is a **lower bound on how well TOON does** — which is the
 * safe direction for a competitor's arm to be wrong in, and it is stated here so nobody has to guess.
 *
 * # The finding, before you run it
 *
 * TOON's tabular win applies to a *uniform* array. The A2UI-shaped IR's `components` array is not uniform —
 * a `head` node has `text`, a `form` has `children` and `intents`, an `input` has `bind` and `properties` —
 * so it falls back to list form and the saving comes only from dropping punctuation. `report.mjs` prints
 * the uniformity rate beside the number, because "TOON is only 30% smaller here" is a claim about *this
 * IR's shape*, not about TOON, and stating it the other way round would be unfair to the format.
 */

/** Characters that force a scalar to be quoted, because bare they would be read as structure. */
const NEEDS_QUOTE = /[",:[\]{}\n]|^\s|\s$|^$|^-$/;

function scalar(value) {
  if (value === null) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") return Number.isFinite(value) ? String(value) : "null";
  const text = String(value);
  // A bare token that would parse as something else has to be quoted, or the encoding is lossy: the
  // string "true" and the boolean `true` must not both come out as `true`.
  const ambiguous =
    NEEDS_QUOTE.test(text) ||
    text === "true" ||
    text === "false" ||
    text === "null" ||
    (text !== "" && !Number.isNaN(Number(text)));
  return ambiguous ? JSON.stringify(text) : text;
}

const isPlainObject = (v) => v !== null && typeof v === "object" && !Array.isArray(v);

/**
 * Whether an array can use the tabular form: at least two elements, every one a flat object, all with the
 * same keys in the same order, and no value that is itself a container.
 *
 * Strict on purpose. A "mostly uniform" array encoded tabularly would need a hole marker per missing
 * field, and inventing one would make this encoder's output something no TOON decoder reads — a made-up
 * dialect that happens to score well is the one result this arm must not produce.
 */
export function tabularKeys(array) {
  if (!Array.isArray(array) || array.length < 2) return null;
  if (!array.every(isPlainObject)) return null;
  const keys = Object.keys(array[0]);
  if (keys.length === 0) return null;
  for (const row of array) {
    const rowKeys = Object.keys(row);
    if (rowKeys.length !== keys.length) return null;
    if (rowKeys.some((k, i) => k !== keys[i])) return null;
    if (keys.some((k) => row[k] !== null && typeof row[k] === "object")) return null;
  }
  return keys;
}

function emit(value, key, depth, out) {
  const pad = "  ".repeat(depth);
  const label = key === null ? "" : `${scalar(key)}`;

  if (Array.isArray(value)) {
    if (value.length === 0) {
      out.push(`${pad}${label}[0]:`);
      return;
    }
    // Scalars inline: `roots[4]: n0,n1,n4,n5`.
    if (value.every((v) => v === null || typeof v !== "object")) {
      out.push(`${pad}${label}[${value.length}]: ${value.map(scalar).join(",")}`);
      return;
    }
    const keys = tabularKeys(value);
    if (keys) {
      out.push(`${pad}${label}[${value.length}]{${keys.join(",")}}:`);
      const inner = "  ".repeat(depth + 1);
      for (const row of value) {
        out.push(`${inner}${keys.map((k) => scalar(row[k])).join(",")}`);
      }
      return;
    }
    // Non-uniform: one list entry per element, which is where this IR lands.
    out.push(`${pad}${label}[${value.length}]:`);
    for (const item of value) {
      if (isPlainObject(item)) {
        const entries = Object.entries(item);
        const inner = "  ".repeat(depth + 1);
        // `- ` on the first field, so an element boundary is visible without a delimiter.
        entries.forEach(([k, v], i) => {
          if (i === 0 && v !== null && typeof v !== "object") {
            out.push(`${inner}- ${scalar(k)}: ${scalar(v)}`);
          } else if (i === 0) {
            out.push(`${inner}- ${scalar(k)}:`);
            emitInto(v, null, depth + 3, out);
          } else {
            emit(v, k, depth + 2, out);
          }
        });
      } else {
        out.push(`${"  ".repeat(depth + 1)}- ${scalar(item)}`);
      }
    }
    return;
  }

  if (isPlainObject(value)) {
    out.push(`${pad}${label}:`);
    for (const [k, v] of Object.entries(value)) emit(v, k, depth + 1, out);
    return;
  }

  out.push(`${pad}${label}: ${scalar(value)}`);
}

function emitInto(value, key, depth, out) {
  if (Array.isArray(value) || isPlainObject(value)) {
    if (Array.isArray(value)) {
      emit(value, key ?? "items", depth, out);
    } else {
      for (const [k, v] of Object.entries(value)) emit(v, k, depth, out);
    }
  } else {
    out.push(`${"  ".repeat(depth)}${scalar(value)}`);
  }
}

/** Encode a JSON-compatible value as TOON-shaped text. */
export function encode(value) {
  const out = [];
  if (isPlainObject(value)) {
    for (const [k, v] of Object.entries(value)) emit(v, k, 0, out);
  } else {
    emit(value, null, 0, out);
  }
  return `${out.join("\n")}\n`;
}

/**
 * How much of a payload TOON's tabular form actually reaches.
 *
 * Reported alongside the token count because it is the difference between "TOON saves 30% on this IR" and
 * "TOON saves 30%". The first is true and the second is not a claim this harness can make.
 */
export function uniformity(value) {
  let arrays = 0;
  let tabular = 0;
  let rowsTotal = 0;
  let rowsTabular = 0;
  const walk = (v) => {
    if (Array.isArray(v)) {
      if (v.some(isPlainObject)) {
        arrays++;
        rowsTotal += v.length;
        if (tabularKeys(v)) {
          tabular++;
          rowsTabular += v.length;
        }
      }
      v.forEach(walk);
      return;
    }
    if (isPlainObject(v)) Object.values(v).forEach(walk);
  };
  walk(value);
  return {
    objectArrays: arrays,
    tabularArrays: tabular,
    rows: rowsTotal,
    tabularRows: rowsTabular,
    tabularRowShare: rowsTotal === 0 ? null : Number((rowsTabular / rowsTotal).toFixed(3)),
  };
}

/* --------------------------------------------------------------------- decoding */

/**
 * Decode TOON-shaped text back to a value.
 *
 * # Why an encoder for a rival arm ships with a decoder
 *
 * Without one, "TOON is 31% smaller" is indistinguishable from "we deleted 31% of the characters". A
 * serialisation that cannot be read back is not a serialisation, and a comparison against one is not a
 * comparison — it is the strongest possible version of rigging the arm we are supposed to be steelmanning.
 *
 * So `selftest.mjs` encodes every payload the harness measures, decodes it, and asserts deep equality
 * against the original. That is the claim: **on all twelve A2UI payloads, the encoding is lossless.** If a
 * document ever breaks it, the number is wrong and the test says so rather than the number quietly being
 * optimistic.
 *
 * Covers exactly the subset `encode` produces. Anything else throws rather than guessing, because a decoder
 * that recovers *something* from input it does not understand would let a lossy encoding pass the very test
 * it exists to fail.
 */
export function decode(text) {
  const lines = text.split("\n").filter((l) => l.trim() !== "");
  let i = 0;

  const indentOf = (line) => line.length - line.trimStart().length;

  function unscalar(token) {
    const t = token.trim();
    if (t.startsWith('"')) return JSON.parse(t);
    if (t === "true") return true;
    if (t === "false") return false;
    if (t === "null") return null;
    if (t !== "" && !Number.isNaN(Number(t))) return Number(t);
    return t;
  }

  // `a,b,"c,d"` — split on commas that are not inside a quoted run.
  function splitRow(text) {
    const out = [];
    let cur = "";
    let quoted = false;
    for (let p = 0; p < text.length; p++) {
      const ch = text[p];
      if (quoted) {
        cur += ch;
        if (ch === "\\") {
          cur += text[++p] ?? "";
        } else if (ch === '"') {
          quoted = false;
        }
        continue;
      }
      if (ch === '"') {
        quoted = true;
        cur += ch;
      } else if (ch === ",") {
        out.push(cur);
        cur = "";
      } else {
        cur += ch;
      }
    }
    out.push(cur);
    return out;
  }

  /** `key[3]{a,b}:` / `key[3]: x,y,z` / `key: v` / `key:` */
  function parseHead(line) {
    const body = line.trimStart().replace(/^- /, "");
    const m = body.match(/^("(?:[^"\\]|\\.)*"|[^:[\]]*?)(?:\[(\d+)\])?(?:\{([^}]*)\})?:(.*)$/);
    if (!m) throw new Error(`toon: cannot parse \`${line}\``);
    return {
      key: m[1] === "" ? null : unscalar(m[1]),
      count: m[2] === undefined ? null : Number(m[2]),
      fields: m[3] === undefined ? null : m[3].split(","),
      rest: m[4],
      dash: /^\s*- /.test(line),
    };
  }

  /** Every entry at exactly `indent`, as an object. */
  function readObject(indent) {
    const obj = {};
    while (i < lines.length && indentOf(lines[i]) === indent) {
      const head = parseHead(lines[i]);
      i++;
      obj[head.key] = readValue(head, indent);
    }
    return obj;
  }

  function readValue(head, indent) {
    if (head.count !== null) {
      if (head.count === 0) return [];
      if (head.rest.trim() !== "") {
        return splitRow(head.rest.trim()).map(unscalar);
      }
      if (head.fields) {
        const rows = [];
        for (let n = 0; n < head.count; n++) {
          const cells = splitRow(lines[i].trim()).map(unscalar);
          i++;
          rows.push(Object.fromEntries(head.fields.map((f, c) => [f, cells[c]])));
        }
        return rows;
      }
      // Non-uniform list: each element opens with `- ` at indent + 2.
      const items = [];
      const inner = indent + 2;
      while (i < lines.length && indentOf(lines[i]) === inner) {
        const first = parseHead(lines[i]);
        if (!first.dash) throw new Error(`toon: expected a list element at \`${lines[i]}\``);
        i++;
        const item = {};
        item[first.key] = readValue(first, inner);
        // Remaining fields of this element sit one level deeper, which is the same column the `- `
        // occupies — so `indentOf` distinguishes them from the next element by the dash alone.
        while (i < lines.length && indentOf(lines[i]) === inner + 2) {
          const f = parseHead(lines[i]);
          i++;
          item[f.key] = readValue(f, inner + 2);
        }
        items.push(item);
      }
      return items;
    }
    if (head.rest.trim() === "") {
      // A nested object, or a bare `key:` with nothing under it.
      if (i < lines.length && indentOf(lines[i]) > indent) return readObject(indent + 2);
      return {};
    }
    return unscalar(head.rest);
  }

  return readObject(0);
}
