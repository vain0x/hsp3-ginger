use super::*;

type DocAnalysisMap = HashMap<DocId, DocAnalysis>;

pub(crate) enum EntryPoints {
    Docs(Vec<DocId>),

    #[allow(unused)]
    NonCommon,
}

impl Default for EntryPoints {
    fn default() -> Self {
        EntryPoints::Docs(vec![])
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

    // 解析結果:
    computed: bool,
    pub(super) active_docs: HashSet<DocId>,
    pub(super) active_help_docs: HashSet<DocId>,
    // common doc -> hsphelp doc
    pub(super) help_docs: HashMap<DocId, DocId>,
    pub(super) public_env: PublicEnv,
    pub(super) ns_env: HashMap<RcStr, SymbolEnv>,
    pub(super) doc_symbols_map: HashMap<DocId, Vec<SymbolRc>>,
    pub(super) def_sites: Vec<(SymbolRc, Loc)>,
    pub(super) use_sites: Vec<(SymbolRc, Loc)>,

    /// (loc, doc): locにあるincludeがdocに解決されたことを表す。
    pub(super) include_resolution: Vec<(Loc, DocId)>,

    diagnosed: bool,
    pub(super) diagnostics: Vec<(String, Loc)>,
}

impl ProjectAnalysis {
    pub(crate) fn invalidate(&mut self) {
        self.computed = false;
        self.active_docs.clear();
        self.active_help_docs.clear();
        self.help_docs.clear();
        self.public_env.clear();
        self.ns_env.clear();
        self.doc_symbols_map.clear();
        self.def_sites.clear();
        self.use_sites.clear();
        self.include_resolution.clear();

        self.diagnosed = false;
        self.diagnostics.clear();
    }

    pub(crate) fn is_computed(&self) -> bool {
        self.computed
    }

    fn compute_active_docs(&mut self, doc_analysis_map: &DocAnalysisMap) {
        let entrypoints = &self.entrypoints;
        let common_docs = self.common_docs.as_ref();
        let hsphelp_info = self.hsphelp_info.as_ref();
        let project_docs = self.project_docs.as_ref();
        let active_docs = &mut self.active_docs;
        let help_docs = &mut self.help_docs;
        let active_help_docs = &mut self.active_help_docs;
        let include_resolution = &mut self.include_resolution;
        let diagnostics = &mut self.diagnostics;

        match entrypoints {
            EntryPoints::Docs(entrypoints) => {
                assert_ne!(entrypoints.len(), 0);

                // エントリーポイントから推移的にincludeされるドキュメントを集める。
                let mut stack = entrypoints
                    .iter()
                    .map(|&doc| (doc, None))
                    .collect::<Vec<_>>();
                active_docs.extend(entrypoints.iter().cloned());

                while let Some((doc, _)) = stack.pop() {
                    debug_assert!(active_docs.contains(&doc));
                    let da = match doc_analysis_map.get(&doc) {
                        Some(it) => it,
                        None => continue,
                    };

                    for &(ref path, loc) in &da.includes {
                        let path = path.as_str();
                        let doc_opt = project_docs
                            .find(path, Some(loc.doc))
                            .or_else(|| common_docs.get(path).cloned());
                        let d = match doc_opt {
                            Some(it) => it,
                            None => {
                                diagnostics.push((
                                    format!("includeを解決できません: {:?}", path),
                                    loc.clone(),
                                ));
                                continue;
                            }
                        };
                        include_resolution.push((loc, d));
                        if active_docs.insert(d) {
                            stack.push((d, Some(loc)));
                        }
                    }
                }
            }
            EntryPoints::NonCommon => {
                // includeされていないcommonのファイルだけ除外する。

                let mut included_docs = HashSet::new();
                let in_common = common_docs.values().cloned().collect::<HashSet<_>>();

                for (&doc, da) in doc_analysis_map.iter() {
                    if in_common.contains(&doc) {
                        continue;
                    }

                    for (include, _) in &da.includes {
                        let doc_opt = self.common_docs.get(include.as_str()).cloned();
                        included_docs.extend(doc_opt);
                    }
                }

                active_docs.extend(
                    doc_analysis_map
                        .keys()
                        .cloned()
                        .filter(|doc| !in_common.contains(&doc) || included_docs.contains(&doc)),
                );
            }
        }

        // hsphelp
        {
            trace!("active_help_docs.len={}", active_help_docs.len());
            active_help_docs.extend(hsphelp_info.builtin_docs.iter().cloned());

            for (&common_doc, &hs_doc) in &hsphelp_info.linked_docs {
                if active_docs.contains(&common_doc) {
                    active_help_docs.insert(hs_doc);
                    help_docs.insert(common_doc, hs_doc);
                }
            }
        }
    }

