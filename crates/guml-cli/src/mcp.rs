//! `guml mcp` — the compiler as a Model Context Protocol server.
//!
//! # What this is for
//!
//! GUML has no training data by construction, so until now using it meant putting the spec and the
//! vocabulary in a system prompt: ~3,000 tokens, on every conversation, for a language the model has
//! never seen. That is the adoption tax, and it is the single largest argument against the whole idea.
//!
//! An MCP server removes it. The model asks for what it needs, when it needs it:
//!
//! ```text
//! guml_registry(["card", "btn", "list"])   -> ~180 tokens, not 3,000
//! guml_check(source)                       -> every error, with codes and suggestions
//! guml_repair(source)                      -> the fixable ones fixed, no model call
//! ```
//!
//! The second and third are the part no prompt can provide at any price: the model finds out whether
//! its output is *correct*, from the same compiler that will build it, before anyone runs it.
//!
//! # Why there is no SDK here
//!
//! `rmcp` exists and is good. It also brings tokio and an async runtime into a binary whose entire
//! dependency tree is currently 79 lines, for a protocol surface of **five methods over
//! newline-delimited JSON-RPC** — read a line, match a string, write a line. `serde_json` is already a
//! dependency because diagnostics serialise through it.
//!
//! So this is written directly: no new crates, no runtime, and `cargo deny` has nothing new to
//! license-check. The trade is that protocol changes land here by hand — acceptable for a surface this
//! small and this stable, and the conformance test speaks the wire format rather than calling internals,
//! so a change that breaks a real client breaks the test.
//!
//! # Errors are results, not protocol failures
//!
//! A document that does not compile is **not** a JSON-RPC error. It is a successful call whose result
//! says what is wrong, because the model needs to *read* that and try again. Reserving protocol errors
//! for protocol problems — unknown method, malformed params — is what keeps the two distinguishable at
//! the client.

use anyhow::Result;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

/// The revision of MCP this speaks.
///
/// Clients send their own in `initialize` and negotiate down. Echoing theirs when we understand it,
/// and stating ours when we do not, is what the spec asks for and what keeps an older client working.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// Tool definitions, in the shape `tools/list` returns.
///
/// The descriptions are written for a **model**, not for a person browsing documentation: they say
/// when to call the tool and what it costs, because that is what a caller with no knowledge of GUML
/// needs in order to choose. `guml_registry`'s description says "call this first" for the same reason.
fn tools() -> Value {
    json!([
        {
            "name": "guml_registry",
            "description":
                "The GUML component vocabulary, or a prompt-sized slice of it. CALL THIS FIRST, with \
                 the tags a task plausibly needs — a dozen tags cost ~180 tokens where the whole \
                 vocabulary costs ~3,800. Omit `tags` only when you genuinely need everything.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Tag names, e.g. [\"card\", \"btn\", \"list\"]. Omit for all 49."
                    }
                }
            }
        },
        {
            "name": "guml_spec",
            "description":
                "The GUML language rules: syntax, directives, bindings, actions. ~3,000 tokens. \
                 The vocabulary is NOT in here — that is `guml_registry`. Call this once per session \
                 before writing GUML for the first time.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "guml_check",
            "description":
                "Compile-check a GUML document. Returns EVERY problem in one pass, each with a stable \
                 code, a line, and a literal replacement where the fix is unambiguous. Call this on \
                 anything you wrote before returning it — it is the same compiler that will build it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "The GUML document." },
                    "level": {
                        "type": "string",
                        "enum": ["core", "app"],
                        "description": "`core` is markup only — no state, data, actions or js. Default `app`."
                    }
                },
                "required": ["source"]
            }
        },
        {
            "name": "guml_repair",
            "description":
                "Fix mechanically what can be fixed with no further reasoning: unwrap a code fence, \
                 normalise formatting, apply every unambiguous suggestion. Run this BEFORE spending a \
                 turn re-writing a document yourself — it is free and it resolves a real share of what \
                 a first draft gets wrong. Returns the repaired text and what remains.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "The GUML document, fenced or not." }
                },
                "required": ["source"]
            }
        },
        {
            "name": "guml_compile",
            "description":
                "Compile GUML to framework source. `html` produces a complete page needing no \
                 JavaScript and no build step; `react` produces a component. Use `level: \"core\"` when \
                 compiling a document you did not write — `js` blocks pass through unchanged.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "backend": {
                        "type": "string",
                        "description": "react, html, html-fragment, svelte, wc, json, a2ui, mcp-ui. Default react."
                    },
                    "level": { "type": "string", "enum": ["core", "app"] }
                },
                "required": ["source"]
            }
        }
    ])
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn registry_for(level: Option<&str>) -> Result<guml_registry::Registry> {
    match level {
        None | Some("app") => Ok(guml_registry::Registry::builtin()),
        Some("core") => Ok(guml_registry::Registry::core()),
        Some(other) => anyhow::bail!("unknown level {other:?}: expected \"core\" or \"app\""),
    }
}

