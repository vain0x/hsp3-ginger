pub(super) mod features;
pub(super) mod lsp_handler;
pub(super) mod lsp_main;
pub(super) mod lsp_model;
pub(super) mod lsp_receiver;
pub(super) mod lsp_sender;

use serde::{Deserialize, Serialize};

pub(self) use lsp_handler::LspHandler;
pub(self) use lsp_model::LspModel;
pub(self) use lsp_receiver::LspReceiver;
pub(self) use lsp_sender::LspSender;

#[derive(Serialize, Deserialize)]
pub(super) struct LspRequest<Params> {
    pub jsonrpc: String,
    pub id: i64,
    pub method: String,
    pub params: Params,
}

#[derive(Serialize, Deserialize)]
pub(super) struct LspResponse<Result> {
    pub jsonrpc: String,
    pub id: i64,
    pub result: Result,
}

#[derive(Serialize, Deserialize)]
pub(super) struct LspNotification<Params> {
    pub jsonrpc: String,
    pub method: String,
    pub params: Params,
}

/// LSP message (request or notification) without results/params
/// just for deserialization.
#[derive(Deserialize)]
pub(super) struct LspMessageOpaque {
    pub method: String,
}