    fn compute_symbols(&mut self, doc_analysis_map: &DocAnalysisMap, module_map: &ModuleMap) {
        let active_docs = &self.active_docs;
        let public_env = &mut self.public_env;
        let ns_env = &mut self.ns_env;
        let doc_symbols_map = &mut self.doc_symbols_map;
        let def_sites = &mut self.def_sites;
        let use_sites = &mut self.use_sites;

        // 複数ファイルに渡る環境を構築する。
        for (&doc, da) in doc_analysis_map.iter() {
            if !active_docs.contains(&doc) {
                continue;
            }

            extend_public_env_from_symbols(&da.preproc_symbols, public_env, ns_env);
        }

        // 変数の定義箇所を決定する。
        doc_symbols_map.extend(
            doc_analysis_map
                .iter()
                .filter(|(&doc, _)| active_docs.contains(&doc))
                .map(|(&doc, da)| (doc, da.preproc_symbols.clone())),
        );

        for (&doc, da) in doc_analysis_map.iter() {
            if !active_docs.contains(&doc) {
                continue;
            }

            let symbols = doc_symbols_map.get_mut(&doc).unwrap();

            def_sites.extend(symbols.iter().filter_map(|symbol| {
                let loc = symbol.preproc_def_site_opt?;
                Some((symbol.clone(), loc))
            }));

            crate::analysis::var::analyze_var_def(
                doc,
                da.tree_opt.as_ref().unwrap(),
                &module_map,
                symbols,
                public_env,
                ns_env,
                def_sites,
                use_sites,
            );

            // ヘルプファイルの情報をシンボルに統合する。
            if let Some(hs_doc) = self.help_docs.get(&doc) {
                if let Some(hs_symbols) = self.hsphelp_info.doc_symbols.get(&hs_doc) {
                    let mut hs_symbols_map = hs_symbols
                        .iter()
                        .map(|s| (s.label.as_str(), s.clone()))
                        .collect::<HashMap<_, _>>();

                    for symbol in symbols {
                        let mut link_opt = symbol.linked_symbol_opt.borrow_mut();
                        if link_opt.is_some() {
                            continue;
                        }

                        let s = match hs_symbols_map.remove(symbol.name.as_str()) {
                            Some(it) => it,
                            None => continue,
                        };

                        *link_opt = Some(s.clone());
                    }
                }
            }
        }
    }

    pub(crate) fn compute<'a>(
        &'a mut self,
        doc_analysis_map: &'a DocAnalysisMap,
        module_map: &ModuleMap,
    ) -> ProjectAnalysisRef<'a> {
        if self.computed {
            return ProjectAnalysisRef {
                doc_analysis_map,
                project: self,
            };
        }
        self.computed = true;

        self.compute_active_docs(doc_analysis_map);
        self.compute_symbols(doc_analysis_map, module_map);

        // デバッグ用: 集計を出す。
        let total_symbol_count = self
            .doc_symbols_map
            .values()
            .map(|symbols| symbols.len())
            .sum::<usize>();
        trace!(
            "computed: active_docs={} def_sites={} use_sites={} symbols={}",
            self.active_docs.len(),
            self.def_sites.len(),
            self.use_sites.len(),
            total_symbol_count
        );

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
                kind: Some(K::Keyword),
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

    pub(crate) fn collect_all_symbols(self, name_filter: &str, symbols: &mut Vec<(SymbolRc, Loc)>) {
        let p = self.project;

        let name_filter = name_filter.trim().to_ascii_lowercase();

        let map = p
            .def_sites
            .iter()
            .filter(|(symbol, _)| symbol.name.contains(&name_filter))
            .map(|(symbol, loc)| (symbol.clone(), *loc))
            .collect::<HashMap<_, _>>();

        for (&doc, doc_symbols) in &p.doc_symbols_map {
            if !p.active_docs.contains(&doc) {
                continue;
            }

            for symbol in doc_symbols {
                if !symbol.name.contains(&name_filter) {
                    continue;
                }

                let def_site = match map.get(symbol) {
                    Some(it) => *it,
                    None => continue,
                };

                symbols.push((symbol.clone(), def_site));
            }
        }
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
