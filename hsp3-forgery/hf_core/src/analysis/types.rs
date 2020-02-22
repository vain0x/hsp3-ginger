pub(crate) struct SignatureHelp {
    pub(crate) command: String,
    pub(crate) params: Vec<String>,
    pub(crate) active_param_index: usize,
}
