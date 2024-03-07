use super::*;

pub(crate) type DocAnalysisMap = HashMap<DocId, DocAnalysis>;

/// ワークスペースの外側のデータ
#[derive(Default)]
pub(crate) struct WorkspaceHost {
    pub(crate) builtin_env: Rc<SymbolEnv>,
    pub(crate) common_docs: Rc<HashMap<String, DocId>>,
    pub(crate) hsphelp_info: Rc<HspHelpInfo>,
    pub(crate) entrypoints: EntryPoints,
}

#[derive(Default)]
pub(crate) struct WorkspaceAnalysis {
    // state:
    dirty_docs: HashSet<DocId>,

    // input:
    doc_texts: HashMap<DocId, (Lang, RcStr)>,

    common_docs: Rc<HashMap<String, DocId>>,
    hsphelp_info: Rc<HspHelpInfo>,
    entrypoints: EntryPoints,
    pub(super) project_docs: Rc<ProjectDocs>,

    // computed:
    pub(super) active_docs: HashSet<DocId>,
    pub(super) active_help_docs: HashSet<DocId>,
    pub(super) help_docs: HashMap<DocId, DocId>,

    pub(super) public_env: PublicEnv,
    pub(super) ns_env: HashMap<RcStr, SymbolEnv>,
    pub(super) doc_symbols_map: HashMap<DocId, Vec<SymbolRc>>,
    pub(crate) def_sites: Vec<(SymbolRc, Loc)>,
    pub(crate) use_sites: Vec<(SymbolRc, Loc)>,

    /// (loc, doc): locにあるincludeがdocに解決されたことを表す。
    pub(super) include_resolution: Vec<(Loc, DocId)>,

    pub(super) diagnostics: Vec<(String, Loc)>,

    // すべてのドキュメントの解析結果を使って構築される情報:
    pub(crate) doc_analysis_map: DocAnalysisMap,
    module_map: ModuleMap,
}

pub(crate) struct AnalysisRef<'a> {
    // input:
    doc_texts: &'a HashMap<DocId, (Lang, RcStr)>,

    hsphelp_info: &'a HspHelpInfo,
    pub(super) project_docs: &'a ProjectDocs,

    // computed:
    pub(super) active_docs: &'a HashSet<DocId>,
    pub(super) active_help_docs: &'a HashSet<DocId>,

    pub(super) doc_symbols_map: &'a HashMap<DocId, Vec<SymbolRc>>,
    pub(super) def_sites: &'a [(SymbolRc, Loc)],
    pub(super) use_sites: &'a [(SymbolRc, Loc)],

    // すべてのドキュメントの解析結果を使って構築される情報:
    pub(super) doc_analysis_map: &'a DocAnalysisMap,
}

impl WorkspaceAnalysis {
    pub(crate) fn initialize(&mut self, host: WorkspaceHost) {
        let WorkspaceHost {
            common_docs,
            hsphelp_info,
            builtin_env,
            entrypoints,
        } = host;

        self.common_docs = common_docs;
        self.hsphelp_info = hsphelp_info;
        self.entrypoints = entrypoints;

        self.public_env.builtin = builtin_env;
    }

    pub(crate) fn update_doc(&mut self, doc: DocId, lang: Lang, text: RcStr) {
        self.dirty_docs.insert(doc);
        self.doc_texts.insert(doc, (lang, text));
        self.doc_analysis_map
            .entry(doc)
            .and_modify(|a| a.invalidate());
    }

    pub(crate) fn close_doc(&mut self, doc: DocId) {
        self.dirty_docs.insert(doc);
        self.doc_texts.remove(&doc);
        self.doc_analysis_map.remove(&doc);
    }

    pub(crate) fn set_project_docs(&mut self, project_docs: ProjectDocs) {
        self.project_docs = Rc::new(project_docs);
    }

