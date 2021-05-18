use super::*;
use crate::{
    assists::{
        completion::ACompletionItem,
        signature_help::{SignatureHelpContext, SignatureHelpHost},
    },
    parse::*,
    source::range_is_touched,
};

macro_rules! or {
    ($opt:expr, $alt:expr) => {
        match $opt {
            Some(it) => it,
            None => $alt,
        }
    };
}

pub(crate) enum EntryPoints {
    Docs(Vec<DocId>),
    NonCommon,
}

impl Default for EntryPoints {
    fn default() -> Self {
        EntryPoints::Docs(vec![])
    }
}

#[derive(Default)]
pub(crate) struct ProjectAnalysis {
    pub(crate) entrypoints: EntryPoints,
}

pub(crate) enum Diagnostic {
    Undefined,
    VarRequired,
}

struct Ctx {
    use_site_map: HashMap<(DocId, Pos), ASymbol>,
    diagnostics: Vec<(Diagnostic, Loc)>,
}

impl Ctx {
    fn symbol(&self, loc: Loc) -> Option<ASymbol> {
        self.use_site_map.get(&(loc.doc, loc.start())).cloned()
    }
}

fn on_stmt(stmt: &PStmt, ctx: &mut Ctx) {
    match stmt {
        PStmt::Label(_) => {}
        PStmt::Assign(_) => {}
        PStmt::Command(stmt) => {
            let loc = stmt.command.body.loc;
            let symbol = match ctx.symbol(loc) {
                Some(it) => it,
                None => {
                    ctx.diagnostics.push((Diagnostic::Undefined, loc));
                    return;
                }
            };

            if let Some(signature_data) = symbol.signature_opt() {
                for (arg, (param, _, _)) in stmt.args.iter().zip(&signature_data.params) {
                    match param {
                        Some(PParamTy::Var) | Some(PParamTy::Modvar) | Some(PParamTy::Array) => {}
                        _ => continue,
                    }

                    let mut rval = false;
                    let mut expr_opt = arg.expr_opt.as_ref();
                    while let Some(expr) = expr_opt {
                        match expr {
                            PExpr::Compound(compound) => {
                                let name = &compound.name().body;

                                let symbol = match ctx.symbol(name.loc) {
                                    Some(it) => it,
                                    _ => break,
                                };

                                rval = match symbol.kind {
                                    ASymbolKind::Label
                                    | ASymbolKind::Const
                                    | ASymbolKind::Enum
                                    | ASymbolKind::DefFunc
                                    | ASymbolKind::DefCFunc
                                    | ASymbolKind::ModFunc
                                    | ASymbolKind::ModCFunc
                                    | ASymbolKind::ComInterface
                                    | ASymbolKind::ComFunc => true,
                                    ASymbolKind::Param(Some(param)) => match param {
                                        PParamTy::Var
                                        | PParamTy::Array
                                        | PParamTy::Modvar
                                        | PParamTy::Local => false,
                                        _ => true,
                                    },
                                    _ => false,
                                };
                                break;
                            }
                            PExpr::Paren(expr) => expr_opt = expr.body_opt.as_deref(),
                            _ => {
                                rval = true;
                                break;
                            }
                        }
                    }
                    if rval {
                        let range = match arg.expr_opt.as_ref() {
                            Some(expr) => expr.compute_range(),
                            None => stmt.command.body.loc.range,
                        };
                        let loc = loc.with_range(range);
                        ctx.diagnostics.push((Diagnostic::VarRequired, loc));
                    }
                }
            }
        }
        PStmt::DefFunc(stmt) => {
            for stmt in &stmt.stmts {
                on_stmt(stmt, ctx);
            }
        }
        PStmt::Module(stmt) => {
            for stmt in &stmt.stmts {
                on_stmt(stmt, ctx);
            }
        }
        _ => {}
    }
}

/// ワークスペースの外側のデータ
#[derive(Default)]
pub(crate) struct HostData {
    pub(crate) builtin_env: Rc<SymbolEnv>,
    pub(crate) common_docs: Rc<HashMap<String, DocId>>,
    pub(crate) entrypoints: Vec<DocId>,
}

