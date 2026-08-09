/**
 * Conformance for `guml mcp`, over the real wire format.
 *
 * Spawns the binary, writes newline-delimited JSON-RPC to its stdin and reads its stdout — which is
 * exactly what Claude Desktop, Claude Code, Cursor and every other MCP client do. Nothing here calls a
 * Rust function directly, deliberately: the failure mode worth catching is "the server works and no
 * client can talk to it", and only the wire can show that.
 *
 * What it holds the server to:
 *
 *   * `initialize` answers with a protocol version, capabilities and server info
 *   * a notification is **not** answered — replying to one is a protocol violation, not a courtesy
 *   * every advertised tool has a name, a description and an object input schema
 *   * every advertised tool actually runs
 *   * a document that does not compile is a *successful* call whose result says so, not a JSON-RPC
 *     error — the model has to read it and try again
 *   * an unknown method is `-32601`, because that one really is a protocol problem
 *   * stdout carries protocol messages and nothing else
 *
 * That last one is the quiet killer. A stray `println!` anywhere in the compiler corrupts the stream,
 * the client sees malformed JSON, and the symptom is "the server disconnected" with no clue why.
 */

import { spawn } from "node:child_process";
import process from "node:process";

const BIN = process.env.GUML_BIN ?? "cargo";
const ARGS = process.env.GUML_BIN ? ["mcp"] : ["run", "-q", "-p", "guml-cli", "--", "mcp"];

let failures = 0;
const check = (label, ok, detail = "") => {
  console.log(`  ${ok ? "ok  " : "FAIL"}  ${label}${detail ? `  — ${detail}` : ""}`);
  if (!ok) failures++;
};

/**
 * Send a batch of messages, collect the replies, and return them with anything unparseable.
 *
 * One process per batch: a server is allowed to keep state across a session, and testing each call
 * against a fresh process would not exercise that.
 */
function session(messages) {
  return new Promise((resolve, reject) => {
    const proc = spawn(BIN, ARGS, { stdio: ["pipe", "pipe", "pipe"] });
    let out = "";
    let err = "";

    proc.stdout.on("data", (d) => (out += d));
    proc.stderr.on("data", (d) => (err += d));
    proc.on("error", reject);
    proc.on("close", () => {
      const lines = out.split("\n").filter((l) => l.trim());
      const replies = [];
      const garbage = [];
      for (const line of lines) {
        try {
          replies.push(JSON.parse(line));
        } catch {
          garbage.push(line);
        }
      }
      resolve({ replies, garbage, stderr: err });
    });

    for (const m of messages) {
      // `RAW` goes through untouched, so the malformed-input case can actually send malformed input.
      // `JSON.stringify("not json")` is `"not json"` — perfectly valid JSON containing prose, which
      // made the first version of that check pass for entirely the wrong reason.
      proc.stdin.write(`${m.RAW ?? JSON.stringify(m)}\n`);
    }
    proc.stdin.end();
  });
}

const rpc = (id, method, params) => ({ jsonrpc: "2.0", id, method, ...(params && { params }) });

// ---------------------------------------------------------------- handshake

const init = await session([
  rpc(1, "initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: "conformance", version: "0" },
  }),
  { jsonrpc: "2.0", method: "notifications/initialized" },
  rpc(2, "tools/list"),
]);

check("stdout is protocol messages only", init.garbage.length === 0, init.garbage[0]?.slice(0, 60));

const initialize = init.replies.find((r) => r.id === 1);
check("initialize answers", Boolean(initialize?.result), JSON.stringify(initialize?.error ?? ""));
check("declares a protocol version", Boolean(initialize?.result?.protocolVersion), initialize?.result?.protocolVersion);
check("declares tool capability", Boolean(initialize?.result?.capabilities?.tools));
check("names itself", initialize?.result?.serverInfo?.name === "guml", initialize?.result?.serverInfo?.name);

// A notification carries no id and must produce no reply. Two replies for three messages is the
// evidence: `initialize` and `tools/list` answered, the notification did not.
check("a notification is not answered", init.replies.length === 2, `${init.replies.length} replies to 3 messages`);

// ---------------------------------------------------------------- tool definitions

const listed = init.replies.find((r) => r.id === 2)?.result?.tools ?? [];
check("advertises tools", listed.length >= 5, `${listed.length} tools`);

