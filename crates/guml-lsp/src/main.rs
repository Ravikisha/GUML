//! `guml-lsp` — the language server.
//!
//! Deliberately thin. Every answer comes from [`features`], which is plain functions over text
//! and is tested without a client; this file is translation between those and the protocol.
//! The split matters because a language server is otherwise the hardest part of a toolchain to
//! test, and an untested one that returns confident wrong positions is worse than none.
//!
//! Nothing here re-implements the language. Diagnostics are `guml_compiler::check`, highlighting
//! is the compiler's classifier, formatting is `guml fmt`. A human and a model therefore see the
//! *same* errors, which is the property the whole diagnostic design exists to protect.

mod features;
mod navigate;

use dashmap::DashMap;
use guml_diagnostics::Severity;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    /// Open documents, by URI. The client owns the buffer; this is the server's copy of what it
    /// last said, which is all a stateless analysis needs.
    docs: DashMap<Url, String>,
}

impl Backend {
    fn text(&self, uri: &Url) -> String {
        self.docs.get(uri).map(|d| d.clone()).unwrap_or_default()
    }

    /// Analyse and publish. Called on open and on every change: `check` is measured at 1.77 ms
    /// on 200 lines, so there is no debounce here to get wrong.
    async fn publish(&self, uri: Url) {
        let src = self.text(&uri);
        let diagnostics = features::diagnostics(&src)
            .into_iter()
            .map(|d| Diagnostic {
                range: Range {
                    start: Position::new(d.start.line, d.start.character),
                    end: Position::new(d.end.line, d.end.character),
                },
                severity: Some(match d.severity {
                    Severity::Error => DiagnosticSeverity::ERROR,
                    Severity::Warning => DiagnosticSeverity::WARNING,
                    Severity::Note => DiagnosticSeverity::INFORMATION,
                }),
                code: Some(NumberOrString::String(d.code.clone())),
                source: Some("guml".into()),
                // The help line carries the actionable half of the message, so it belongs in
                // what the editor shows rather than in a detail nobody opens.
                message: match &d.help {
                    Some(help) => format!("{}\n\n{help}", d.message),
                    None => d.message.clone(),
                },
                ..Default::default()
            })
            .collect();

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "guml-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                // Full text on every change. The compiler is fast enough that incremental sync
                // would be an optimisation with a correctness risk and no measured benefit.
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    // `{` opens a binding and a space starts a new positional: the two places
                    // where the useful list changes.
                    trigger_characters: Some(vec!["{".into(), " ".into()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                // `prepare` so the editor can refuse before the user types a new name, rather than
                // after — a rename dialog that accepts input and then errors is the worse order.
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: features::TOKEN_TYPES
                                    .iter()
                                    .map(|t| SemanticTokenType::new(t))
                                    .collect(),
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "guml-lsp ready — diagnostics from the compiler itself")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        self.docs.insert(uri.clone(), params.text_document.text);
        self.publish(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().next_back() {
            self.docs.insert(uri.clone(), change.text);
        }
        self.publish(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.docs.remove(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let at = params.text_document_position.position;
        let src = self.text(&uri);

        let items = features::completions(
            &src,
            features::Position { line: at.line, character: at.character },
        )
        .into_iter()
        .map(|c| CompletionItem {
            kind: Some(match c.kind {
                features::CompletionKind::Tag => CompletionItemKind::CLASS,
                features::CompletionKind::Modifier => CompletionItemKind::ENUM_MEMBER,
                features::CompletionKind::Attribute => CompletionItemKind::PROPERTY,
                features::CompletionKind::State => CompletionItemKind::VARIABLE,
                features::CompletionKind::Resource => CompletionItemKind::MODULE,
            }),
            detail: Some(c.detail),
            label: c.label,
            ..Default::default()
        })
        .collect();

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let at = params.text_document_position_params.position;
        let src = self.text(&uri);

        Ok(features::hover(&src, features::Position { line: at.line, character: at.character })
            .map(|text| Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: text,
                }),
                range: None,
            }))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let src = self.text(&params.text_document.uri);
        Ok(features::format(&src).map(|text| {
            // One edit replacing everything: the formatter reasons about whole documents, and a
            // minimal diff computed here could not be verified against it.
            let end = features::offset_to_position(&src, src.len());
            vec![TextEdit {
                range: Range {
                    start: Position::new(0, 0),
                    end: Position::new(end.line, end.character),
                },
                new_text: text,
            }]
        }))
    }

    /// Go to the declaration a name refers to.
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri.clone();
        let src = self.text(&uri);
        let at = pos(params.text_document_position_params.position);
        Ok(navigate::definition(&src, at)
            .map(|r| GotoDefinitionResponse::Scalar(Location { uri, range: lsp_range(r) })))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let src = self.text(&uri);
        let at = pos(params.text_document_position.position);
        let Some(name) = src
            .lines()
            .nth(at.line as usize)
            .and_then(|l| features::word_at(l, at.character as usize))
        else {
            return Ok(None);
        };
        Ok(Some(
            navigate::references(&src, &name)
                .into_iter()
                .map(|r| Location { uri: uri.clone(), range: lsp_range(r) })
                .collect(),
        ))
    }