    /// 未実行の解析処理があるなら行う
    fn compute(&mut self) {
        if self.dirty_docs.is_empty() {
            return;
        }

        // invalidate:
        {
            self.active_docs.clear();
            self.active_help_docs.clear();
            self.help_docs.clear();
            self.public_env.clear();
            self.ns_env.clear();
            self.doc_symbols_map.clear();
            self.def_sites.clear();
            self.use_sites.clear();
            self.include_resolution.clear();
            self.diagnostics.clear();
        }

        // compute:
        let mut doc_analysis_map = take(&mut self.doc_analysis_map);
        self.module_map.clear();

        for doc in self.dirty_docs.drain() {
            let (lang, text) = match self.doc_texts.get(&doc) {
                Some(it) => it,
                None => continue,
            };

            match lang {
                Lang::HelpSource => {
                    // todo
                    continue;
                }
                Lang::Hsp3 => {}
            }

            let tokens = crate::token::tokenize(doc, text.clone());
            let p_tokens: RcSlice<_> = PToken::from_tokens(tokens.into()).into();
            let root = crate::parse::parse_root(p_tokens.to_owned());
            let preproc = crate::analysis::preproc::analyze_preproc(doc, &root);

            let da = doc_analysis_map.entry(doc).or_default();
            da.set_syntax(p_tokens, root);
            da.set_preproc(preproc);

            self.module_map
                .extend(da.module_map.iter().map(|(&m, rc)| (m, rc.clone())));
        }

        self.doc_analysis_map = doc_analysis_map;

        // NOTE: プロジェクトシステムの移行中
        {
            compute_active_docs::compute_active_docs(
                &self.doc_analysis_map,
                &self.entrypoints,
                &self.common_docs,
                &self.hsphelp_info,
                &self.project_docs,
                &mut self.active_docs,
                &mut self.active_help_docs,
                &mut self.help_docs,
                &mut self.include_resolution,
            );

            compute_symbols::compute_symbols(
                &self.hsphelp_info,
                &self.active_docs,
                &self.help_docs,
                &self.doc_analysis_map,
                &self.module_map,
                &mut self.public_env,
                &mut self.ns_env,
                &mut self.doc_symbols_map,
                &mut self.def_sites,
                &mut self.use_sites,
            );
        }

        // デバッグ用: 集計を出す。
        {
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
        }

        assert_eq!(self.diagnostics.len(), 0);
    }

    pub(crate) fn get_analysis(&self) -> AnalysisRef<'_> {
        AnalysisRef {
            doc_texts: &self.doc_texts,
            hsphelp_info: &self.hsphelp_info,
            project_docs: &self.project_docs,
            active_docs: &self.active_docs,
            active_help_docs: &self.active_help_docs,
            doc_symbols_map: &self.doc_symbols_map,
            def_sites: &self.def_sites,
            use_sites: &self.use_sites,
            doc_analysis_map: &self.doc_analysis_map,
        }
    }

    pub(crate) fn compute_analysis(&mut self) -> AnalysisRef<'_> {
        self.compute();
        self.get_analysis()
    }
}

