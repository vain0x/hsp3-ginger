use super::*;

#[derive(Default)]
pub(crate) struct DocAnalysis {
    // 構文:
    pub(crate) tokens: RcSlice<PToken>,
    pub(crate) tree_opt: Option<PRoot>,

    // プリプロセス:
    pub(crate) includes: Vec<(RcStr, Loc)>,
    pub(crate) modules: HashMap<AModule, AModuleData>,
    pub(crate) deffuncs: HashMap<ADefFunc, ADefFuncData>,
    pub(crate) preproc_symbols: Vec<SymbolRc>,

    // 構文リント:
    pub(crate) syntax_lint_done: bool,
    pub(crate) syntax_lints: Vec<(SyntaxLint, Loc)>,
}

impl DocAnalysis {
    pub(crate) fn invalidate(&mut self) {
        self.tokens = [].into();
        self.tree_opt = None;
        self.includes.clear();
        self.modules.clear();
        self.deffuncs.clear();
        self.preproc_symbols.clear();
        self.syntax_lint_done = false;
        self.syntax_lints.clear();
    }

    pub(crate) fn set_syntax(&mut self, tokens: RcSlice<PToken>, tree: PRoot) {
        self.tokens = tokens;
        self.tree_opt = Some(tree);
    }

    pub(crate) fn set_preproc(&mut self, preproc: PreprocAnalysisResult) {
        self.includes = preproc.includes;
        self.modules = preproc.modules;
        self.deffuncs = preproc.deffuncs;
        self.preproc_symbols = preproc.symbols;
    }
}
