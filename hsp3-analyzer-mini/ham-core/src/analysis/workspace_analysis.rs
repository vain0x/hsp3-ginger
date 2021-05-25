use super::*;

type DocAnalysisMap = HashMap<DocId, DocAnalysis>;

/// ワークスペースの外側のデータ
#[derive(Default)]
pub(crate) struct WorkspaceHost {
    pub(crate) builtin_env: Rc<SymbolEnv>,
    pub(crate) common_docs: Rc<HashMap<String, DocId>>,
    pub(crate) entrypoints: Vec<DocId>,
}

#[derive(Default)]
pub(crate) struct WorkspaceAnalysis {
    dirty_docs: HashSet<DocId>,
    doc_texts: HashMap<DocId, RcStr>,

    // すべてのドキュメントの解析結果を使って構築される情報:
    doc_analysis_map: DocAnalysisMap,
    module_map: ModuleMap,
    project1: ProjectAnalysis,
    project_opt: Option<ProjectAnalysis>,
}

impl WorkspaceAnalysis {
    pub(crate) fn initialize(&mut self, host: WorkspaceHost) {
        let WorkspaceHost {
            common_docs,
            builtin_env,
            entrypoints,
        } = host;

        self.project_opt = if !entrypoints.is_empty() {
            let mut p = ProjectAnalysis::default();
            p.entrypoints = EntryPoints::Docs(entrypoints);
            p.common_docs = common_docs.clone();
            p.public_env.builtin = builtin_env.clone();
            Some(p)
        } else {
            None
        };

        self.project1.entrypoints = EntryPoints::NonCommon;
        self.project1.common_docs = common_docs;
        self.project1.public_env.builtin = builtin_env;
    }

    pub(crate) fn update_doc(&mut self, doc: DocId, text: RcStr) {
        self.dirty_docs.insert(doc);
        self.doc_texts.insert(doc, text);
        self.doc_analysis_map
            .entry(doc)
            .and_modify(|a| a.invalidate());
    }

    pub(crate) fn close_doc(&mut self, doc: DocId) {
        self.dirty_docs.insert(doc);
        self.doc_texts.remove(&doc);
        self.doc_analysis_map.remove(&doc);
    }

    pub(crate) fn set_project_docs(&mut self, project_docs: Rc<HashMap<String, DocId>>) {
        for p in [Some(&mut self.project1), self.project_opt.as_mut()]
            .iter_mut()
            .flatten()
        {
            p.project_docs = project_docs.clone();
        }
    }

    fn compute(&mut self) {
        if self.dirty_docs.is_empty() {
            return;
        }

        self.project1.invalidate();
        if let Some(p) = self.project_opt.as_mut() {
            p.invalidate();
        }

        let mut doc_analysis_map = take(&mut self.doc_analysis_map);
        self.module_map.clear();

        for doc in self.dirty_docs.drain() {
            let text = match self.doc_texts.get(&doc) {
                Some(text) => text,
                None => continue,
            };

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

        // 以前の解析結果を捨てる:
        for p in [Some(&mut self.project1), self.project_opt.as_mut()]
            .iter_mut()
            .flatten()
        {
            p.compute(&self.doc_analysis_map, &self.module_map);
        }

        assert_eq!(self.project1.diagnostics.len(), 0);
    }

    pub(crate) fn in_preproc(&mut self, doc: DocId, pos: Pos16) -> Option<bool> {
        self.compute();

        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        Some(in_preproc(pos, tokens))
    }

    pub(crate) fn in_str_or_comment(&mut self, doc: DocId, pos: Pos16) -> Option<bool> {
        self.compute();

        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        Some(in_str_or_comment(pos, tokens))
    }

    pub(crate) fn get_tokens(&mut self, doc: DocId) -> Option<(RcStr, RcSlice<PToken>, &PRoot)> {
        self.compute();

        let text = self.doc_texts.get(&doc)?;
        let da = self.doc_analysis_map.get(&doc)?;
        Some((text.clone(), da.tokens.clone(), da.tree_opt.as_ref()?))
    }

    pub(crate) fn get_ident_at(&mut self, doc: DocId, pos: Pos16) -> Option<(RcStr, Loc)> {
        self.compute();

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

    pub(crate) fn require_some_project(&mut self) -> ProjectAnalysisRef {
        self.compute();

        let p = self.project_opt.as_mut().unwrap_or(&mut self.project1);
        p.compute(&self.doc_analysis_map, &self.module_map)
    }

    pub(crate) fn require_project_for_doc(&mut self, doc: DocId) -> ProjectAnalysisRef {
        self.compute();

        if let Some(p) = self.project_opt.as_mut() {
            if p.active_docs.contains(&doc) {
                debug_assert!(p.is_computed());
                return p.compute(&self.doc_analysis_map, &self.module_map);
            }
        }

        debug_assert!(self.project1.is_computed());
        self.project1
            .compute(&self.doc_analysis_map, &self.module_map)
    }

    pub(crate) fn diagnose(&mut self, diagnostics: &mut Vec<(String, Loc)>) {
        self.compute();

        self.diagnose_precisely(diagnostics);
    }

    pub(crate) fn diagnose_syntax_lints(&mut self, lints: &mut Vec<(SyntaxLint, Loc)>) {
        self.compute();

        let p = match self.project_opt.as_ref() {
            Some(it) => it,
            None => return,
        };

        for (&doc, da) in self.doc_analysis_map.iter() {
            if !p.active_docs.contains(&doc) {
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

    pub(crate) fn diagnose_precisely(&mut self, diagnostics: &mut Vec<(String, Loc)>) {
        self.compute();

        let p = match &self.project_opt {
            Some(it) => it,
            None => return,
        };

        // diagnose:

        let use_site_map = p
            .use_sites
            .iter()
            .map(|(symbol, loc)| ((loc.doc, loc.start()), symbol.clone()))
            .collect::<HashMap<_, _>>();

        let mut ctx = Sema {
            use_site_map,
            diagnostics: vec![],
        };

        for (&doc, da) in self.doc_analysis_map.iter() {
            if !p.active_docs.contains(&doc) {
                continue;
            }

            let root = match &da.tree_opt {
                Some(it) => it,
                None => continue,
            };

            ctx.on_root(root);
        }

        // どのプロジェクトに由来するか覚えておく必要がある
        diagnostics.extend(ctx.diagnostics.into_iter().map(|(d, loc)| {
            let msg = match d {
                Diagnostic::Undefined => "定義が見つかりません",
                Diagnostic::VarRequired => "変数か配列の要素が必要です。",
            }
            .to_string();
            (msg, loc)
        }));
    }
}
