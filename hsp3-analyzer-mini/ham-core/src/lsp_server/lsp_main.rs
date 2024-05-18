use super::*;
use crate::{
    analyzer::{options::AnalyzerOptions, Analyzer},
    ide::diagnose::{filter_diagnostics, DiagnosticsCache},
    lsp_server::lsp_main::lsp_log::init_log,
};
use lsp_server::{Connection, ExtractError, Message, RequestId, Response};
use lsp_types::{
    error_codes,
    notification::{self, Notification as _},
    request::{self, Request as _},
    OneOf,
};
use serde::Serialize;
use std::{env, mem, path::PathBuf};

pub fn run_lsp_server(hsp3_root: PathBuf) {
    init_log();

    debug!("run_lsp_server, hsp3_root={:?}", hsp3_root);

    // 環境変数から設定をロードする:
    let lsp_config = LspConfig {
        document_symbol_enabled: env::var("HAM_DOCUMENT_SYMBOL_ENABLED").map_or(true, |s| s == "1"),
        watcher_enabled: env::var("HAM_WATCHER_ENABLED").map_or(true, |s| s == "1"),
    };
    let options = AnalyzerOptions {
        lint_enabled: env::var("HAM_LINT").map_or(true, |s| s == "1"),
    };

    // サーバーが持つ状態:
    let mut an = Analyzer::new(hsp3_root, options);
    let mut state = State::default();

    // connection (クライアントとの通信手段) として標準入出力やスレッドの準備を行う
    let (cx, io_threads) = Connection::stdio();

    // LSPサーバーの初期化処理を行う
    // (`cx.initialize` によって "initialized" 通知が来るまで通信が進み、"initialize" リクエストのパラメータが返される)
    let server_capabilities =
        serde_json::to_value(functions::generate_server_capabilities(&lsp_config)).unwrap();

    let init_params = match cx.initialize(server_capabilities) {
        Ok(value) => serde_json::from_value::<lsp_types::InitializeParams>(value).unwrap(),
        Err(err) => {
            error!("initialize");
            if err.channel_is_disconnected() {
                io_threads.join().unwrap();
            }
            return;
        }
    };

    let watchable = lsp_config.watcher_enabled
        && init_params
            .capabilities
            .workspace
            .and_then(|x| x.did_change_watched_files)
            .and_then(|x| x.dynamic_registration)
            .unwrap_or(false);

    if watchable {
        functions::register_file_system_watcher(&cx);
    }

    if let Some(folders) = init_params.workspace_folders {
        for folder in folders {
            an.add_workspace_folder(folder);
        }
    }

    an.did_initialize();

    // メインループ:
    debug!("Starting main loop");

    for msg in &cx.receiver {
        match msg {
            Message::Request(req) => {
                // debug!("got request: {req:?}");

                // "shutdown" リクエストなら `true` になる。
                // ("exit" 通知が来るまで通信が行われる)
                if cx.handle_shutdown(&req).unwrap() {
                    break;
                }

                dispatch_request(&cx, &mut an, &mut state, req);
                continue;
            }
            Message::Response(resp) => {
                debug!("got response: {resp:?} (ignored)");
                continue;
            }
            Message::Notification(nn) => {
                // debug!("got notification: {nn:?}");

                // `handle_shutdown` によって処理されるため
                debug_assert!(nn.method != notification::Exit::METHOD);

                dispatch_notification(&cx, &mut an, &mut state, nn);
                continue;
            }
        }
    }

    io_threads.join().unwrap();
    debug!("Exiting gracefully");
}

// -----------------------------------------------
// Dispatcher
// -----------------------------------------------

#[derive(Default)]
struct State {
    diagnostics_invalidated: bool,
    diagnostics_cache: DiagnosticsCache,
}

