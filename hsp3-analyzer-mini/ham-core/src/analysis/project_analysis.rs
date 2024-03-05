use self::workspace_analysis::DocAnalysisMap;
use super::*;

pub(crate) enum EntryPoints {
    NonCommon,
    Docs(Vec<DocId>),
}

impl Default for EntryPoints {
    fn default() -> Self {
        EntryPoints::NonCommon
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ProjectAnalysisRef<'a> {
    project: &'a ProjectAnalysis,
}

#[derive(Default)]
pub(crate) struct ProjectAnalysis {
    // 入力:
    pub(crate) entrypoints: EntryPoints,
    pub(super) common_docs: Rc<HashMap<String, DocId>>,
    pub(super) hsphelp_info: Rc<HspHelpInfo>,
    pub(super) project_docs: Rc<ProjectDocs>,
    pub(super) active_docs: HashSet<DocId>,
    pub(super) active_help_docs: HashSet<DocId>,
    // common doc -> hsphelp doc
    pub(super) help_docs: HashMap<DocId, DocId>,

    // 解析結果:
    pub(super) public_env: PublicEnv,
    pub(super) ns_env: HashMap<RcStr, SymbolEnv>,
    pub(super) doc_symbols_map: HashMap<DocId, Vec<SymbolRc>>,
    pub(super) def_sites: Vec<(SymbolRc, Loc)>,
    pub(super) use_sites: Vec<(SymbolRc, Loc)>,

    /// (loc, doc): locにあるincludeがdocに解決されたことを表す。
    pub(super) include_resolution: Vec<(Loc, DocId)>,

    pub(super) diagnostics: Vec<(String, Loc)>,
}

impl ProjectAnalysis {
    // NOTE: プロジェクトシステムの移行中。ここに計算処理はもうない
    pub(crate) fn compute<'a>(
        &'a self,
        #[allow(unused)] doc_analysis_map: &'a DocAnalysisMap,
    ) -> ProjectAnalysisRef<'a> {
        ProjectAnalysisRef { project: self }
    }
}

impl<'a> ProjectAnalysisRef<'a> {
    pub(crate) fn locate_symbol(self, doc: DocId, pos: Pos16) -> Option<(SymbolRc, Loc)> {
        self.project
            .def_sites
            .iter()
            .chain(&self.project.use_sites)
            .find(|&(_, loc)| loc.is_touched(doc, pos))
            .cloned()
    }

    pub(crate) fn collect_symbol_defs(self, symbol: &SymbolRc, locs: &mut Vec<Loc>) {
        for &(ref s, loc) in &self.project.def_sites {
            if s == symbol {
                locs.push(loc);
            }
        }
    }

    pub(crate) fn collect_symbol_uses(self, symbol: &SymbolRc, locs: &mut Vec<Loc>) {
        for &(ref s, loc) in &self.project.use_sites {
            if s == symbol {
                locs.push(loc);
            }
        }
    }
}
