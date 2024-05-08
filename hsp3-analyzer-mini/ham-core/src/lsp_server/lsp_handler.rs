use super::*;
use crate::analyzer::Analyzer;
use lsp_types::request::Request;
use lsp_types::*;
use std::io;

pub(super) struct LspHandler<W: io::Write> {
    config: LspConfig,
    sender: LspSender<W>,
    analyzer: Analyzer,
}

impl<W: io::Write> LspHandler<W> {
    pub(crate) fn new(config: LspConfig, sender: LspSender<W>, analyzer: Analyzer) -> Self {
        Self {
            config,
            sender,
            analyzer,
        }
    }

    fn register_file_system_watcher(&mut self) {
        if !self.config.watcher_enabled {
            return;
        }

        self.sender.send_request(
            // 他のリクエストを送らないので id=1 しか使わない。
            1,
            "client/registerCapability",
            RegistrationParams {
                registrations: vec![Registration {
                    id: "1".into(),
                    method: "workspace/didChangeWatchedFiles".into(),
                    register_options: Some(
                        serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                            watchers: vec![FileSystemWatcher {
                                kind: Some(
                                    WatchKind::Create | WatchKind::Change | WatchKind::Delete,
                                ),
                                glob_pattern: "**/*.hsp".into(),
                            }],
                        })
                        .unwrap(),
                    ),
                }],
            },
        );
    }

    fn initialize<'a>(&'a mut self, params: InitializeParams) -> InitializeResult {
        let watchable = params
            .capabilities
            .workspace
            .and_then(|x| x.did_change_watched_files)
            .and_then(|x| x.dynamic_registration)
            .unwrap_or(false);

        if let Some(folders) = params.workspace_folders {
            for folder in folders {
                self.analyzer.add_workspace_folder(folder);
            }
        }

        if !watchable {
            self.config.watcher_enabled = false;
        }

        InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        ..TextDocumentSyncOptions::default()
                    },
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(true),
                    trigger_characters: None,
                    ..CompletionOptions::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
                document_symbol_provider: if self.config.document_symbol_enabled {
                    Some(OneOf::Left(true))
                } else {
                    None
                },
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::PARAMETER, // 0
                                    SemanticTokenType::VARIABLE,  // 1
                                    SemanticTokenType::FUNCTION,  // 2
                                    SemanticTokenType::MACRO,     // 3
                                    SemanticTokenType::NAMESPACE, // 4
                                    SemanticTokenType::KEYWORD,   // 5
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::READONLY, // 0b01
                                    SemanticTokenModifier::STATIC,   // 0b10
                                ],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec![
                        " ".to_string(),
                        "(".to_string(),
                        ",".to_string(),
                    ]),
                    ..Default::default()
                }),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            // 参考: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        }
    }

    fn did_initialize(&mut self) {
        self.analyzer.did_initialize();
        self.register_file_system_watcher();
    }

    fn shutdown(&mut self) {
        self.analyzer.shutdown();
    }

    fn did_exit(&mut self, _json: &str) {
        std::process::exit(0)
    }

    fn text_document_did_open(&mut self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.analyzer.open_doc(doc.uri, doc.version, doc.text);
    }

    fn text_document_did_change(&mut self, params: DidChangeTextDocumentParams) {
        let text = (params.content_changes.into_iter())
            .next()
            .map(|c| c.text)
            .unwrap_or("".to_string());

        let doc = params.text_document;
        let version = doc.version;

        self.analyzer.change_doc(doc.uri, version, text);
    }

    fn text_document_did_close(&mut self, params: DidCloseTextDocumentParams) {
        self.analyzer.close_doc(params.text_document.uri);
    }

    fn text_document_code_action(&mut self, params: CodeActionParams) -> Vec<CodeAction> {
        self.analyzer.compute_ref().code_action(
            params.text_document.uri,
            params.range,
            params.context,
        )
    }

    fn text_document_completion(&mut self, params: CompletionParams) -> CompletionList {
        self.analyzer.compute_ref().completion(
            params.text_document_position.text_document.uri,
            params.text_document_position.position,
        )
    }

    fn text_document_completion_resolve(
        &mut self,
        params: CompletionItem,
    ) -> Option<CompletionItem> {
        self.analyzer.compute_ref().completion_resolve(params)
    }

    fn text_document_formatting(
        &mut self,
        params: DocumentFormattingParams,
    ) -> Option<Vec<TextEdit>> {
        self.analyzer
            .compute_ref()
            .formatting(params.text_document.uri)
    }

    fn text_document_definition(
        &mut self,
        params: TextDocumentPositionParams,
    ) -> lsp_types::GotoDefinitionResponse {
        let definitions = self
            .analyzer
            .compute_ref()
            .definitions(params.text_document.uri, params.position);

        if definitions.len() == 1 {
            lsp_types::GotoDefinitionResponse::Scalar(definitions.into_iter().next().unwrap())
        } else {
            lsp_types::GotoDefinitionResponse::Array(definitions)
        }
    }

    fn text_document_highlight(
        &mut self,
        params: TextDocumentPositionParams,
    ) -> Vec<lsp_types::DocumentHighlight> {
        self.analyzer
            .compute_ref()
            .document_highlight(params.text_document.uri, params.position)
    }

    fn text_document_symbol(
        &mut self,
        params: DocumentSymbolParams,
    ) -> Option<lsp_types::DocumentSymbolResponse> {
        self.analyzer
            .compute_ref()
            .document_symbol(params.text_document.uri)
    }

    fn text_document_hover(&mut self, params: TextDocumentPositionParams) -> Option<Hover> {
        self.analyzer
            .compute_ref()
            .hover(params.text_document.uri, params.position)
    }

    fn text_document_prepare_rename(
        &mut self,
        params: TextDocumentPositionParams,
    ) -> Option<PrepareRenameResponse> {
        self.analyzer
            .compute_ref()
            .prepare_rename(params.text_document.uri, params.position)
    }

    fn text_document_references(&mut self, params: ReferenceParams) -> Vec<Location> {
        self.analyzer.compute_ref().references(
            params.text_document_position.text_document.uri,
            params.text_document_position.position,
            params.context.include_declaration,
        )
    }

    fn text_document_rename(&mut self, params: RenameParams) -> Option<WorkspaceEdit> {
        self.analyzer.compute_ref().rename(
            params.text_document_position.text_document.uri,
            params.text_document_position.position,
            params.new_name,
        )
    }

    fn text_document_semantic_tokens_full(
        &mut self,
        params: SemanticTokensParams,
    ) -> SemanticTokensResult {
        let uri = params.text_document.uri;
        SemanticTokensResult::Tokens(self.analyzer.compute_ref().semantic_tokens(uri))
    }

    fn text_document_signature_help(
        &mut self,
        params: SignatureHelpParams,
    ) -> Option<SignatureHelp> {
        let (uri, position) = {
            let p = params.text_document_position_params;
            (p.text_document.uri, p.position)
        };

        self.analyzer.compute_ref().signature_help(uri, position)
    }

    fn workspace_did_change_watched_files(&mut self, params: DidChangeWatchedFilesParams) {
        for param in params.changes {
            match param.typ {
                FileChangeType::CREATED => self.analyzer.on_file_created(param.uri),
                FileChangeType::CHANGED => self.analyzer.on_file_changed(param.uri),
                FileChangeType::DELETED => self.analyzer.on_file_deleted(param.uri),
                _ => continue,
            }
        }
    }

    fn workspace_symbol(&mut self, params: WorkspaceSymbolParams) -> Vec<SymbolInformation> {
        self.analyzer.compute_ref().workspace_symbol(params.query)
    }

    fn diagnose(&mut self) {
        let diagnostics = self.analyzer.compute_ref().diagnose();

        for (uri, version, diagnostics) in diagnostics {
            self.sender.send_notification(
                "textDocument/publishDiagnostics",
                PublishDiagnosticsParams {
                    uri,
                    version,
                    diagnostics,
                },
            );
        }
    }

    fn did_receive(&mut self, json: &str) {
        let msg = serde_json::from_str::<LspMessageOpaque>(json).unwrap();

        let method = match msg.method {
            Some(it) => it,

            // registerCapabilityのレスポンス。
            None if json.contains("\"result\"") && !json.contains("\"error\"") => return,

            None => {
                // TODO: エラー処理？
                warn!("no method: {}", json);
                return;
            }
        };

        match method.as_str() {
            "initialize" => {
                let msg = serde_json::from_str::<LspRequest<InitializeParams>>(json).unwrap();
                let (params, msg_id) = (msg.params, msg.id);
                let response = self.initialize(params);
                self.sender.send_response(msg_id, response);
            }
            "initialized" => {
                self.did_initialize();
                self.diagnose();
            }
            "shutdown" => {
                let msg = serde_json::from_str::<LspRequest<()>>(json).unwrap();
                self.shutdown();
                self.sender.send_response(msg.id, ());
            }
            "exit" => {
                self.did_exit(json);
            }
            "textDocument/didOpen" => {
                let msg: LspNotification<DidOpenTextDocumentParams> =
                    serde_json::from_str(&json).expect("didOpen msg");
                self.text_document_did_open(msg.params);
                self.diagnose();
            }
            "textDocument/didChange" => {
                let msg: LspNotification<DidChangeTextDocumentParams> =
                    serde_json::from_str(&json).expect("didChange msg");
                self.text_document_did_change(msg.params);
                self.diagnose();
            }
            "textDocument/didClose" => {
                let msg = serde_json::from_str::<LspNotification<DidCloseTextDocumentParams>>(json)
                    .unwrap();
                self.text_document_did_close(msg.params);
                self.diagnose();
            }
            lsp_types::request::CodeActionRequest::METHOD => {
                let msg = serde_json::from_str::<LspRequest<CodeActionParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_code_action(msg.params);
                self.sender.send_response(msg_id, response);
            }
            "textDocument/completion" => {
                let msg = serde_json::from_str::<LspRequest<CompletionParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_completion(msg.params);
                self.sender.send_response(msg_id, response);
            }
            lsp_types::request::ResolveCompletionItem::METHOD => {
                let msg = serde_json::from_str::<LspRequest<CompletionItem>>(json).unwrap();
                match self.text_document_completion_resolve(msg.params) {
                    Some(response) => self.sender.send_response(msg.id, response),
                    None => self.sender.send_error_code(
                        Some(Value::from(msg.id)),
                        -32001, // unknown
                        "Resolve completion failed.".into(),
                    ),
                }
            }
            lsp_types::request::Formatting::METHOD => {
                let msg =
                    serde_json::from_str::<LspRequest<DocumentFormattingParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_formatting(msg.params);
                self.sender.send_response(msg_id, response);
            }
            "textDocument/definition" => {
                let msg =
                    serde_json::from_str::<LspRequest<TextDocumentPositionParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_definition(msg.params);
                self.sender.send_response(msg_id, response);
            }
            "textDocument/documentHighlight" => {
                let msg =
                    serde_json::from_str::<LspRequest<TextDocumentPositionParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_highlight(msg.params);
                self.sender.send_response(msg_id, response);
            }
            request::DocumentSymbolRequest::METHOD => {
                let msg = serde_json::from_str::<LspRequest<DocumentSymbolParams>>(json).unwrap();
                let response = self.text_document_symbol(msg.params);
                self.sender.send_response(msg.id, response);
            }
            "textDocument/hover" => {
                let msg: LspRequest<TextDocumentPositionParams> =
                    serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_hover(msg.params);
                self.sender.send_response(msg_id, response);
            }
            request::PrepareRenameRequest::METHOD => {
                let msg: LspRequest<TextDocumentPositionParams> =
                    serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_prepare_rename(msg.params);
                self.sender.send_response(msg_id, response);
            }
            "textDocument/references" => {
                let msg: LspRequest<ReferenceParams> = serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_references(msg.params);
                self.sender.send_response(msg_id, response);
            }
            request::Rename::METHOD => {
                let msg: LspRequest<RenameParams> = serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_rename(msg.params);
                self.sender.send_response(msg_id, response);
            }
            request::SemanticTokensFullRequest::METHOD => {
                let msg: LspRequest<SemanticTokensParams> =
                    serde_json::from_str(json).expect("semantic tokens full msg");
                let response: SemanticTokensResult =
                    self.text_document_semantic_tokens_full(msg.params);
                self.sender.send_response(msg.id, response);
            }
            request::SignatureHelpRequest::METHOD => {
                let msg: LspRequest<SignatureHelpParams> = serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_signature_help(msg.params);
                self.sender.send_response(msg_id, response);
            }
            "workspace/didChangeWatchedFiles" => {
                let msg: LspNotification<DidChangeWatchedFilesParams> =
                    serde_json::from_str(json).expect("workspace/didChangeWatchedFiles msg");
                self.workspace_did_change_watched_files(msg.params);
                self.diagnose();
            }
            request::WorkspaceSymbol::METHOD => {
                let msg: LspRequest<WorkspaceSymbolParams> =
                    serde_json::from_str(json).expect("workspace/symbol msg");
                let response = self.workspace_symbol(msg.params);
                self.sender.send_response(msg.id, response);
            }
            "$/cancelRequest" => self.sender.send_error_code(
                msg.id,
                error::METHOD_NOT_FOUND,
                "キャンセルは未実装です。",
            ),
            _ => self.sender.send_error_code(
                msg.id,
                error::METHOD_NOT_FOUND,
                "未実装のメソッドを無視します。",
            ),
        }
    }

    pub(crate) fn main(mut self, mut receiver: LspReceiver<impl io::Read>) {
        loop {
            receiver.read_next(|json| self.did_receive(json));
        }
    }
}