/// リクエストを処理する
fn dispatch_request(
    cx: &lsp_server::Connection,
    an: &mut Analyzer,
    state: &mut State,
    req: lsp_server::Request,
) {
    match req.method.as_str() {
        // "textDocument/codeAction"
        request::CodeActionRequest::METHOD => {
            let (id, params) = cast_req::<request::CodeActionRequest>(req).unwrap();
            let result = an.compute_ref().code_action(
                params.text_document.uri,
                params.range,
                params.context,
            );
            cx.sender.send(new_ok_response(id, result)).unwrap();

            functions::publish_diagnostics(cx, an, state);
            return;
        }
        // "textDocument/completion"
        request::Completion::METHOD => {
            let (id, params) = cast_req::<request::Completion>(req).unwrap();
            let result = an.compute_ref().completion(
                params.text_document_position.text_document.uri,
                params.text_document_position.position,
            );
            cx.sender.send(new_ok_response(id, result)).unwrap();

            functions::publish_diagnostics(cx, an, state);
            return;
        }
        // "completionItem/resolve"
        request::ResolveCompletionItem::METHOD => {
            let (id, params) = cast_req::<request::ResolveCompletionItem>(req).unwrap();
            match an.compute_ref().completion_resolve(params) {
                Some(result) => {
                    cx.sender.send(new_ok_response(id, result)).unwrap();
                }
                None => {
                    cx.sender
                        .send(Message::Response(lsp_server::Response::new_err(
                            id,
                            error_codes::UNKNOWN_ERROR_CODE as i32,
                            "Resolve completion failed.".to_string(),
                        )))
                        .unwrap();
                    return;
                }
            }

            functions::publish_diagnostics(cx, an, state);
            return;
        }
        // "textDocument/definition"
        request::GotoDefinition::METHOD => {
            let (id, params) = cast_req::<request::GotoDefinition>(req).unwrap();
            let (doc, pos) = {
                let p = params.text_document_position_params;
                (p.text_document, p.position)
            };

            let definitions = an.compute_ref().definitions(doc.uri, pos);

            let result = if definitions.len() == 1 {
                lsp_types::GotoDefinitionResponse::Scalar(definitions.into_iter().next().unwrap())
            } else {
                lsp_types::GotoDefinitionResponse::Array(definitions)
            };

            cx.sender.send(new_ok_response(id, result)).unwrap();
            return;
        }
        // "textDocument/documentHighlight"
        request::DocumentHighlightRequest::METHOD => {
            let (id, params) = cast_req::<request::DocumentHighlightRequest>(req).unwrap();
            let pp = params.text_document_position_params;
            let result = an
                .compute_ref()
                .document_highlight(pp.text_document.uri, pp.position);
            cx.sender.send(new_ok_response(id, result)).unwrap();

            functions::publish_diagnostics(cx, an, state);
            return;
        }
        // "textDocument/documentSymbol"
        request::DocumentSymbolRequest::METHOD => {
            let (id, params) = cast_req::<request::DocumentSymbolRequest>(req).unwrap();
            let result = an.compute_ref().document_symbol(params.text_document.uri);
            cx.sender.send(new_ok_response(id, result)).unwrap();

            functions::publish_diagnostics(cx, an, state);
            return;
        }
        // "textDocument/formatting"
        request::Formatting::METHOD => {
            let (id, params) = cast_req::<request::Formatting>(req).unwrap();
            let result = an.compute_ref().formatting(params.text_document.uri);
            cx.sender.send(new_ok_response(id, result)).unwrap();
            return;
        }
        // "textDocument/hover"
        request::HoverRequest::METHOD => {
            let (id, params) = cast_req::<request::HoverRequest>(req).unwrap();
            let pp = params.text_document_position_params;
            let result = an.compute_ref().hover(pp.text_document.uri, pp.position);
            cx.sender.send(new_ok_response(id, result)).unwrap();

            functions::publish_diagnostics(cx, an, state);
            return;
        }
        // "textDocument/prepareRename"
        request::PrepareRenameRequest::METHOD => {
            let (id, params) = cast_req::<request::PrepareRenameRequest>(req).unwrap();
            let result = an
                .compute_ref()
                .prepare_rename(params.text_document.uri, params.position);
            cx.sender.send(new_ok_response(id, result)).unwrap();
            return;
        }
        // "textDocument/references"
        request::References::METHOD => {
            let (id, params) = cast_req::<request::References>(req).unwrap();
            let pp = params.text_document_position;
            let result = an.compute_ref().references(
                pp.text_document.uri,
                pp.position,
                params.context.include_declaration,
            );
            cx.sender.send(new_ok_response(id, result)).unwrap();
            return;
        }
        // "textDocument/rename"
        request::Rename::METHOD => {
            let (id, params) = cast_req::<request::Rename>(req).unwrap();
            let pp = params.text_document_position;
            let result =
                an.compute_ref()
                    .rename(pp.text_document.uri, pp.position, params.new_name);
            cx.sender.send(new_ok_response(id, result)).unwrap();
            return;
        }
        // "textDocument/semanticTokens/full"
        request::SemanticTokensFullRequest::METHOD => {
            let (id, params) = cast_req::<request::SemanticTokensFullRequest>(req).unwrap();
            let result = lsp_types::SemanticTokensResult::Tokens(
                an.compute_ref().semantic_tokens(params.text_document.uri),
            );
            cx.sender.send(new_ok_response(id, result)).unwrap();
            return;
        }
        // "textDocument/signatureHelp"
        request::SignatureHelpRequest::METHOD => {
            let (id, params) = cast_req::<request::SignatureHelpRequest>(req).unwrap();
            let pp = params.text_document_position_params;
            let result = an
                .compute_ref()
                .signature_help(pp.text_document.uri, pp.position);
            cx.sender.send(new_ok_response(id, result)).unwrap();
            return;
        }
        // "workspace/symbol"
        request::WorkspaceSymbolRequest::METHOD => {
            let (id, params) = cast_req::<request::WorkspaceSymbolRequest>(req).unwrap();
            let result = an.compute_ref().workspace_symbol(params.query);
            cx.sender.send(new_ok_response(id, result)).unwrap();
            return;
        }
        _ => {
            // 未実装のリクエストにエラーレスポンスを返す
            cx.sender
                .send(Message::Response(Response::new_err(
                    req.id,
                    -32601,
                    "Method Not Found".to_string(),
                )))
                .unwrap();
            return;
        }
    }
}

