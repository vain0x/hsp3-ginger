pub(super) mod lsp_config;
pub(crate) mod lsp_log;
pub(super) mod lsp_main_v2;

pub(self) use lsp_config::LspConfig;

/// テキストドキュメントのバージョン番号
///
/// (エディタ上で編集されるたびに変わる番号。
///  いつの状態のテキストドキュメントを指しているかを明確にするためのもの)
pub(crate) type TextDocumentVersion = i32;

pub(crate) const NO_VERSION: TextDocumentVersion = 1;
