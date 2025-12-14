#[derive(Debug, Default)]
pub(crate) struct LspConfig {
    pub(crate) document_symbol_enabled: bool,
    pub(crate) lint_enabled: bool,
    pub(crate) watcher_enabled: bool,
}
