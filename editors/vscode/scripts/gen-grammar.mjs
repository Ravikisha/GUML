#!/usr/bin/env node
/**
 * Generate the TextMate grammar from the compiler's registry.
 *
 * # Why a TextMate grammar exists at all
 *
 * The language server is the real highlighter — it consults the registry, so it knows that the
 * remainder of a `p` line is prose and not four modifiers. A regex grammar cannot know that.
 *
 * But an editor colours a file the instant it opens, and the server attaches a moment later.
 * Without this, every `.guml` file flashes plain white first. So this is deliberately the
 * *weaker* highlighter, used for a few hundred milliseconds and then superseded by semantic
 * tokens.
 *
 * The vocabulary is read from `guml registry`, never typed here. A hand-maintained tag list in
 * an editor grammar is a copy that drifts, and this project has already fixed that exact bug
 * once in the docs site.
 *
 *   node scripts/gen-grammar.mjs
 */
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const EXT = resolve(HERE, "..");
const ROOT = resolve(EXT, "..", "..");

function registry() {
  const raw = execFileSync("cargo", ["run", "-q", "-p", "guml-cli", "--", "registry"], {
    cwd: ROOT,
    encoding: "utf8",
  });
  const tags = [];
  let modifiers = [];
  for (const line of raw.split(/\r?\n/)) {
    if (line.startsWith("modifiers:")) {
      modifiers = line.replace("modifiers:", "").trim().split(/\s+/);
      continue;
    }
    const m = /^(\S+)\s+(\S+)\s+/.exec(line.trim());
    if (m) tags.push({ name: m[1], kind: m[2] });
  }
  return { tags, modifiers };
}

const { tags, modifiers } = registry();
const textTags = tags.filter((t) => t.kind === "Text").map((t) => t.name);
const otherTags = tags.filter((t) => t.kind !== "Text").map((t) => t.name);
const DIRECTIVES = ["page", "type", "state", "store", "data", "route", "auth", "def", "js", "raw"];

/** Longest first, so `h1` is not matched as `h` followed by `1`. */
const alt = (words) => [...words].sort((a, b) => b.length - a.length).join("|");

const grammar = {
  $schema:
    "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
  name: "GUML",
  scopeName: "source.guml",
  // GENERATED — see scripts/gen-grammar.mjs. Do not edit by hand.
  patterns: [
    { include: "#comment" },
    { include: "#directive" },
    { include: "#textTag" },
    { include: "#element" },
  ],
  repository: {
    comment: {
      match: "^\\s*//.*$",
      name: "comment.line.double-slash.guml",
    },

    directive: {
      // `page Tasks`, `state filter=all|open|done`, `data tasks:Task[] GET /api/tasks`
      begin: `^\\s*(${alt(DIRECTIVES)})\\b`,
      beginCaptures: { 1: { name: "keyword.control.directive.guml" } },
      end: "$",
      patterns: [
        { include: "#string" },
        { include: "#binding" },
        { include: "#method" },
        { include: "#route" },
        { include: "#number" },
        { match: "\\b[A-Z][A-Za-z0-9]*\\b", name: "entity.name.type.guml" },
        { match: "[=|:,\\[\\]{}]", name: "punctuation.separator.guml" },
      ],
    },

    // A text tag takes the rest of the line as prose. This is the rule a regex grammar gets
    // wrong without the registry: `p Press the center button` has no modifiers in it.
    textTag: {
      begin: `^\\s*(${alt(textTags)})\\b`,
      beginCaptures: { 1: { name: "entity.name.tag.text.guml" } },
      end: "$",
      patterns: [
        { include: "#binding" },
        { match: ".", name: "string.unquoted.prose.guml" },
      ],
    },

    element: {
      begin: `^\\s*(${alt(otherTags)})\\b`,
      beginCaptures: { 1: { name: "entity.name.tag.guml" } },
      end: "$",
      patterns: [
        { include: "#action" },
        { include: "#content" },
        { include: "#string" },
        { include: "#binding" },
        { include: "#attribute" },
        { include: "#anchor" },
        { include: "#route" },
        { include: "#modifier" },
        { include: "#number" },
      ],
    },

    // `>` swallows the rest of the line by construction, which makes it the one thing a regex
    // grammar can get exactly right.
    action: { begin: ">", end: "$", name: "meta.function-call.action.guml" },

    // Everything after `|` is content.
    content: {
      begin: "\\|",
      beginCaptures: { 0: { name: "punctuation.separator.content.guml" } },
      end: "$",
      patterns: [{ include: "#binding" }, { match: ".", name: "string.unquoted.prose.guml" }],
    },

    binding: { begin: "\\{", end: "\\}", name: "variable.other.binding.guml" },
    string: { begin: '"', end: '"', name: "string.quoted.double.guml" },
    attribute: {
      match: "\\b([a-zA-Z][\\w-]*)(=)",
      captures: {
        1: { name: "entity.other.attribute-name.guml" },
        2: { name: "punctuation.separator.guml" },
      },
    },
    modifier: { match: `\\b(${alt(modifiers)})\\b`, name: "support.type.modifier.guml" },
    anchor: { match: "#[\\w-]+", name: "entity.name.label.guml" },
    route: { match: "(?<=\\s)/[^\\s]*", name: "markup.underline.link.guml" },
    method: { match: "\\b(GET|POST|PUT|PATCH|DELETE|HEAD)\\b", name: "keyword.other.method.guml" },
    number: { match: "\\b\\d[\\d.]*\\b", name: "constant.numeric.guml" },
  },
};

mkdirSync(join(EXT, "syntaxes"), { recursive: true });
writeFileSync(
  join(EXT, "syntaxes", "guml.tmLanguage.json"),
  `${JSON.stringify(grammar, null, 2)}\n`,
);
console.log(
  `wrote syntaxes/guml.tmLanguage.json (${tags.length} tags, ${textTags.length} of them prose, ` +
    `${modifiers.length} modifiers) — from the compiler's registry`,
);
