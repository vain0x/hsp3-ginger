use super::*;
use crate::lang_service::LangService;
use lsp_types::request::Request;
use lsp_types::*;
use std::io;

pub(super) struct LspHandler<W: io::Write> {
    sender: LspSender<W>,
    model: LangService,
}

impl<W: io::Write> LspHandler<W> {
    pub(crate) fn new(sender: LspSender<W>, model: LangService) -> Self {
        Self { sender, model }
    }

    fn initialize<'a>(&'a mut self, params: InitializeParams) -> InitializeResult {
        self.model.initialize(params.root_uri);

        InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::Full),
                        ..TextDocumentSyncOptions::default()
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: None,
                    trigger_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                document_formatting_provider: Some(true),
                definition_provider: Some(true),
                document_highlight_provider: Some(true),
                hover_provider: Some(true),
                references_provider: Some(true),
                rename_provider: Some(RenameProviderCapability::Options(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec![
                        " ".to_string(),
                        "(".to_string(),
                        ",".to_string(),
                    ]),
                    ..Default::default()
                }),
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
        self.model.did_initialize();
        self.diagnose();
    }

    fn shutdown(&mut self) {
        self.model.shutdown();
    }

    fn did_exit(&mut self, _json: &str) {
        std::process::exit(0)
    }

    fn text_document_did_open(&mut self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.model.open_doc(doc.uri, doc.version, doc.text);
    }

    fn text_document_did_change(&mut self, params: DidChangeTextDocumentParams) {
        let text = (params.content_changes.into_iter())
            .next()
            .map(|c| c.text)
            .unwrap_or("".to_string());

        let doc = params.text_document;
        let version = doc.version.unwrap_or(0);

        self.model.change_doc(doc.uri, version, text);
    }

    fn text_document_did_close(&mut self, params: DidCloseTextDocumentParams) {
        self.model.close_doc(params.text_document.uri);
    }

    fn text_document_completion(&mut self, params: CompletionParams) -> CompletionList {
        self.model.completion(
            params.text_document_position.text_document.uri,
            params.text_document_position.position,
        )
    }

    fn text_document_formatting(
        &mut self,
        params: DocumentFormattingParams,
    ) -> Option<Vec<TextEdit>> {
        self.model.formatting(params.text_document.uri)
    }

    fn text_document_definition(
        &mut self,
        params: TextDocumentPositionParams,
    ) -> lsp_types::GotoDefinitionResponse {
        let definitions = self
            .model
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
        self.model
            .document_highlight(params.text_document.uri, params.position)
    }

    fn text_document_hover(&mut self, params: TextDocumentPositionParams) -> Option<Hover> {
        self.model.hover(params.text_document.uri, params.position)
    }

    fn text_document_prepare_rename(
        &mut self,
        params: TextDocumentPositionParams,
    ) -> Option<PrepareRenameResponse> {
        self.model
            .prepare_rename(params.text_document.uri, params.position)
    }

    fn text_document_references(&mut self, params: ReferenceParams) -> Vec<Location> {
        self.model.references(
            params.text_document_position.text_document.uri,
            params.text_document_position.position,
            params.context.include_declaration,
        )
    }

    fn text_document_rename(&mut self, params: RenameParams) -> Option<WorkspaceEdit> {
        self.model.rename(
            params.text_document_position.text_document.uri,
            params.text_document_position.position,
            params.new_name,
        )
    }

    fn text_document_signature_help(
        &mut self,
        params: SignatureHelpParams,
    ) -> Option<SignatureHelp> {
        let (uri, position) = {
            let p = params.text_document_position_params;
            (p.text_document.uri, p.position)
        };

        self.model.signature_help(uri, position)
    }

    fn diagnose(&mut self) {
        let diagnostics = self.model.diagnose();

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

        match msg.method.as_str() {
            "initialize" => {
                let msg = serde_json::from_str::<LspRequest<InitializeParams>>(json).unwrap();
                let (params, msg_id) = (msg.params, msg.id);
                let response = self.initialize(params);
                self.sender.send_response(msg_id, response);
            }
            "initialized" => {
                self.did_initialize();
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
            "textDocument/completion" => {
                let msg = serde_json::from_str::<LspRequest<CompletionParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_completion(msg.params);
                self.sender.send_response(msg_id, response);
            }
            lsp_types::request::Formatting::METHOD => {
                let msg =
                    serde_json::from_str::<LspRequest<DocumentFormattingParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_formatting(msg.params);
                self.sender.send_response(msg_id, response);
                self.diagnose();
            }
            "textDocument/definition" => {
                let msg =
                    serde_json::from_str::<LspRequest<TextDocumentPositionParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_definition(msg.params);
                self.sender.send_response(msg_id, response);
                self.diagnose();
            }
            "textDocument/documentHighlight" => {
                let msg =
                    serde_json::from_str::<LspRequest<TextDocumentPositionParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_highlight(msg.params);
                self.sender.send_response(msg_id, response);
                self.diagnose();
            }
            "textDocument/hover" => {
                let msg: LspRequest<TextDocumentPositionParams> =
                    serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_hover(msg.params);
                self.sender.send_response(msg_id, response);
                self.diagnose();
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
                self.diagnose();
            }
            request::Rename::METHOD => {
                let msg: LspRequest<RenameParams> = serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_rename(msg.params);
                self.sender.send_response(msg_id, response);
                self.diagnose();
            }
            request::SignatureHelpRequest::METHOD => {
                let msg: LspRequest<SignatureHelpParams> = serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_signature_help(msg.params);
                self.sender.send_response(msg_id, response);
                self.diagnose();
            }
            _ => warn!("Msg unresolved."),
        }
    }

    pub(crate) fn main(mut self, mut receiver: LspReceiver<impl io::Read>) {
        loop {
            receiver.read_next(|json| self.did_receive(json));
        }
    }
}