    /// Refuse early, before the user has typed a new name.
    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let src = self.text(&params.text_document.uri);
        let at = pos(params.position);
        // A name is renameable exactly when it has a definition in this document.
        match navigate::definition(&src, at) {
            Some(_) => {
                let line = src.lines().nth(at.line as usize).unwrap_or("");
                let Some(name) = features::word_at(line, at.character as usize) else {
                    return Ok(None);
                };
                // The word's own range, so the editor pre-fills the dialog with just the name.
                // Character offset, not byte offset: the protocol counts UTF-16 code units, and a line
                // with an em dash before the name would otherwise place the range too far left.
                let col =
                    line.find(&name).map(|byte| line[..byte].chars().count() as u32).unwrap_or(0);
                Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                    range: Range {
                        start: Position::new(at.line, col),
                        end: Position::new(at.line, col + name.chars().count() as u32),
                    },
                    placeholder: name,
                }))
            }
            None => Ok(None),
        }
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let src = self.text(&uri);
        let at = pos(params.text_document_position.position);

        match navigate::rename(&src, at, &params.new_name) {
            Ok(ranges) if ranges.is_empty() => Ok(None),
            Ok(ranges) => {
                let edits: Vec<TextEdit> = ranges
                    .into_iter()
                    .map(|r| TextEdit { range: lsp_range(r), new_text: params.new_name.clone() })
                    .collect();
                Ok(Some(WorkspaceEdit {
                    changes: Some([(uri, edits)].into_iter().collect()),
                    ..Default::default()
                }))
            }
            // Surfaced as a protocol error so the editor shows the reason instead of applying a
            // rename that would break the document.
            Err(why) => Err(tower_lsp::jsonrpc::Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: describe(&why).into(),
                data: None,
            }),
        }
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let src = self.text(&params.text_document.uri);
        let range = navigate::Range { start: pos(params.range.start), end: pos(params.range.end) };
        Ok(navigate::format_range(&src, range)
            .map(|(r, text)| vec![TextEdit { range: lsp_range(r), new_text: text }]))
    }

    /// Quick fixes, straight from the diagnostics.
    ///
    /// The compiler already worked out the edit — this is the payoff for `suggestion` being a
    /// machine-applicable replacement rather than prose. Template suggestions (`aria="…"`) and
    /// bare words attached to whole-line spans are excluded upstream, because applying either
    /// silently damages the document.
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let src = self.text(&uri);
        let wanted = params.range;

        let actions = features::diagnostics(&src)
            .into_iter()
            .filter_map(|d| {
                let fix = d.quick_fix.clone()?;
                let range = Range {
                    start: Position::new(d.start.line, d.start.character),
                    end: Position::new(d.end.line, d.end.character),
                };
                // Only offer fixes for diagnostics the cursor is actually near.
                if range.end.line < wanted.start.line || range.start.line > wanted.end.line {
                    return None;
                }

                let mut changes = std::collections::HashMap::new();
                changes.insert(uri.clone(), vec![TextEdit { range, new_text: fix.clone() }]);

                Some(CodeActionOrCommand::CodeAction(CodeAction {
                    title: format!("{}: replace with `{fix}`", d.code),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: None,
                    edit: Some(WorkspaceEdit { changes: Some(changes), ..Default::default() }),
                    is_preferred: Some(true),
                    ..Default::default()
                }))
            })
            .collect();
        let mut actions: Vec<CodeActionOrCommand> = actions;

        // Document-level actions. The per-diagnostic fix is the wrong shape for the common case: a pasted
        // generation has six unknown tags, and fixing them one at a time is six keystrokes for six edits
        // the compiler had already described completely.
        //
        // The whole-document range is what an editor expects for a source action, and `u32::MAX` as the
        // end is how the rest of this file already spells "to the end of the line".
        let whole = Range {
            start: Position::new(0, 0),
            end: Position::new(src.lines().count() as u32, u32::MAX),
        };
        let replace_all = |text: String| {
            let mut changes = std::collections::HashMap::new();
            changes.insert(uri.clone(), vec![TextEdit { range: whole, new_text: text }]);
            Some(WorkspaceEdit { changes: Some(changes), ..Default::default() })
        };

        if let Some(fixed) = features::fix_all(&src) {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Fix all: apply every unambiguous suggestion".to_string(),
                // `source.fixAll` is the kind an editor can be configured to run on save, which is the
                // point — a document that can be repaired with no model call should not need a keystroke.
                kind: Some(CodeActionKind::SOURCE_FIX_ALL),
                edit: replace_all(fixed),
                ..Default::default()
            }));
        }

        if let Some(repaired) = features::repair(&src) {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Repair: strip packaging, format, fix".to_string(),
                // Deliberately **not** `SOURCE_FIX_ALL`. This also deletes — a code fence, trailing
                // commentary — and an editor must not silently remove lines on save under an action a
                // user configured for "fix".
                kind: Some(CodeActionKind::SOURCE),
                edit: replace_all(repaired),
                ..Default::default()
            }));
        }

        Ok(Some(actions))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let src = self.text(&params.text_document.uri);
        let symbols = features::symbols(&src)
            .into_iter()
            .map(|s| {
                let range =
                    Range { start: Position::new(s.line, 0), end: Position::new(s.line, u32::MAX) };
                #[allow(deprecated)]
                SymbolInformation {
                    name: s.name,
                    kind: SymbolKind::FIELD,
                    tags: None,
                    deprecated: None,
                    location: Location { uri: params.text_document.uri.clone(), range },
                    container_name: Some(s.detail),
                }
            })
            .collect();
        Ok(Some(DocumentSymbolResponse::Flat(symbols)))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let src = self.text(&params.text_document.uri);
        let data = features::semantic_tokens(&src)
            .chunks(5)
            .map(|c| SemanticToken {
                delta_line: c[0],
                delta_start: c[1],
                length: c[2],
                token_type: c[3],
                token_modifiers_bitset: c[4],
            })
            .collect();
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data })))
    }
}

fn pos(p: Position) -> features::Position {
    features::Position { line: p.line, character: p.character }
}

fn lsp_range(r: navigate::Range) -> Range {
    Range {
        start: Position::new(r.start.line, r.start.character),
        end: Position::new(r.end.line, r.end.character),
    }
}

fn describe(why: &navigate::RenameError) -> String {
    use navigate::RenameError as E;
    match why {
        E::NotADeclaration => {
            "only a `state`, `data`, `type` or `def` declared in this document can be renamed"
                .to_string()
        }
        E::BadName(n) => format!(
            "`{n}` cannot be written as a GUML name: letters, digits and `_`, starting with a letter"
        ),
        E::Taken(what) => what.clone(),
        E::WouldBreak(first) => {
            format!("that rename would break the document — {first}")
        }
    }
}

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| Backend { client, docs: DashMap::new() });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket).serve(service).await;
}