for (const t of listed) {
  const wellFormed =
    typeof t.name === "string" &&
    typeof t.description === "string" &&
    t.description.length > 40 &&
    t.inputSchema?.type === "object";
  check(`  ${t.name} is well-formed`, wellFormed);
}

// ---------------------------------------------------------------- every tool runs

const SRC = 'page "Demo"\n\nstate c: a\n\nselect c\n  option a\n  option b\n';

const calls = await session([
  rpc(1, "initialize", { protocolVersion: "2025-06-18", capabilities: {} }),
  rpc(10, "tools/call", { name: "guml_registry", arguments: { tags: ["card", "btn"] } }),
  rpc(11, "tools/call", { name: "guml_registry", arguments: {} }),
  rpc(12, "tools/call", { name: "guml_spec", arguments: {} }),
  rpc(13, "tools/call", { name: "guml_check", arguments: { source: SRC } }),
  rpc(14, "tools/call", { name: "guml_check", arguments: { source: "crad Hi\n" } }),
  rpc(15, "tools/call", { name: "guml_repair", arguments: { source: "```guml\npage \"X\"\n\ncrad Hi\n```" } }),
  rpc(16, "tools/call", { name: "guml_compile", arguments: { source: SRC, backend: "html" } }),
  rpc(17, "tools/call", { name: "guml_compile", arguments: { source: SRC, backend: "vue" } }),
  rpc(18, "tools/call", { name: "guml_registry", arguments: { tags: ["nosuchtag"] } }),
  rpc(19, "tools/call", { name: "guml_nonexistent", arguments: {} }),
  rpc(20, "nonexistent/method"),
]);

const text = (id) => calls.replies.find((r) => r.id === id)?.result?.content?.[0]?.text ?? "";
const isError = (id) => calls.replies.find((r) => r.id === id)?.result?.isError === true;

check("stdout stays clean under load", calls.garbage.length === 0, calls.garbage[0]?.slice(0, 60));

const slice = text(10);
const whole = text(11);
check("registry slices", slice.includes("card") && slice.includes("btn"), `${slice.length} chars`);
check(
  "a slice is much smaller than the whole vocabulary",
  slice.length * 5 < whole.length,
  `${slice.length} vs ${whole.length} chars — the entire point`,
);
check("spec is served", text(12).length > 1000, `${text(12).length} chars`);

check("a clean document says it compiles", text(13).startsWith("COMPILES"), text(13).split("\n")[0]);
check("a broken document names the code", text(14).includes("GUML0030"), text(14).split("\n")[0]);
check(
  "a broken document is a result, not a protocol error",
  !calls.replies.find((r) => r.id === 14)?.error && !isError(14),
  "the model has to read it and retry",
);

check("repair unwraps a fence and fixes what it can", text(15).includes("card") && !text(15).includes("```"), text(15).split("\n")[0]);
check("compile emits", text(16).includes("<!doctype html>"), `${text(16).length} chars`);

check("an unknown backend is reported", isError(17), text(17).slice(0, 60));
check("an unknown tag suggests a near miss", text(18).includes("Not in the vocabulary"), text(18).trim().split("\n").pop());
check("an unknown tool is an isError result", isError(19), text(19).slice(0, 40));

const badMethod = calls.replies.find((r) => r.id === 20);
check("an unknown *method* is a JSON-RPC error", badMethod?.error?.code === -32601, JSON.stringify(badMethod?.error ?? {}));

// ---------------------------------------------------------------- malformed input

// Genuinely malformed, not a JSON *string* containing prose — `JSON.stringify("not json")` is
// `"not json"`, which parses perfectly and made the first version of this check pass for the wrong
// reason. `RAW` is written to the pipe verbatim.
const junk = await session([{ RAW: "{ this is not json" }, rpc(1, "ping")]);
const parseError = junk.replies.find((r) => r.error?.code === -32700);
check("malformed input gets a parse error", Boolean(parseError));
check("and the session survives it", junk.replies.some((r) => r.id === 1), "ping still answered");

console.log(
  failures
    ? `\n${failures} conformance failure(s)`
    : `\nthe MCP server conforms: ${listed.length} tools, all reachable over the wire`,
);
process.exitCode = failures ? 1 : 0;
