pub(crate) struct AnalyzerOptions {
    pub(crate) lint_enabled: bool,
}

impl AnalyzerOptions {
    #[cfg(test)]
    pub(crate) fn minimal() -> Self {
        Self {
            lint_enabled: false,
        }
    }
}

impl Default for AnalyzerOptions {
    fn default() -> Self {
        Self { lint_enabled: true }
    }
}
