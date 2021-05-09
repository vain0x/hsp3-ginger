use super::{
    a_scope::*,
    a_symbol::*,
    comment::{calculate_details, collect_comments},
    name_system::*,
    preproc::ASignatureData,
    syntax_linter::SyntaxLint,
    ASymbol, ASymbolDetails,
};
use crate::{
    analysis::*,
    assists::{
        completion::ACompletionItem,
        signature_help::{SignatureHelpContext, SignatureHelpHost},
    },
    parse::*,
    source::{range_is_touched, DocId, Loc, Pos, Pos16},
    token::TokenKind,
    utils::{rc_slice::RcSlice, rc_str::RcStr},
};
use std::{
    collections::{HashMap, HashSet},
    mem::take,
    rc::Rc,
};

macro_rules! or {
    ($opt:expr, $alt:expr) => {
        match $opt {
            Some(it) => it,
            None => $alt,
        }
    };
}

#[derive(Default)]
pub(crate) struct ProjectAnalysis {
    pub(crate) entrypoints: Vec<DocId>,
}

pub(crate) enum Diagnostic {
    Undefined,
}

struct Ctx {
    use_site_map: HashMap<(DocId, Pos), AWsSymbol>,
    diagnostics: Vec<(Diagnostic, Loc)>,
}

fn on_stmt(stmt: &PStmt, ctx: &mut Ctx) {
    match stmt {
        PStmt::Label(_) => {}
        PStmt::Assign(_) => {}
        PStmt::Command(stmt) => {
            let loc = stmt.command.body.loc;
            let ws_symbol_opt = ctx.use_site_map.get(&(loc.doc, loc.start()));
            if ws_symbol_opt.is_none() {
                ctx.diagnostics.push((Diagnostic::Undefined, loc));
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
    pub(crate) builtin_env: SymbolEnv,
    pub(crate) builtin_signatures: HashMap<AWsSymbol, Rc<ASignatureData>>,
    pub(crate) common_docs: HashMap<String, DocId>,
}

#[derive(Default)]
pub(crate) struct AWorkspaceAnalysis {
    dirty_docs: HashSet<DocId>,
    doc_texts: HashMap<DocId, RcStr>,

    builtin_signatures: HashMap<AWsSymbol, Rc<ASignatureData>>,
    common_docs: HashMap<String, DocId>,
    project_docs: HashMap<String, DocId>,

    // ドキュメントごとの解析結果:
    doc_analysis_map: HashMap<DocId, DocAnalysis>,

    // すべてのドキュメントの解析結果を使って構築される情報:
    active_docs: HashSet<DocId>,
    public_env: APublicEnv,
    ns_env: HashMap<RcStr, SymbolEnv>,
    def_sites: Vec<(AWsSymbol, Loc)>,
    use_sites: Vec<(AWsSymbol, Loc)>,

    // エントリーポイントを起点として構築される情報:
    projects: Vec<ProjectAnalysis>,
}

impl AWorkspaceAnalysis {
    pub(crate) fn initialize(&mut self, host_data: HostData) {
        let HostData {
            builtin_signatures,
            common_docs,
            builtin_env,
        } = host_data;

        self.builtin_signatures = builtin_signatures;
        self.common_docs = common_docs;
        self.public_env.builtin = builtin_env;
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

        for da in self.doc_analysis_map.values_mut() {
            da.rollback_to_preproc();
        }

        debug_assert!(self
            .doc_analysis_map
            .values()
            .all(|da| da.after_preproc() && !da.after_symbols()));

        // エントリーポイントを決定する。
        // 仮実装として、コメントかどこかに "ginger=entrypoint" と書いてあるファイルはエントリーポイントとみなす。実際には、どのファイルがエントリーポイントかを指定する設定ファイル(Cargo.tomlのようなもの)を置いてもらう。
        {
            let mut p = self.projects.drain(..).next().unwrap_or_default();
            p.entrypoints.clear();
            p.entrypoints.extend(
                self.doc_texts
                    .iter()
                    .filter(|(_, text)| text.contains("ginger=entrypoint"))
                    .map(|(&doc, _)| doc),
            );
            if !p.entrypoints.is_empty() {
                self.projects.push(p);
            }
        }

        // 有効なドキュメントを検出する。(includeされていないcommonのファイルは無視する。)
        let mut included_docs = HashSet::new();
        let in_common = self.common_docs.values().cloned().collect::<HashSet<_>>();

        for (&doc, da) in &self.doc_analysis_map {
            if in_common.contains(&doc) {
                continue;
            }

            for include in &da.includes {
                let doc_opt = self.common_docs.get(include.as_str()).cloned();
                included_docs.extend(doc_opt);
            }
        }

        self.active_docs.extend(
            self.doc_analysis_map
                .keys()
                .cloned()
                .filter(|doc| !in_common.contains(&doc) || included_docs.contains(&doc)),
        );

        // 複数ファイルに渡る環境を構築する。
        for (&doc, da) in &self.doc_analysis_map {
            if !self.active_docs.contains(&doc) {
                continue;
            }

            extend_public_env_from_symbols(
                doc,
                &da.symbols,
                &mut self.public_env,
                &mut self.ns_env,
            );
        }

        // 変数の定義箇所を決定する。
        for (&doc, da) in &mut self.doc_analysis_map {
            if !self.active_docs.contains(&doc) {
                continue;
            }

            crate::analysis::var::analyze_var_def(
                doc,
                da.tree_opt.as_ref().unwrap(),
                &mut da.symbols,
                &mut self.public_env,
                &mut self.ns_env,
                &mut self.def_sites,
                &mut self.use_sites,
            );
            da.symbols_updated();
        }

        // シンボルの定義・使用箇所を収集する。
        for (&doc, da) in &mut self.doc_analysis_map {
            if !self.active_docs.contains(&doc) {
                continue;
            }

            debug_assert!(da.after_symbols());
            for (i, symbol_data) in da.symbols.iter().enumerate() {
                let symbol = ASymbol::new(i);

                self.def_sites.extend(
                    symbol_data
                        .def_sites
                        .iter()
                        .map(|&loc| (AWsSymbol { doc, symbol }, loc)),
                );

                self.use_sites.extend(
                    symbol_data
                        .use_sites
                        .iter()
                        .map(|&loc| (AWsSymbol { doc, symbol }, loc)),
                );
            }
        }

        // eprintln!("global_env={:#?}", &self.global_env);
        // eprintln!("analysis_map={:#?}", &self.doc_analysis_map);
        // eprintln!("def_sites={:#?}", &self.def_sites);
        // eprintln!("use_sites={:#?}", &self.use_sites);
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

        let symbol_signatures = self
            .doc_analysis_map
            .iter()
            .flat_map(|(&doc, da)| {
                da.symbols
                    .iter()
                    .enumerate()
                    .filter_map(move |(i, symbol_data)| {
                        let symbol = ASymbol::new(i);
                        let ws_symbol = AWsSymbol { doc, symbol };
                        let signature = Rc::clone(symbol_data.signature_opt.as_ref()?);
                        Some((ws_symbol, signature))
                    })
            })
            .collect::<HashMap<_, _>>();

        let tree = &self.doc_analysis_map.get(&doc)?.tree_opt.as_ref()?;

        let use_site_map = self
            .use_sites
            .iter()
            .filter_map(|&(ws_symbol, loc)| {
                if loc.doc == doc {
                    Some((loc.start(), ws_symbol))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let mut h = SignatureHelpHost {
            builtin_signatures: take(&mut self.builtin_signatures),
            symbol_signatures,
            use_site_map,
        };
        let out = h.process(pos, tree);
        self.builtin_signatures = h.builtin_signatures;

        out
    }

    pub(crate) fn locate_symbol(&mut self, doc: DocId, pos: Pos16) -> Option<(AWsSymbol, Loc)> {
        self.compute();

        // eprintln!("symbol_uses={:?}", &self.use_sites);

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
    pub(crate) fn symbol_name(&self, wa_symbol: AWsSymbol) -> Option<&str> {
        let doc_analysis = self.doc_analysis_map.get(&wa_symbol.doc)?;
        let symbol_data = &doc_analysis.symbols[wa_symbol.symbol.get()];
        Some(&symbol_data.name)
    }

    pub(crate) fn get_symbol_details(
        &self,
        wa_symbol: AWsSymbol,
    ) -> Option<(RcStr, &'static str, ASymbolDetails)> {
        let doc_analysis = self.doc_analysis_map.get(&wa_symbol.doc)?;
        let symbol_data = &doc_analysis.symbols[wa_symbol.symbol.get()];
        Some((
            symbol_data.name.clone(),
            symbol_data.kind.as_str(),
            calculate_details(&collect_comments(&symbol_data.leader)),
        ))
    }

    pub(crate) fn diagnose(&mut self, diagnostics: &mut Vec<(String, Loc)>) {
        self.compute();

        self.diagnose_precisely(diagnostics);
    }

    pub(crate) fn collect_symbol_defs(&mut self, ws_symbol: AWsSymbol, locs: &mut Vec<Loc>) {
        self.compute();

        for &(s, loc) in &self.def_sites {
            if s == ws_symbol {
                locs.push(loc);
            }
        }
    }

    pub(crate) fn collect_symbol_uses(&mut self, ws_symbol: AWsSymbol, locs: &mut Vec<Loc>) {
        self.compute();

        for &(s, loc) in &self.use_sites {
            if s == ws_symbol {
                locs.push(loc);
            }
        }
    }

    pub(crate) fn collect_completion_items<'a>(
        &'a mut self,
        doc: DocId,
        pos: Pos16,
        completion_items: &mut Vec<ACompletionItem<'a>>,
    ) {
        self.compute();

        let scope = match self.doc_analysis_map.get(&doc) {
            Some(da) => resolve_scope_at(&da.modules, &da.deffuncs, pos),
            None => ALocalScope::default(),
        };

        let doc_symbols = self
            .doc_analysis_map
            .iter()
            .filter_map(|(&d, da)| {
                if d == doc || self.active_docs.contains(&d) {
                    Some((d, da.symbols.as_slice()))
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

    pub(crate) fn diagnose_syntax_lints(&mut self, lints: &mut Vec<(SyntaxLint, Loc)>) {
        self.compute();

        for (&doc, da) in &self.doc_analysis_map {
            if !self.active_docs.contains(&doc) {
                continue;
            }

            let tree = or!(da.tree_opt.as_ref(), continue);
            lints.extend(crate::analysis::syntax_linter::syntax_lint(tree));
        }
    }

    pub(crate) fn diagnose_precisely(&mut self, diagnostics: &mut Vec<(String, Loc)>) {
        self.compute();

        let mut active_docs = HashSet::new();

        for p in &self.projects {
            let mut q = p.entrypoints.clone();

            active_docs.clear();
            active_docs.extend(q.iter().cloned());

            while let Some(doc) = q.pop() {
                let da = or!(self.doc_analysis_map.get(&doc), continue);
                for s in &da.includes {
                    let d = *or!(self.project_docs.get(s.as_str()), continue);
                    if active_docs.insert(d) {
                        q.push(d);
                    }
                }
            }

            // signature help のときも作った
            let use_site_map = self
                .use_sites
                .iter()
                .filter_map(|&(ws_symbol, loc)| {
                    if active_docs.contains(&loc.doc) {
                        let AWsSymbol { doc, symbol } = ws_symbol;
                        if active_docs.contains(&doc)
                            && self
                                .doc_analysis_map
                                .get(&doc)
                                .and_then(|d| d.symbols.get(symbol.get()))
                                .map_or(false, |s| s.kind == ASymbolKind::DefFunc)
                        {
                            Some(((loc.doc, loc.start()), ws_symbol))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
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
                }
                .to_string();
                (msg, loc)
            }));
        }
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
    use crate::source::{DocId, Pos};
    use std::collections::HashMap;

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
            assert_eq!(actual, expected_map[name], "name={}", name);
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
            assert_eq!(actual, expected_map[name], "name={}", name);
        }
    }
}
