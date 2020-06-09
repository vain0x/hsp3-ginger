pub(super) mod lsp_handler;
pub(super) mod lsp_main;
pub(super) mod lsp_receiver;
pub(super) mod lsp_sender;

use serde::{Deserialize, Serialize};

pub(self) use lsp_handler::LspHandler;
pub(self) use lsp_receiver::LspReceiver;
pub(self) use lsp_sender::LspSender;

#[derive(Serialize, Deserialize)]
pub(super) struct LspRequest<Params> {
    pub(crate) jsonrpc: String,
    pub(crate) id: i64,
    pub(crate) method: String,
    pub(crate) params: Params,
}

#[derive(Serialize, Deserialize)]
pub(super) struct LspResponse<Result> {
    pub(crate) jsonrpc: String,
    pub(crate) id: i64,
    pub(crate) result: Result,
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
    pub(crate) method: String,
}