/// Render diagnostics for a *model* to act on.
///
/// One line per problem, with the code first. A model reading this needs the code (stable, and the
/// thing to look up), the line, and the suggestion; the caret art the terminal renderer produces is
/// noise in a tool result and costs tokens.
fn render_diagnostics(diags: &guml_diagnostics::Diagnostics) -> String {
    if diags.items.is_empty() {
        return "COMPILES. No problems.".to_string();
    }

    // **The verdict first, on its own line.** A model handed a list of warnings and left to infer
    // whether the document is usable will usually infer wrong and start rewriting something that was
    // already correct. Warnings are advice; only errors mean it did not compile, and the difference
    // has to be stated rather than implied by the word "warning" appearing in a list.
    let mut out = if diags.has_errors() {
        let n =
            diags.items.iter().filter(|d| d.severity == guml_diagnostics::Severity::Error).count();
        format!("DOES NOT COMPILE — {n} error(s). Fix these and check again.\n\n")
    } else {
        "COMPILES, with advisory warnings. The document is usable as-is.\n\n".to_string()
    };

    for d in &diags.items {
        let severity = format!("{:?}", d.severity).to_lowercase();
        out.push_str(&format!("{} [{}] line {}: {}\n", severity, d.id, d.span.line, d.message));
        if let Some(help) = &d.help {
            out.push_str(&format!("    help: {help}\n"));
        }
        if let Some(s) = &d.suggestion {
            out.push_str(&format!("    replace with: {s}\n"));
        }
    }
    out
}

/// Run one tool. `Err` means the *call* was malformed; a document that does not compile is `Ok`.
fn call_tool(name: &str, args: &Value) -> Result<String> {
    match name {
        "guml_spec" => Ok(guml_registry::SPEC.to_string()),

        "guml_registry" => {
            let reg = guml_registry::Registry::builtin();
            let requested: Vec<String> = args
                .get("tags")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default();

            // No `tags` means the whole vocabulary. An empty slice would be an empty prompt — the
            // model would be told the language has no words and would invent some.
            let names: Vec<String> = if requested.is_empty() {
                reg.names().map(str::to_string).collect()
            } else {
                requested
            };

            let unknown: Vec<&String> = names.iter().filter(|n| reg.get(n).is_none()).collect();
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            let mut out = reg.prompt_context(&refs);

            // A tag the caller asked for that does not exist is worth saying, with the near-miss the
            // compiler would have suggested. Silence here reads as "that tag exists and has no docs".
            if !unknown.is_empty() {
                out.push_str("\nNot in the vocabulary:\n");
                for name in unknown {
                    match reg.suggest(name) {
                        Some(near) => out.push_str(&format!("  {name} — did you mean `{near}`?\n")),
                        None => out.push_str(&format!("  {name}\n")),
                    }
                }
            }
            Ok(out)
        }

        "guml_check" => {
            let source =
                str_arg(args, "source").ok_or_else(|| anyhow::anyhow!("`source` is required"))?;
            let reg = registry_for(str_arg(args, "level"))?;
            let (_, diags) = guml_compiler::check_with(source, &reg);
            Ok(render_diagnostics(&diags))
        }

        "guml_repair" => {
            let source =
                str_arg(args, "source").ok_or_else(|| anyhow::anyhow!("`source` is required"))?;
            let repaired = guml_compiler::repair::repair(source, 3);
            let (_, diags) = guml_compiler::check(&repaired.text);

            let mut out = String::new();
            if repaired.applied.is_empty() && !repaired.reformatted {
                out.push_str("Nothing was mechanically fixable.\n\n");
            } else {
                out.push_str(&format!(
                    "Applied {} fix(es){}.\n\n",
                    repaired.applied.len(),
                    if repaired.reformatted { " and reformatted" } else { "" }
                ));
            }
            out.push_str("--- repaired document ---\n");
            out.push_str(&repaired.text);
            out.push_str("\n--- remaining ---\n");
            out.push_str(&render_diagnostics(&diags));
            Ok(out)
        }

        "guml_compile" => {
            let source =
                str_arg(args, "source").ok_or_else(|| anyhow::anyhow!("`source` is required"))?;
            let backend = str_arg(args, "backend").unwrap_or("react");
            if guml_codegen::backend(backend).is_none() {
                anyhow::bail!(
                    "unknown backend {backend:?}: expected one of {}",
                    guml_codegen::backend_names().join(", ")
                );
            }
            let reg = registry_for(str_arg(args, "level"))?;
            let opts = guml_compiler::Options { backend: backend.to_string(), registry: reg };
            let result = guml_compiler::compile(source, &opts);

            // Errors first. A model handed 200 lines of output with a diagnostic buried underneath
            // will use the output; putting the refusal at the top is what makes it act on it.
            if result.diagnostics.has_errors() {
                return Ok(format!(
                    "Did not compile. Fix these and call again:\n\n{}",
                    render_diagnostics(&result.diagnostics)
                ));
            }

            let mut out = String::new();
            for f in &result.files {
                out.push_str(&format!("--- {} ---\n{}\n", f.path, f.contents));
            }
            if !result.diagnostics.items.is_empty() {
                out.push_str("\n--- warnings ---\n");
                out.push_str(&render_diagnostics(&result.diagnostics));
            }
            Ok(out)
        }

        other => anyhow::bail!("unknown tool {other:?}"),
    }
}

