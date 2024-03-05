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
    pub(super) def_sites: Vec<(SymbolRc, Loc)>,
    pub(super) use_sites: Vec<(SymbolRc, Loc)>,
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
