pub(super) mod lsp_config;
pub(super) mod lsp_handler;
pub(super) mod lsp_main;
pub(super) mod lsp_main_v2;
pub(super) mod lsp_receiver;
pub(super) mod lsp_sender;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(self) use lsp_config::LspConfig;
pub(self) use lsp_handler::LspHandler;
pub(self) use lsp_receiver::LspReceiver;
pub(self) use lsp_sender::LspSender;

/// テキストドキュメントのバージョン番号
///
/// (エディタ上で編集されるたびに変わる番号。
///  いつの状態のテキストドキュメントを指しているかを明確にするためのもの)
pub(crate) type TextDocumentVersion = i32;

pub(crate) const NO_VERSION: TextDocumentVersion = 1;

/// サーバーからクライアントに送るメッセージ
pub(super) enum Outgoing<T> {
    Request {
        id: Value,
        method: String,
        params: T,
    },
    Notification {
        method: String,
        params: T,
    },
    Response {
        id: Value,
        result: T,
    },
    Error {
        id: Option<Value>,
        code: i64,
        msg: String,
        data: T,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct LspRequest<Params> {
    pub(crate) jsonrpc: String,
    pub(crate) id: Value,
    pub(crate) method: String,
    pub(crate) params: Params,
}

#[derive(Serialize, Deserialize)]
pub(super) struct LspResponse<Result> {
    pub(crate) jsonrpc: String,
    pub(crate) id: Value,
    pub(crate) result: Result,
}

#[derive(Serialize, Deserialize)]
pub(super) struct LspErrorResponse<Data> {
    pub(crate) jsonrpc: String,
    pub(crate) id: Value,
    pub(crate) error: LspError<Data>,
}

/// <https://microsoft.github.io/language-server-protocol/specifications/specification-current/#responseMessage>
#[derive(Serialize, Deserialize)]
pub(super) struct LspError<Data> {
    pub(crate) code: i64,
    pub(crate) msg: String,
    pub(crate) data: Data,
}

#[derive(Serialize, Deserialize)]
pub(super) struct LspNotification<Params> {
    pub(crate) jsonrpc: String,
    pub(crate) method: String,
    pub(crate) params: Params,
}

/// LSP message (request or notification) without results/params
/// just for deserialization.
#[derive(Deserialize)]
pub(super) struct LspMessageOpaque {
    pub(crate) method: Option<String>,
    pub(crate) id: Option<Value>,
}

pub(crate) mod error {
    pub(crate) const METHOD_NOT_FOUND: i64 = -32601;
}