impl AnalysisRef<'_> {
    pub(crate) fn hsphelp_info(&self) -> &HspHelpInfo {
        &self.hsphelp_info
    }

    pub(crate) fn is_active_doc(&self, doc: DocId) -> bool {
        debug_assert!(!self.active_help_docs.contains(&doc));
        self.active_docs.contains(&doc)
    }

    pub(crate) fn is_active_help_doc(&self, doc: DocId) -> bool {
        debug_assert!(!self.active_docs.contains(&doc));
        self.active_help_docs.contains(&doc)
    }

    pub(crate) fn in_preproc(&self, doc: DocId, pos: Pos16) -> Option<bool> {
        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        Some(in_preproc(pos, tokens))
    }

    pub(crate) fn in_str_or_comment(&self, doc: DocId, pos: Pos16) -> Option<bool> {
        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        Some(in_str_or_comment(pos, tokens))
    }

    pub(crate) fn has_include_guard(&self, doc: DocId) -> bool {
        self.doc_analysis_map
            .get(&doc)
            .map_or(false, |da| da.include_guard.is_some())
    }

    pub(crate) fn on_include_guard(&self, doc: DocId, pos: Pos16) -> Option<Loc> {
        Some(
            self.doc_analysis_map
                .get(&doc)?
                .include_guard
                .as_ref()
                .filter(|g| g.loc.is_touched(doc, pos))?
                .loc,
        )
    }

    pub(crate) fn get_syntax(&self, doc: DocId) -> Option<DocSyntax> {
        let (_, text) = self
            .doc_texts
            .get(&doc)
            .filter(|&&(lang, _)| lang == Lang::Hsp3)?;
        let da = self.doc_analysis_map.get(&doc)?;
        Some(DocSyntax {
            text: text.clone(),
            tokens: da.tokens.clone(),
            root: da.tree_opt.as_ref()?,
        })
    }

    pub(crate) fn get_ident_at(&self, doc: DocId, pos: Pos16) -> Option<(RcStr, Loc)> {
        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        let token = match tokens.binary_search_by_key(&pos, |t| t.body_pos16()) {
            Ok(i) => tokens[i].body.as_ref(),
            Err(i) => tokens
                .iter()
                .skip(i.saturating_sub(1))
                .take(3)
                .find_map(|t| {
                    if t.body.kind == TokenKind::Ident && range_is_touched(&t.body.loc.range, pos) {
                        Some(t.body.as_ref())
                    } else {
                        None
                    }
                })?,
        };
        Some((token.text.clone(), token.loc))
    }

    pub(crate) fn require_project_for_doc(&self, _doc: DocId) -> ProjectAnalysisRef<'_> {
        ProjectAnalysisRef {
            def_sites: &self.def_sites,
            use_sites: &self.use_sites,
        }
    }

    pub(crate) fn diagnose(&self, diagnostics: &mut Vec<(String, Loc)>) {
        self.diagnose_precisely(diagnostics);
    }

    pub(crate) fn diagnose_syntax_lints(&self, lints: &mut Vec<(SyntaxLint, Loc)>) {
        for (&doc, da) in self.doc_analysis_map.iter() {
            if !self.is_active_doc(doc) {
                continue;
            }

            // if !da.syntax_lint_done {
            //     debug_assert_eq!(da.syntax_lints.len(), 0);
            //     let tree = or!(da.tree_opt.as_ref(), continue);
            //     crate::analysis::syntax_linter::syntax_lint(&tree, &mut da.syntax_lints);
            //     da.syntax_lint_done = true;
            // }
            // lints.extend(da.syntax_lints.iter().cloned());

            let tree = match &da.tree_opt {
                Some(it) => it,
                None => continue,
            };
            crate::analysis::syntax_linter::syntax_lint(&tree, lints);
        }
    }

    fn diagnose_precisely(&self, diagnostics: &mut Vec<(String, Loc)>) {
        // diagnose:

        let use_site_map = self
            .use_sites
            .iter()
            .map(|(symbol, loc)| ((loc.doc, loc.start()), symbol.clone()))
            .collect::<HashMap<_, _>>();

        let mut ctx = SemaLinter {
            use_site_map,
            diagnostics: vec![],
        };

        for (&doc, da) in self.doc_analysis_map.iter() {
            if !self.is_active_doc(doc) {
                continue;
            }

            let root = match &da.tree_opt {
                Some(it) => it,
                None => continue,
            };

            ctx.on_root(root);
        }

        diagnostics.extend(ctx.diagnostics.into_iter().map(|(d, loc)| {
            let msg = match d {
                Diagnostic::Undefined => "定義が見つかりません",
                Diagnostic::VarRequired => "変数か配列の要素が必要です。",
            }
            .to_string();
            (msg, loc)
        }));

        // diagnostics.extend(self.diagnostics.clone());
    }
}

pub(crate) struct DocSyntax<'a> {
    pub(crate) text: RcStr,
    pub(crate) tokens: RcSlice<PToken>,
    pub(crate) root: &'a PRoot,
}

/// シグネチャヘルプの生成に使うデータ
pub(crate) struct SignatureHelpDb {
    use_site_map: HashMap<Pos, SymbolRc>,
}

impl SignatureHelpDb {
    pub(crate) fn generate(wa: &AnalysisRef<'_>, doc: DocId) -> Self {
        let use_site_map = wa
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
        Self { use_site_map }
    }

    pub(crate) fn resolve_symbol(&self, pos: Pos) -> Option<&SymbolRc> {
        self.use_site_map.get(&pos)
    }
}