fn result(id: Value, value: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": value })
}

fn error(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Handle one message. `None` means "no reply" — correct for a notification, where sending one is a
/// protocol violation rather than merely unnecessary.
pub fn handle(message: &Value) -> Option<Value> {
    let method = message.get("method").and_then(Value::as_str).unwrap_or_default();
    let id = message.get("id").cloned();

    // A notification has no `id` and must never be answered. `?` returns `None`, which is the
    // "send nothing" signal the caller acts on.
    let id = id?;

    match method {
        "initialize" => {
            // Echo the client's version when we understand it; otherwise state ours and let it decide.
            // Insisting on ours would break an older client for no reason.
            let requested = message
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(Value::as_str);
            let version = match requested {
                Some(v) if v == PROTOCOL_VERSION => v,
                _ => PROTOCOL_VERSION,
            };
            Some(result(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "guml", "version": env!("CARGO_PKG_VERSION") }
                }),
            ))
        }

        "tools/list" => Some(result(id, json!({ "tools": tools() }))),

        "tools/call" => {
            let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

            Some(match call_tool(name, &args) {
                Ok(text) => result(id, json!({ "content": [{ "type": "text", "text": text }] })),
                // A malformed call comes back as an `isError` tool result rather than a JSON-RPC
                // error, because the model is the one that has to correct it and a protocol error
                // may never reach it.
                Err(e) => result(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": e.to_string() }],
                        "isError": true
                    }),
                ),
            })
        }

        "ping" => Some(result(id, json!({}))),

        _ => Some(error(id, -32601, &format!("method not found: {method}"))),
    }
}

/// Serve on stdio until the client closes it.
///
/// Newline-delimited JSON, which is what MCP's stdio transport specifies — not the `Content-Length`
/// framing LSP uses. Every reply is flushed immediately: a client waiting on a response that is
/// sitting in a buffer looks exactly like a server that has hung.
///
/// **Nothing may be written to stdout except protocol messages.** A stray `println!` anywhere in the
/// compiler would corrupt the stream, which is why diagnostics go through `render_diagnostics` into a
/// result rather than being printed.
pub fn serve() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let message: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                // Parse errors carry no id, so the reply carries null — the one case where that is
                // correct rather than lazy.
                let reply = error(Value::Null, -32700, &format!("parse error: {e}"));
                writeln!(stdout, "{reply}")?;
                stdout.flush()?;
                continue;
            }
        };

        if let Some(reply) = handle(&message) {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
