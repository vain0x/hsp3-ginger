use super::*;
use crate::{
    analyzer::{options::AnalyzerOptions, Analyzer},
    lsp_server::lsp_main::init_log,
};
use lsp_server::{Connection, ExtractError, Message, RequestId, Response};
use lsp_types::{
    notification::{self, Notification as _},
    request::{self, Request as _},
    OneOf,
};
use std::{env, path::PathBuf};

#[allow(unused)]
pub fn run_lsp_server(hsp3_root: PathBuf) {
    init_log();

    trace!("run_lsp_server(v2), hsp3_root={:?}", hsp3_root);

    // options from env:
    // let lsp_config = LspConfig {
    //     document_symbol_enabled: env::var("HAM_DOCUMENT_SYMBOL_ENABLED").map_or(true, |s| s == "1"),
    //     watcher_enabled: env::var("HAM_WATCHER_ENABLED").map_or(true, |s| s == "1"),
    // };
    let options = AnalyzerOptions {
        lint_enabled: env::var("HAM_LINT").map_or(true, |s| s == "1"),
    };

    let mut an = Analyzer::new(hsp3_root, options);

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (cx, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(lsp_types::ServerCapabilities {
        definition_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })
    .unwrap();

    // LSPサーバーの初期化処理を行う
    // ("initialized" 通知が来るまで通信が進み、"initialize" リクエストのパラメータが返される)
    let init_params = match cx.initialize(server_capabilities) {
        Ok(value) => serde_json::from_value::<lsp_types::InitializeParams>(value).unwrap(),
        Err(e) => {
            error!("initialize");
            if e.channel_is_disconnected() {
                io_threads.join().unwrap();
            }
            return;
        }
    };

    if let Some(folders) = init_params.workspace_folders {
        for folder in folders {
            an.add_workspace_folder(folder);
        }
    }

    an.did_initialize();

    trace!("starting example main loop");
    for msg in &cx.receiver {
        match msg {
            Message::Request(req) => {
                trace!("got request: {req:?}");

                // "shutdown" リクエストなら `true` になる。
                // ("exit" 通知が来るまで通信が行われる)
                if cx.handle_shutdown(&req).unwrap() {
                    break;
                }

                dispatch_request(&cx, &mut an, req);
                continue;
            }
            Message::Response(resp) => {
                trace!("got response: {resp:?}");
                continue;
            }
            Message::Notification(nn) => {
                trace!("got notification: {nn:?}");

                // `handle_shutdown` によって処理されるため
                debug_assert!(nn.method != notification::Exit::METHOD);

                dispatch_notification(&cx, &mut an, nn);
                continue;
            }
        }
    }

    io_threads.join().unwrap();
    info!("graceful exit");
}

/// リクエストを処理する
fn dispatch_request(cx: &lsp_server::Connection, an: &mut Analyzer, req: lsp_server::Request) {
    match req.method.as_str() {
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

            cx.sender
                .send(Message::Response(Response::new_ok(id, result)))
                .unwrap();
            return;
        }
        _ => {
            // 未実装のリクエストにエラーレスポンスを返す
            cx.sender
                .send(Message::Response(Response::new_err(
                    req.id,
                    error::METHOD_NOT_FOUND as i32,
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
            return;
        }
        // "testDocument/didClose"
        notification::DidCloseTextDocument::METHOD => {
            let params = cast_nn::<notification::DidCloseTextDocument>(nn).unwrap();
            let d = params.text_document;

            an.close_doc(d.uri);
            // self.diagnostics_invalidated = true;
        }
        _ if nn.method.starts_with("$/") => {
            // "$/" で始まるメソッド名の通知は暗黙に無視してよい
            trace!("Notification ignored: {:?}", nn.method);
        }
        _ => {
            trace!("Notification supported ({})", nn.method);
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