/// 通知を処理する
fn dispatch_notification(
    _cx: &lsp_server::Connection,
    an: &mut Analyzer,
    state: &mut State,
    nn: lsp_server::Notification,
) {
    match nn.method.as_str() {
        // "initialized"
        notification::Initialized::METHOD => {
            // let params = cast_n::<notification::Initialized>(n).unwrap();
            return;
        }
        // "textDocument/didOpen"
        notification::DidOpenTextDocument::METHOD => {
            let params = cast_nn::<notification::DidOpenTextDocument>(nn).unwrap();
            // ls.text_document_did_open(params.text_document);
            // ls.diagnose();
            let doc = params.text_document;
            an.open_doc(doc.uri, doc.version, doc.text);

            state.diagnostics_invalidated = true;
            return;
        }
        // "textDocument/didChange"
        notification::DidChangeTextDocument::METHOD => {
            let params = cast_nn::<notification::DidChangeTextDocument>(nn).unwrap();
            let text = (params.content_changes.into_iter())
                .next()
                .map(|c| c.text)
                .unwrap_or_default();

            let doc = params.text_document;
            an.change_doc(doc.uri, doc.version, text);

            state.diagnostics_invalidated = true;
            return;
        }
        // "testDocument/didClose"
        notification::DidCloseTextDocument::METHOD => {
            let params = cast_nn::<notification::DidCloseTextDocument>(nn).unwrap();
            let d = params.text_document;

            an.close_doc(d.uri);
            state.diagnostics_invalidated = true;
            return;
        }
        // "workspace/didChangeWatchedFiles"
        notification::DidChangeWatchedFiles::METHOD => {
            let params = cast_nn::<notification::DidChangeWatchedFiles>(nn).unwrap();

            for param in params.changes {
                match param.typ {
                    lsp_types::FileChangeType::CREATED => an.on_file_created(param.uri),
                    lsp_types::FileChangeType::CHANGED => an.on_file_changed(param.uri),
                    lsp_types::FileChangeType::DELETED => an.on_file_deleted(param.uri),
                    _ => continue,
                }
            }

            state.diagnostics_invalidated = true;
            return;
        }
        _ if nn.method.starts_with("$/") => {
            // "$/" で始まるメソッド名の通知は暗黙に無視してよい
            debug!("Notification ignored: {:?}", nn.method);
        }
        _ => {
            debug!("Notification supported ({})", nn.method);
        }
    }
}

