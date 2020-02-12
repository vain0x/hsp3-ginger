use super::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct LspRequest<Params> {
    pub(crate) jsonrpc: String,
    pub(crate) id: i64,
    pub(crate) method: String,
    pub(crate) params: Params,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LspResponse<Result> {
    pub(crate) jsonrpc: String,
    pub(crate) id: i64,
    pub(crate) result: Result,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LspNotification<Params> {
    pub(crate) jsonrpc: String,
    pub(crate) method: String,
    pub(crate) params: Params,
}

/// LSP message (request or notification) without results/params
/// just for deserialization.
#[derive(Deserialize)]
pub(crate) struct LspMessageOpaque {
    pub(crate) method: String,
}