#[derive(Default)]
pub(crate) struct AWorkspaceAnalysis {
    dirty_docs: HashSet<DocId>,
    doc_texts: HashMap<DocId, RcStr>,

    common_docs: Rc<HashMap<String, DocId>>,
    project_docs: Rc<HashMap<String, DocId>>,

    // ドキュメントごとの解析結果:
    doc_analysis_map: HashMap<DocId, DocAnalysis>,

    // すべてのドキュメントの解析結果を使って構築される情報:
    active_docs: HashSet<DocId>,
    public_env: APublicEnv,
    ns_env: HashMap<RcStr, SymbolEnv>,
    doc_symbols_map: HashMap<DocId, Vec<ASymbol>>,
    def_sites: Vec<(ASymbol, Loc)>,
    use_sites: Vec<(ASymbol, Loc)>,

    // エントリーポイントを起点として構築される情報:
    project_opt: Option<ProjectAnalysis>,
}

impl AWorkspaceAnalysis {
    pub(crate) fn initialize(&mut self, host_data: HostData) {
        let HostData {
            common_docs,
            builtin_env,
            entrypoints,
        } = host_data;

        self.common_docs = common_docs;
        self.public_env.builtin = builtin_env;

        self.project_opt = if !entrypoints.is_empty() {
            Some(ProjectAnalysis {
                entrypoints: EntryPoints::Docs(entrypoints),
            })
        } else {
            None
        };
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
        self.project_docs = project_docs;
    }

    fn compute_active_docs(
        &self,
        entrypoints: &EntryPoints,
        active_docs: &mut HashSet<DocId>,
        diagnostics: &mut Vec<(String, Loc)>,
    ) {
        match entrypoints {
            EntryPoints::Docs(entrypoints) => {
                assert_ne!(entrypoints.len(), 0);

                // エントリーポイントから推移的にincludeされるドキュメントを集める。
                let mut stack = entrypoints
                    .iter()
                    .map(|&doc| (doc, None))
                    .collect::<Vec<_>>();
                active_docs.extend(stack.iter().map(|&(doc, _)| doc));

                while let Some((doc, _)) = stack.pop() {
                    debug_assert!(active_docs.contains(&doc));
                    let da = or!(self.doc_analysis_map.get(&doc), continue);

                    for (path, loc) in &da.includes {
                        let path = path.as_str();
                        let doc_opt = self
                            .project_docs
                            .get(path)
                            .cloned()
                            .or_else(|| self.common_docs.get(path).cloned());
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
                        if active_docs.insert(d) {
                            stack.push((d, Some(loc)));
                        }
                    }
                }
            }
            EntryPoints::NonCommon => {
                // includeされていないcommonのファイルだけ除外する。

                let mut included_docs = HashSet::new();
                let in_common = self.common_docs.values().cloned().collect::<HashSet<_>>();

                for (&doc, da) in &self.doc_analysis_map {
                    if in_common.contains(&doc) {
                        continue;
                    }

                    for (include, _) in &da.includes {
                        let doc_opt = self.common_docs.get(include.as_str()).cloned();
                        included_docs.extend(doc_opt);
                    }
                }

                active_docs.extend(
                    self.doc_analysis_map
                        .keys()
                        .cloned()
                        .filter(|doc| !in_common.contains(&doc) || included_docs.contains(&doc)),
                );
            }
        }
    }