// ===============================================

// (`dispatch_xxx` に書くには長い処理をここに集める)

mod functions {
    use super::*;
    use lsp_types::*;

    pub(super) fn generate_server_capabilities(config: &LspConfig) -> ServerCapabilities {
        ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::FULL),
                    save: Some(TextDocumentSyncSaveOptions::Supported(true)),
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
            document_symbol_provider: if config.document_symbol_enabled {
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
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
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
                }),
            ),
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec![" ".to_string(), "(".to_string(), ",".to_string()]),
                ..Default::default()
            }),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            ..ServerCapabilities::default()
        }
    }

    pub(super) fn register_file_system_watcher(cx: &lsp_server::Connection) {
        cx.sender
            .send(Message::Request(lsp_server::Request::new(
                // id 1 しか使わない (ほかのリクエストを送らない)
                RequestId::from(1),
                // "client/registerCapability"
                request::RegisterCapability::METHOD.to_string(),
                RegistrationParams {
                    registrations: vec![Registration {
                        id: "1".to_string(),
                        method: "workspace/didChangeWatchedFiles".to_string(),
                        register_options: Some(
                            serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                                watchers: vec![FileSystemWatcher {
                                    kind: Some(
                                        WatchKind::Create | WatchKind::Change | WatchKind::Delete,
                                    ),
                                    glob_pattern: GlobPattern::from("**/*.hsp".to_string()),
                                }],
                            })
                            .unwrap(),
                        ),
                    }],
                },
            )))
            .unwrap();
    }

    /// `diagnostics` の変更があれば再送信する
    ///
    /// (この関数は `initialized`, `didSave` または解析系リクエストの処理後に呼ばれる)
    pub(super) fn publish_diagnostics(
        cx: &lsp_server::Connection,
        an: &mut Analyzer,
        state: &mut State,
    ) {
        // この処理は起動後に1回、およびドキュメントの変更のたびに1回だけ行う
        if !mem::replace(&mut state.diagnostics_invalidated, false) {
            return;
        }

        let mut diagnostics = an.compute_ref().diagnose();

        filter_diagnostics(&mut state.diagnostics_cache, &mut diagnostics);

        for (uri, version, diagnostics) in diagnostics {
            cx.sender
                .send(Message::Notification(lsp_server::Notification::new(
                    // "textDocument/publishDiagnostics"
                    notification::PublishDiagnostics::METHOD.to_string(),
                    PublishDiagnosticsParams {
                        uri,
                        version,
                        diagnostics,
                    },
                )))
                .unwrap();
        }
    }
}

// -----------------------------------------------
// Util
// -----------------------------------------------

/// リクエストをidとパラメータに変換する
fn cast_req<T>(
    req: lsp_server::Request,
) -> Result<(RequestId, T::Params), ExtractError<lsp_server::Request>>
where
    T: request::Request,
    T::Params: serde::de::DeserializeOwned,
{
    req.extract(T::METHOD)
}

/// 通知をパラメータに変換する
fn cast_nn<T>(
    nn: lsp_server::Notification,
) -> Result<T::Params, ExtractError<lsp_server::Notification>>
where
    T: notification::Notification,
    T::Params: serde::de::DeserializeOwned,
{
    nn.extract(T::METHOD)
}

fn new_ok_response<T: Serialize>(id: RequestId, result: T) -> lsp_server::Message {
    Message::Response(lsp_server::Response::new_ok(id, result))
}
