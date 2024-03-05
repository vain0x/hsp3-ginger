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
    pub(super) def_sites: &'a [(SymbolRc, Loc)],
    pub(super) use_sites: &'a [(SymbolRc, Loc)],
}

impl<'a> ProjectAnalysisRef<'a> {
    pub(crate) fn locate_symbol(self, doc: DocId, pos: Pos16) -> Option<(SymbolRc, Loc)> {
        self.def_sites
            .iter()
            .chain(self.use_sites)
            .find(|&(_, loc)| loc.is_touched(doc, pos))
            .cloned()
    }

    pub(crate) fn collect_symbol_defs(self, symbol: &SymbolRc, locs: &mut Vec<Loc>) {
        for &(ref s, loc) in self.def_sites {
            if s == symbol {
                locs.push(loc);
            }
        }
    }

    pub(crate) fn collect_symbol_uses(self, symbol: &SymbolRc, locs: &mut Vec<Loc>) {
        for &(ref s, loc) in self.use_sites {
            if s == symbol {
                locs.push(loc);
            }
        }
    }
}