// FIXME: lsp_typesをここで使うべきではない
// (hover, completionの2か所で使われている。ここではシンボルを生成して、completion側でCompletionItemに変換するべき)
/// プリプロセッサ命令やプリプロセッサ関連のキーワードを入力補完候補として列挙する
pub(crate) fn collect_preproc_completion_items(
    wa: &AnalysisRef<'_>,
    completion_items: &mut Vec<lsp_types::CompletionItem>,
) {
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
        let sort_prefix = 'a';
        completion_items.push(lsp_types::CompletionItem {
            kind: Some(lsp_types::CompletionItemKind::KEYWORD),
            label: keyword.to_string(),
            detail: Some(detail.to_string()),
            sort_text: Some(format!("{}{}", sort_prefix, keyword)),
            ..Default::default()
        });
    }

    completion_items.extend(
        wa.hsphelp_info()
            .doc_symbols
            .iter()
            .filter(|(&doc, _)| wa.is_active_help_doc(doc))
            .flat_map(|(_, symbols)| symbols.iter().filter(|s| s.label.starts_with("#")))
            .cloned(),
    );
}

/// 指定位置のスコープに属するシンボルを列挙する (入力補完用)
pub(crate) fn collect_symbols_in_scope(
    wa: &AnalysisRef<'_>,
    doc: DocId,
    pos: Pos16,
    out_symbols: &mut Vec<SymbolRc>,
) {
    let scope = match wa.doc_analysis_map.get(&doc) {
        Some(da) => resolve_scope_at(da, pos),
        None => return,
    };

    let doc_symbols = wa
        .doc_symbols_map
        .iter()
        .filter_map(|(&d, symbols)| {
            if d == doc || wa.is_active_doc(d) {
                Some((d, symbols.as_slice()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    fn collect_local(symbols: &[SymbolRc], local: &LocalScope, out_symbols: &mut Vec<SymbolRc>) {
        for s in symbols {
            let scope = match &s.scope_opt {
                Some(it) => it,
                None => continue,
            };
            if scope.is_visible_to(local) {
                out_symbols.push(s.clone());
            }
        }
    }

    // 指定したドキュメント内のローカルシンボルを列挙する
    if let Some((_, symbols)) = doc_symbols.iter().find(|&&(d, _)| d == doc) {
        collect_local(symbols, &scope, out_symbols);
    }

    // ほかのドキュメントのローカルシンボルを列挙する
    if scope.is_outside_module() {
        for &(d, symbols) in &doc_symbols {
            if d != doc {
                collect_local(symbols, &scope, out_symbols);
            }
        }
    }

    // グローバルシンボルを列挙する
    for &(_, symbols) in &doc_symbols {
        for s in symbols {
            if let Some(Scope::Global) = s.scope_opt {
                out_symbols.push(s.clone());
            }
        }
    }
}

pub(crate) fn collect_doc_symbols(
    wa: &AnalysisRef<'_>,
    doc: DocId,
    symbols: &mut Vec<(SymbolRc, Loc)>,
) {
    let doc_symbols = match wa.doc_symbols_map.get(&doc) {
        Some(it) => it,
        None => return,
    };

    let def_site_map = wa
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

/// 指定したドキュメント内のすべてのシンボルの出現箇所 (定義・使用両方) を列挙する
/// (セマンティックトークン用)
pub(crate) fn collect_symbol_occurrences_in_doc<'a>(
    wa: &AnalysisRef<'a>,
    doc: DocId,
    symbols: &mut Vec<(&'a SymbolRc, Loc)>,
) {
    for (symbol, loc) in wa.def_sites.iter().chain(wa.use_sites) {
        if loc.doc == doc {
            symbols.push((symbol, *loc));
        }
    }
}

pub(crate) fn collect_workspace_symbols(
    wa: &AnalysisRef<'_>,
    query: &str,
    symbols: &mut Vec<(SymbolRc, Loc)>,
) {
    let name_filter = query.trim().to_ascii_lowercase();

    let map = wa
        .def_sites
        .iter()
        .filter(|(symbol, _)| symbol.name.contains(&name_filter))
        .map(|(symbol, loc)| (symbol.clone(), *loc))
        .collect::<HashMap<_, _>>();

    for (&doc, doc_symbols) in wa.doc_symbols_map.iter() {
        if !wa.active_docs.contains(&doc) {
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

/// 指定した位置に `#include` があるなら、その参照先のドキュメントを取得する
#[allow(unused)]
pub(crate) fn find_include_target(wa: &AnalysisRef<'_>, doc: DocId, pos: Pos16) -> Option<DocId> {
    // FIXME: 再実装
    // (include_resolutionが機能停止中のため無効化)
    // let (_, dest_doc) = *wa
    //     .include_resolution
    //     .iter()
    //     .find(|&(loc, _)| loc.is_touched(doc, pos))?;

    // Some(dest_doc)
    None
}