    fn compute_symbols<'a>(
        &self,
        active_docs: &HashSet<DocId>,
        public_env: &mut APublicEnv,
        ns_env: &mut HashMap<RcStr, SymbolEnv>,
        doc_symbols_map: &mut HashMap<DocId, Vec<ASymbol>>,
        def_sites: &mut Vec<(ASymbol, Loc)>,
        use_sites: &mut Vec<(ASymbol, Loc)>,
    ) {
        // 複数ファイルに渡る環境を構築する。
        for (&doc, da) in &self.doc_analysis_map {
            if !active_docs.contains(&doc) {
                continue;
            }

            extend_public_env_from_symbols(&da.preproc_symbols, public_env, ns_env);
        }

        // 変数の定義箇所を決定する。
        doc_symbols_map.extend(
            self.doc_analysis_map
                .iter()
                .filter(|(&doc, _)| active_docs.contains(&doc))
                .map(|(&doc, da)| (doc, da.preproc_symbols.clone())),
        );

        for (&doc, da) in &self.doc_analysis_map {
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
                symbols,
                public_env,
                ns_env,
                def_sites,
                use_sites,
            );
        }
    }

    fn compute(&mut self) {
        // eprintln!("compute (dirty={:?})", &self.dirty_docs);
        if self.dirty_docs.is_empty() {
            return;
        }

        for doc in self.dirty_docs.drain() {
            let text = match self.doc_texts.get(&doc) {
                Some(text) => text,
                None => continue,
            };

            let tokens = crate::token::tokenize(doc, text.clone());
            let p_tokens: RcSlice<_> = PToken::from_tokens(tokens.into()).into();
            let root = crate::parse::parse_root(p_tokens.to_owned());
            let preproc = crate::analysis::preproc::analyze_preproc(doc, &root);

            let da = self.doc_analysis_map.entry(doc).or_default();
            da.set_syntax(p_tokens, root);
            da.set_preproc(preproc);
        }

        // 以前の解析結果を捨てる:
        self.active_docs.clear();
        self.public_env.clear();
        self.ns_env.clear();
        self.def_sites.clear();
        self.use_sites.clear();

        let mut active_docs = take(&mut self.active_docs);
        let mut public_env = take(&mut self.public_env);
        let mut ns_env = take(&mut self.ns_env);
        let mut doc_symbols_map = take(&mut self.doc_symbols_map);
        let mut def_sites = take(&mut self.def_sites);
        let mut use_sites = take(&mut self.use_sites);
        let mut diagnostics = vec![];

        self.compute_active_docs(&EntryPoints::NonCommon, &mut active_docs, &mut diagnostics);

        self.compute_symbols(
            &active_docs,
            &mut public_env,
            &mut ns_env,
            &mut doc_symbols_map,
            &mut def_sites,
            &mut use_sites,
        );

        self.active_docs = active_docs;
        self.public_env = public_env;
        self.ns_env = ns_env;
        self.doc_symbols_map = doc_symbols_map;
        self.def_sites = def_sites;
        self.use_sites = use_sites;

        assert_eq!(diagnostics.len(), 0);

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
    }

    pub(crate) fn in_preproc(&mut self, doc: DocId, pos: Pos16) -> Option<bool> {
        self.compute();

        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        Some(crate::assists::completion::in_preproc(pos, tokens))
    }

    pub(crate) fn in_str_or_comment(&mut self, doc: DocId, pos: Pos16) -> Option<bool> {
        self.compute();

        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        Some(crate::assists::completion::in_str_or_comment(pos, tokens))
    }

    pub(crate) fn get_tokens(&mut self, doc: DocId) -> Option<(RcStr, RcSlice<PToken>, &PRoot)> {
        self.compute();

        let text = self.doc_texts.get(&doc)?;
        let da = self.doc_analysis_map.get(&doc)?;
        Some((text.clone(), da.tokens.clone(), da.tree_opt.as_ref()?))
    }

    pub(crate) fn get_signature_help_context(
        &mut self,
        doc: DocId,
        pos: Pos16,
    ) -> Option<SignatureHelpContext> {
        self.compute();

        let tree = &self.doc_analysis_map.get(&doc)?.tree_opt.as_ref()?;

        let use_site_map = self
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

    pub(crate) fn locate_symbol(&mut self, doc: DocId, pos: Pos16) -> Option<(ASymbol, Loc)> {
        self.compute();

        self.def_sites
            .iter()
            .chain(&self.use_sites)
            .find(|&(_, loc)| loc.is_touched(doc, pos))
            .cloned()
    }

    pub(crate) fn get_ident_at(&mut self, doc: DocId, pos: Pos16) -> Option<(RcStr, Loc)> {
        self.compute();

        let tokens = &self.doc_analysis_map.get(&doc)?.tokens;
        let token = match tokens.binary_search_by_key(&pos, |t| t.body.loc.start().into()) {
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

    #[allow(unused)]
    pub(crate) fn symbol_name(&self, symbol: ASymbol) -> Option<RcStr> {
        let doc_analysis = self.doc_analysis_map.get(&symbol.doc)?;
        Some(symbol.name())
    }

    pub(crate) fn get_symbol_details(
        &self,
        symbol: &ASymbol,
    ) -> Option<(RcStr, &'static str, ASymbolDetails)> {
        Some((
            symbol.name(),
            symbol.kind.as_str(),
            symbol.compute_details(),
        ))
    }

    pub(crate) fn diagnose(&mut self, diagnostics: &mut Vec<(String, Loc)>) {
        self.compute();

        self.diagnose_precisely(diagnostics);
    }

    pub(crate) fn collect_symbol_defs(&mut self, symbol: &ASymbol, locs: &mut Vec<Loc>) {
        self.compute();

        for &(ref s, loc) in &self.def_sites {
            if s == symbol {
                locs.push(loc);
            }
        }
    }

    pub(crate) fn collect_symbol_uses(&mut self, symbol: &ASymbol, locs: &mut Vec<Loc>) {
        self.compute();

        for &(ref s, loc) in &self.use_sites {
            if s == symbol {
                locs.push(loc);
            }
        }
    }

    pub(crate) fn collect_completion_items(
        &mut self,
        doc: DocId,
        pos: Pos16,
        completion_items: &mut Vec<ACompletionItem>,
    ) {
        self.compute();

        let scope = match self.doc_analysis_map.get(&doc) {
            Some(da) => resolve_scope_at(&da.modules, &da.deffuncs, pos),
            None => ALocalScope::default(),
        };

        let doc_symbols = self
            .doc_symbols_map
            .iter()
            .filter_map(|(&d, symbols)| {
                if d == doc || self.active_docs.contains(&d) {
                    Some((d, symbols.as_slice()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        crate::assists::completion::collect_symbols_as_completion_items(
            doc,
            scope,
            &doc_symbols,
            completion_items,
        );
    }

    pub(crate) fn collect_all_symbols(
        &mut self,
        name_filter: &str,
        symbols: &mut Vec<(ASymbol, Loc)>,
    ) {
        self.compute();

        let name_filter = name_filter.trim().to_ascii_lowercase();

        let map = self
            .def_sites
            .iter()
            .filter(|(symbol, _)| symbol.name.contains(&name_filter))
            .map(|(symbol, loc)| (symbol.clone(), *loc))
            .collect::<HashMap<_, _>>();

        for (&doc, doc_symbols) in &self.doc_symbols_map {
            if !self.active_docs.contains(&doc) {
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

    pub(crate) fn collect_doc_symbols(&mut self, doc: DocId, symbols: &mut Vec<(ASymbol, Loc)>) {
        self.compute();

        let doc_symbols = match self.doc_symbols_map.get(&doc) {
            Some(it) => it,
            None => return,
        };

        let def_site_map = self
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

    pub(crate) fn diagnose_syntax_lints(&mut self, lints: &mut Vec<(SyntaxLint, Loc)>) {
        self.compute();

        for (&doc, da) in &mut self.doc_analysis_map {
            if !self.active_docs.contains(&doc) {
                continue;
            }

            if !da.syntax_lint_done {
                debug_assert_eq!(da.syntax_lints.len(), 0);
                let tree = or!(da.tree_opt.as_ref(), continue);
                crate::analysis::syntax_linter::syntax_lint(&tree, &mut da.syntax_lints);
                da.syntax_lint_done = true;
            }

            lints.extend(da.syntax_lints.iter().cloned());
        }
    }

    pub(crate) fn diagnose_precisely(&mut self, diagnostics: &mut Vec<(String, Loc)>) {
        self.compute();

        let p = match &self.project_opt {
            Some(it) => it,
            None => return,
        };

        let mut active_docs = HashSet::new();
        let mut public_env = APublicEnv::default();
        let mut ns_env = HashMap::new();
        let mut def_sites = vec![];
        let mut use_sites = vec![];
        let mut doc_symbols_map = HashMap::new();

        self.compute_active_docs(&p.entrypoints, &mut active_docs, diagnostics);

        self.compute_symbols(
            &active_docs,
            &mut public_env,
            &mut ns_env,
            &mut doc_symbols_map,
            &mut def_sites,
            &mut use_sites,
        );

        // diagnose:

        let use_site_map = use_sites
            .iter()
            .map(|(symbol, loc)| ((loc.doc, loc.start()), symbol.clone()))
            .collect::<HashMap<_, _>>();

        let mut ctx = Ctx {
            use_site_map,
            diagnostics: vec![],
        };

        for (&doc, da) in &self.doc_analysis_map {
            if !active_docs.contains(&doc) {
                continue;
            }

            let root = or!(da.tree_opt.as_ref(), continue);

            for stmt in &root.stmts {
                on_stmt(stmt, &mut ctx);
            }
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

fn resolve_scope_at(
    modules: &HashMap<AModule, AModuleData>,
    deffuncs: &HashMap<ADefFunc, ADefFuncData>,
    pos: Pos16,
) -> ALocalScope {
    let mut scope = ALocalScope::default();

    scope.module_opt = modules.iter().find_map(|(m, module_data)| {
        if range_is_touched(&module_data.content_loc.range, pos) {
            Some(m.clone())
        } else {
            None
        }
    });

    scope.deffunc_opt = deffuncs.iter().find_map(|(&d, deffunc_data)| {
        if range_is_touched(&deffunc_data.content_loc.range, pos) {
            Some(d)
        } else {
            None
        }
    });

    scope
}

#[cfg(test)]
mod tests {
    use super::AWorkspaceAnalysis;
    use super::*;
    use crate::source::{DocId, Pos};

    /// `<|x|>` のようなマーカーを含む文字列を受け取る。間に挟まれている x の部分をマーカーの名前と呼ぶ。
    /// マーカーを取り除いた文字列 text と、text の中でマーカーが指している位置のリストを返す。
    fn parse_cursor_string(s: &str) -> (String, Vec<(&str, Pos)>) {
        let mut output = vec![];

        let mut text = String::with_capacity(s.len());
        let mut pos = Pos::default();
        let mut i = 0;

        while let Some(offset) = s[i..].find("<|") {
            // カーソルを <| の手前まで進める。
            let j = i + offset;
            text += &s[i..j];
            pos += Pos::from(&s[i..j]);
            i += offset + "<|".len();

            // <| と |> の間を名前として取る。
            let name_len = s[i..].find("|>").expect("missing |>");
            let j = i + name_len;
            let name = &s[i..j];
            i += name_len + "|>".len();

            output.push((name, pos));
        }

        text += &s[i..];
        (text, output)
    }

    #[test]
    fn test_locate_static_var_def() {
        let mut wa = AWorkspaceAnalysis::default();

        let doc: DocId = 1;
        let text = r#"
            <|A|>foo = 1
        "#;
        let expected_map = vec![("A", Some("foo"))]
            .into_iter()
            .collect::<HashMap<_, _>>();
        let (text, cursors) = parse_cursor_string(text);

        wa.update_doc(doc, text.into());

        for (name, pos) in cursors {
            let actual = wa
                .locate_symbol(doc, pos.into())
                .and_then(|(symbol, _)| wa.symbol_name(symbol));
            assert_eq!(actual.as_deref(), expected_map[name], "name={}", name);
        }
    }

    #[test]
    fn test_it_works() {
        let mut wa = AWorkspaceAnalysis::default();

        let doc: DocId = 1;
        let text = r#"
            #module
            #deffunc <|A|>hello
                mes "Hello, world!"
                return
            #global

                <|B|>hello
                hello<|C|> <|D|>
        "#;
        let expected_map = vec![
            ("A", Some("hello")),
            ("B", Some("hello")),
            ("C", Some("hello")),
            ("D", None),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let (text, cursors) = parse_cursor_string(text);

        wa.update_doc(doc, text.into());

        for (name, pos) in cursors {
            let actual = wa
                .locate_symbol(doc, pos.into())
                .and_then(|(symbol, _)| wa.symbol_name(symbol));
            assert_eq!(actual.as_deref(), expected_map[name], "name={}", name);
        }
    }
}
