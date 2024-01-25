// `initialize` リクエスト

use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InitConfig {
    pub(super) document_symbol: DocumentSymbol,
}

#[derive(Deserialize)]
pub(super) struct DocumentSymbol {
    pub(super) enabled: bool,
}

impl Default for DocumentSymbol {
    fn default() -> Self {
        Self { enabled: true }
    }
}
