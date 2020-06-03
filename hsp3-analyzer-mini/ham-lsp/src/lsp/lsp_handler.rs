use crate::lsp::*;
use lsp_types::*;
use request::Request;
use std::io;
use std::path::PathBuf;

pub(super) struct LspHandler<W: io::Write> {
    sender: LspSender<W>,
    model: LspModel,
}

impl<W: io::Write> LspHandler<W> {
    pub fn new(sender: LspSender<W>, hsp_root: PathBuf) -> Self {
        Self {
            sender,
            model: LspModel::new(hsp_root),
        }
    }

    fn initialize<'a>(&'a mut self, _params: InitializeParams) -> InitializeResult {
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
                    resolve_provider: Some(true),
                    trigger_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                definition_provider: Some(true),
                document_highlight_provider: Some(true),
                hover_provider: Some(true),
                references_provider: Some(true),
                rename_provider: Some(RenameProviderCapability::Options(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "hsp3-analyzer-mini".to_string(),
                // FIXME: バージョン番号 (cargo の環境変数から env! でとれるはず)
                version: None,
            }),
        }
    }

    fn did_initialize(&mut self) {
        self.model.did_initialize();
    }

    fn shutdown(&mut self) {
        self.model.shutdown();
    }

    fn did_exit(&mut self, _json: &str) {
        std::process::exit(0)
    }

    fn text_document_did_open(&mut self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        let uri = doc.uri.to_owned();
        self.model.open_doc(doc.uri, doc.version, doc.text);

        self.text_document_did_open_or_change(uri);
    }

    fn text_document_did_change(&mut self, params: DidChangeTextDocumentParams) {
        let text = (params.content_changes.into_iter())
            .next()
            .map(|c| c.text)
            .unwrap_or("".to_string());

        let doc = params.text_document;
        let uri = doc.uri.to_owned();
        let version = doc.version.unwrap_or(0);

        self.model.change_doc(doc.uri, version, text);

        self.text_document_did_open_or_change(uri);
    }

    fn text_document_did_open_or_change(&mut self, uri: Url) {
        let diagnostics = self.model.validate(uri.clone());

        self.sender.send_notification(
            "textDocument/publishDiagnostics",
            PublishDiagnosticsParams {
                uri,
                version: None,
                diagnostics,
            },
        );
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

    fn completion_item_resolve(&mut self, completion_item: CompletionItem) -> CompletionItem {
        // FIXME:
        // completion_item.data.take().and_then(|data| -> Option<()> {
        //     // let data = features::completion::parse_data(data)?;
        //     let data = serde_json::from_value::<String>(data).ok()?;
        //     self.model.completion_resolve(&mut completion_item, data);
        //     Some(())
        // });

        completion_item
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
            .highlights(params.text_document.uri, params.position)
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
            }
            "textDocument/didChange" => {
                let msg: LspNotification<DidChangeTextDocumentParams> =
                    serde_json::from_str(&json).expect("didChange msg");
                self.text_document_did_change(msg.params);
            }
            "textDocument/didClose" => {
                let msg = serde_json::from_str::<LspNotification<DidCloseTextDocumentParams>>(json)
                    .unwrap();
                self.text_document_did_close(msg.params);
            }
            "textDocument/completion" => {
                let msg = serde_json::from_str::<LspRequest<CompletionParams>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_completion(msg.params);
                self.sender.send_response(msg_id, response);
            }
            "completionItem/resolve" => {
                let msg = serde_json::from_str::<LspRequest<CompletionItem>>(json).unwrap();
                let msg_id = msg.id;
                let response = self.completion_item_resolve(msg.params);
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
            _ => warn!("Msg unresolved."),
        }
    }

    pub fn main(mut self, mut receiver: LspReceiver<impl io::Read>) {
        loop {
            receiver.read_next(|json| self.did_receive(json));
        }
    }
}
