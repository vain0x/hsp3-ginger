use super::*;
use crate::lsp_model::LspModel;
use ::lsp_types::*;
use std::io;

pub(crate) struct LspHandler<W: io::Write> {
    sender: LspSender<W>,
    model: LspModel,
}

impl<W: io::Write> LspHandler<W> {
    pub(crate) fn new(sender: LspSender<W>, model: LspModel) -> Self {
        LspHandler { sender, model }
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
                    resolve_provider: Some(false),
                    trigger_characters: None,
                }),
                definition_provider: Some(true),
                document_highlight_provider: Some(false),
                hover_provider: Some(false),
                references_provider: Some(false),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec![",".to_string(), "(".to_string()]),
                }),
                ..ServerCapabilities::default()
            },
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
            PublishDiagnosticsParams { uri, diagnostics },
        );
    }

    fn text_document_did_close(&mut self, params: DidCloseTextDocumentParams) {
        self.model.close_doc(params.text_document.uri);
    }

    fn text_document_completion(&mut self, params: CompletionParams) -> CompletionList {
        self.model
            .completion(params.text_document.uri, params.position)
    }

    fn completion_item_resolve(&mut self, completion_item: CompletionItem) -> CompletionItem {
        completion_item
    }

    fn text_document_definition(
        &mut self,
        params: TextDocumentPositionParams,
    ) -> request::GotoDefinitionResponse {
        let definitions = self
            .model
            .definitions(params.text_document.uri, params.position);

        if definitions.len() == 1 {
            request::GotoDefinitionResponse::Scalar(definitions.into_iter().next().unwrap())
        } else {
            request::GotoDefinitionResponse::Array(definitions)
        }
    }

    fn text_document_highlight(
        &mut self,
        params: TextDocumentPositionParams,
    ) -> Vec<DocumentHighlight> {
        self.model
            .highlights(params.text_document.uri, params.position)
    }

    fn text_document_hover(&mut self, params: TextDocumentPositionParams) -> Option<Hover> {
        self.model.hover(params.text_document.uri, params.position)
    }

    fn text_document_references(&mut self, params: ReferenceParams) -> Vec<Location> {
        self.model.references(
            params.text_document.uri,
            params.position,
            params.context.include_declaration,
        )
    }

    fn text_document_signature_help(
        &mut self,
        params: TextDocumentPositionParams,
    ) -> Option<SignatureHelp> {
        self.model
            .signature_help(params.text_document.uri, params.position)
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
            "textDocument/references" => {
                let msg: LspRequest<ReferenceParams> = serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_references(msg.params);
                self.sender.send_response(msg_id, response);
            }
            "textDocument/signatureHelp" => {
                let msg: LspRequest<TextDocumentPositionParams> =
                    serde_json::from_str(json).unwrap();
                let msg_id = msg.id;
                let response = self.text_document_signature_help(msg.params);
                self.sender.send_response(msg_id, response);
            }
            _ => warn!("Msg unresolved."),
        }
    }

    pub(crate) fn main(mut self, mut receiver: LspReceiver<impl io::Read>) -> ! {
        loop {
            receiver.read_next(|json| self.did_receive(json));
        }
    }
}
