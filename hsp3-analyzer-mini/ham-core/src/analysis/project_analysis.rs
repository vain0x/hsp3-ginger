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
    doc_analysis_map: &'a DocAnalysisMap,
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
        doc_analysis_map: &'a DocAnalysisMap,
    ) -> ProjectAnalysisRef<'a> {
        ProjectAnalysisRef {
            doc_analysis_map,
            project: self,
        }
    }
}

impl<'a> ProjectAnalysisRef<'a> {
    fn syntax_tree(self, doc: DocId) -> Option<&'a PRoot> {
        Some(self.doc_analysis_map.get(&doc)?.tree_opt.as_ref()?)
    }

    pub(crate) fn get_signature_help_context(
        self,
        doc: DocId,
        pos: Pos16,
    ) -> Option<SignatureHelpContext> {
        let tree = self.syntax_tree(doc)?;

        let use_site_map = self
            .project
            .use_sites
            .iter()
            .filter_map(|&(ref symbol, loc)| {
                if loc.doc == doc {
                    Some((loc.start(), symbol.clone()))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let mut h = SignatureHelpHost { use_site_map };
        h.process(pos, tree)
    }

    pub(crate) fn locate_symbol(self, doc: DocId, pos: Pos16) -> Option<(SymbolRc, Loc)> {
        self.project
            .def_sites
            .iter()
            .chain(&self.project.use_sites)
            .find(|&(_, loc)| loc.is_touched(doc, pos))
            .cloned()
    }

    pub(crate) fn get_symbol_details(
        self,
        symbol: &SymbolRc,
    ) -> Option<(RcStr, &'static str, SymbolDetails)> {
        Some((
            symbol.name(),
            symbol.kind.as_str(),
            symbol.compute_details(),
        ))
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

    pub(crate) fn collect_symbol_occurrences(self, symbols: &mut Vec<(&'a SymbolRc, Loc)>) {
        for (symbol, loc) in self.project.def_sites.iter().chain(&self.project.use_sites) {
            symbols.push((symbol, *loc));
        }
    }

    pub(crate) fn collect_completion_items(
        self,
        doc: DocId,
        pos: Pos16,
        completion_items: &mut Vec<ACompletionItem>,
    ) {
        let doc_analysis_map = self.doc_analysis_map;
        let p = self.project;

        let scope = match doc_analysis_map.get(&doc) {
            Some(da) => resolve_scope_at(&da.module_map, &da.deffunc_map, pos),
            None => LocalScope::default(),
        };

        let doc_symbols = p
            .doc_symbols_map
            .iter()
            .filter_map(|(&d, symbols)| {
                if d == doc || p.active_docs.contains(&d) {
                    Some((d, symbols.as_slice()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        collect_symbols_as_completion_items(doc, scope, &doc_symbols, completion_items);
    }

    // FIXME: lsp_typesをここで使うべきではない
    pub(crate) fn collect_hsphelp_completion_items(
        self,
        completion_items: &mut Vec<lsp_types::CompletionItem>,
    ) {
        let p = self.project;

        completion_items.extend(
            p.hsphelp_info
                .doc_symbols
                .iter()
                .filter(|(&doc, _)| p.active_help_docs.contains(&doc))
                .flat_map(|(_, symbols)| symbols.iter().filter(|s| !s.label.starts_with("#")))
                .cloned(),
        );
    }

    // FIXME: lsp_typesをここで使うべきではない
    pub(crate) fn collect_preproc_completion_items(
        self,
        completion_items: &mut Vec<lsp_types::CompletionItem>,
    ) {
        let p = self.project;

        for (keyword, detail) in &[
            ("ctype", "関数形式のマクロを表す"),
            ("global", "グローバルスコープを表す"),
            ("local", "localパラメータ、またはローカルスコープを表す"),
            ("int", "整数型のパラメータ、または整数型の定数を表す"),
            ("double", "実数型のパラメータ、または実数型の定数を表す"),
            ("str", "文字列型のパラメータを表す"),
            ("label", "ラベル型のパラメータを表す"),
            ("var", "変数 (配列要素) のパラメータを表す"),
            ("array", "配列変数のパラメータを表す"),
        ] {
            use lsp_types::{CompletionItem as CI, CompletionItemKind as K};
            let sort_prefix = 'a';
            completion_items.push(CI {
                kind: Some(K::KEYWORD),
                label: keyword.to_string(),
                detail: Some(detail.to_string()),
                sort_text: Some(format!("{}{}", sort_prefix, keyword)),
                ..CI::default()
            });
        }

        completion_items.extend(
            p.hsphelp_info
                .doc_symbols
                .iter()
                .filter(|(&doc, _)| p.active_help_docs.contains(&doc))
                .flat_map(|(_, symbols)| symbols.iter().filter(|s| s.label.starts_with("#")))
                .cloned(),
        );
    }

    pub(crate) fn collect_doc_symbols(self, doc: DocId, symbols: &mut Vec<(SymbolRc, Loc)>) {
        let p = self.project;

        let doc_symbols = match p.doc_symbols_map.get(&doc) {
            Some(it) => it,
            None => return,
        };

        let def_site_map = p
            .def_sites
            .iter()
            .filter(|(_, loc)| loc.doc == doc)
            .cloned()
            .collect::<HashMap<_, _>>();

        symbols.extend(doc_symbols.iter().filter_map(|symbol| {
            let loc = def_site_map.get(&symbol)?;
            Some((symbol.clone(), *loc))
        }));
    }

    pub(crate) fn find_include_target(self, doc: DocId, pos: Pos16) -> Option<DocId> {
        let p = self.project;
        let (_, dest_doc) = *p
            .include_resolution
            .iter()
            .find(|&(loc, _)| loc.is_touched(doc, pos))?;

        Some(dest_doc)
    }
}

fn resolve_scope_at(module_map: &ModuleMap, deffunc_map: &DefFuncMap, pos: Pos16) -> LocalScope {
    let mut scope = LocalScope::default();

    scope.module_opt = module_map.iter().find_map(|(&m, module_data)| {
        if range_is_touched(&module_data.content_loc.range, pos) {
            Some(m.clone())
        } else {
            None
        }
    });

    scope.deffunc_opt = deffunc_map.iter().find_map(|(&d, deffunc_data)| {
        if range_is_touched(&deffunc_data.content_loc.range, pos) {
            Some(d)
        } else {
            None
        }
    });

    scope
}
