use super::{
    a_scope::{ADefFunc, AModule, AModuleData, DefineLevel},
    a_symbol::{ASymbolData, AWsSymbol},
    comment::{calculate_details, collect_comments},
    preproc::PreprocAnalysisResult,
    var::{AAnalysis, APublicState},
    AScope, ASymbol, ASymbolDetails,
};
use crate::{
    analysis::a_scope::ALocalScope,
    parse::*,
    source::{range_is_touched, DocId, Loc, Pos16},
    token::TokenKind,
    utils::{rc_slice::RcSlice, rc_str::RcStr},
};
use std::{
    collections::{HashMap, HashSet},
    mem::take,
};

#[derive(Default)]
pub(crate) struct AWorkspaceAnalysis {
    pub(crate) dirty_docs: HashSet<DocId>,
    pub(crate) doc_texts: HashMap<DocId, RcStr>,

    // ドキュメントごとの解析結果:
    pub(crate) doc_syntax_map: HashMap<DocId, ASyntax>,
    pub(crate) doc_preproc_map: HashMap<DocId, PreprocAnalysisResult>,
    pub(crate) doc_analysis_map: HashMap<DocId, AAnalysis>,

    // すべてのドキュメントの解析結果を使って構築される情報:
    pub(crate) public_env: APublicEnv,
    pub(crate) ns_env: HashMap<RcStr, AEnv>,
    pub(crate) def_sites: Vec<(AWsSymbol, Loc)>,
    pub(crate) use_sites: Vec<(AWsSymbol, Loc)>,
}

impl AWorkspaceAnalysis {
    fn invalidate(&mut self, doc: DocId) {
        self.doc_syntax_map.remove(&doc);
        self.doc_preproc_map.remove(&doc);
        self.doc_analysis_map.remove(&doc);
    }

    pub(crate) fn update_doc(&mut self, doc: DocId, text: RcStr) {
        self.dirty_docs.insert(doc);
        self.doc_texts.insert(doc, text);
        self.invalidate(doc);
    }

    pub(crate) fn close_doc(&mut self, doc: DocId) {
        self.dirty_docs.insert(doc);
        self.doc_texts.remove(&doc);
        self.invalidate(doc);
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

            let (syntax, preproc) = {
                let tokens = crate::token::tokenize(doc, text.clone());
                let p_tokens: RcSlice<_> = PToken::from_tokens(tokens.into()).into();
                let root = crate::parse::parse_root(p_tokens.to_owned());
                let preproc = crate::analysis::preproc::analyze_preproc(doc, &root);

                let syntax = ASyntax {
                    tokens: p_tokens,
                    tree: root,
                };
                (syntax, preproc)
            };

            self.doc_syntax_map.insert(doc, syntax);
            self.doc_preproc_map.insert(doc, preproc);
        }

        self.public_env.clear();
        self.ns_env.clear();
        self.def_sites.clear();
        self.use_sites.clear();

        // 複数ファイルに渡る環境を構築する。
        for (&doc, preproc) in &self.doc_preproc_map {
            for (i, symbol_data) in preproc.symbols.iter().enumerate() {
                let symbol = ASymbol::new(i);
                let ws_symbol = AWsSymbol { doc, symbol };

                match &symbol_data.scope_opt {
                    Some(AScope::Global(_)) => {
                        self.public_env
                            .global
                            .insert(symbol_data.name.clone(), ws_symbol);
                    }
                    _ => {}
                }

                if let Some(ns) = &symbol_data.ns_opt {
                    self.ns_env
                        .entry(ns.clone())
                        .or_default()
                        .insert(symbol_data.name.clone(), ws_symbol);
                }
            }
        }

        // 変数の定義箇所を決定する。
        let mut public_state = APublicState {
            env: take(&mut self.public_env),
            ns_env: take(&mut self.ns_env),
            def_sites: take(&mut self.def_sites),
            use_sites: take(&mut self.use_sites),
        };

        for (&doc, syntax) in &self.doc_syntax_map {
            let symbols = self.doc_preproc_map[&doc].symbols.clone();
            let analysis = crate::analysis::var::analyze_var_def(
                doc,
                &syntax.tree,
                symbols,
                &mut public_state,
            );
            self.doc_analysis_map.insert(doc, analysis);
        }

        {
            let APublicState {
                env,
                ns_env,
                def_sites,
                use_sites,
            } = public_state;
            self.public_env = env;
            self.ns_env = ns_env;
            self.def_sites = def_sites;
            self.use_sites = use_sites;
        }

        // シンボルの定義・使用箇所を収集する。
        for (&doc, analysis) in &mut self.doc_analysis_map {
            for (i, symbol_data) in analysis.symbols.iter().enumerate() {
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

        let syntax = self.doc_syntax_map.get(&doc)?;
        let tokens = &syntax.tokens;
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

    pub(crate) fn diagnose(&mut self) {
        self.compute();
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

    pub(crate) fn collect_completion_items(
        &mut self,
        doc: DocId,
        pos: Pos16,
    ) -> Vec<ACompletionItem> {
        self.compute();

        let mut completion_items = vec![];

        let mut scope = ALocalScope::default();

        if let Some(doc_analysis) = self.doc_analysis_map.get(&doc) {
            let syntax = &self.doc_syntax_map[&doc];
            let preproc = &self.doc_preproc_map[&doc];
            scope = resolve_scope_at(&syntax.tree, &preproc.modules, pos);
            collect_local_completion_items(&doc_analysis.symbols, &scope, &mut completion_items);
        }

        if scope.is_outside_module() {
            for (&d, doc_analysis) in &self.doc_analysis_map {
                if d != doc {
                    collect_local_completion_items(
                        &doc_analysis.symbols,
                        &scope,
                        &mut completion_items,
                    );
                }
            }
        }

        for doc_analysis in self.doc_analysis_map.values() {
            collect_global_completion_items(&doc_analysis.symbols, &mut completion_items);
        }

        completion_items
    }
}

pub(crate) struct ASyntax {
    pub(crate) tokens: RcSlice<PToken>,
    pub(crate) tree: PRoot,
}

/// 環境。名前からシンボルへのマップ。
#[derive(Debug, Default)]
pub(crate) struct AEnv {
    map: HashMap<RcStr, AWsSymbol>,
    stack: Vec<(DefineLevel, RcStr, AWsSymbol)>,
}

impl AEnv {
    pub(crate) fn get(&self, name: &str) -> Option<AWsSymbol> {
        self.map.get(name).cloned()
    }

    pub(crate) fn insert(&mut self, name: RcStr, symbol: AWsSymbol) {
        self.map.insert(name, symbol);
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }
}

#[derive(Default)]
pub(crate) struct APublicEnv {
    /// 標準命令などのシンボルが属す環境。(この環境はソースファイルの変更時に無効化しないので、globalと分けている。)
    pub(crate) builtin: AEnv,

    /// あらゆる場所で使えるシンボルが属す環境。(標準命令や `#define global` で定義されたマクロなど)
    pub(crate) global: AEnv,
}

impl APublicEnv {
    pub(crate) fn resolve(&self, name: &str) -> Option<AWsSymbol> {
        self.global.get(name).or_else(|| self.builtin.get(name))
    }

    pub(crate) fn clear(&mut self) {
        self.global.clear();
    }
}

fn resolve_scope_at(
    root: &PRoot,
    modules: &HashMap<AModule, AModuleData>,
    pos: Pos16,
) -> ALocalScope {
    let mut di = 0;
    let mut scope = ALocalScope::default();

    scope.module_opt = modules.iter().find_map(|(m, module_data)| {
        if range_is_touched(&module_data.content_loc.range, pos) {
            Some(m.clone())
        } else {
            None
        }
    });

    for stmt in &root.stmts {
        match stmt {
            PStmt::DefFunc(stmt) => {
                di += 1;

                let content_loc = stmt.hash.body.loc.unite(&stmt.behind);
                if range_is_touched(&content_loc.range, pos) {
                    scope.deffunc_opt = Some(ADefFunc::new(di));
                }
            }
            _ => {}
        }
    }

    scope
}

pub(crate) enum ACompletionItem<'a> {
    Symbol(&'a ASymbolData),
}

fn collect_local_completion_items<'a>(
    symbols: &'a [ASymbolData],
    local: &ALocalScope,
    completion_items: &mut Vec<ACompletionItem<'a>>,
) {
    for s in symbols {
        match &s.scope_opt {
            Some(AScope::Local(scope)) if scope.is_visible_to(local) => {
                completion_items.push(ACompletionItem::Symbol(s));
            }
            _ => continue,
        }
    }
}

fn collect_global_completion_items<'a>(
    symbols: &'a [ASymbolData],
    completion_items: &mut Vec<ACompletionItem<'a>>,
) {
    for s in symbols {
        if let Some(AScope::Global(_)) = s.scope_opt {
            completion_items.push(ACompletionItem::Symbol(s));
        }
    }
}

/// 名前の修飾子。
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Qual {
    /// 非修飾。`xxx`
    Unqualified,

    /// トップレベルの名前空間の修飾付き。`xxx@`
    Toplevel,

    /// モジュールの名前空間の修飾付き。`xxx@m_hoge`
    Module(RcStr),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Name {
    pub(crate) base: RcStr,
    pub(crate) qual: Qual,
}

impl Name {
    pub(crate) fn new(name: &RcStr) -> Self {
        match name.rfind('@') {
            Some(i) if i + 1 == name.len() => Name {
                base: name.slice(0, i),
                qual: Qual::Toplevel,
            },
            Some(i) => Name {
                base: name.slice(0, i),
                qual: Qual::Module(name.slice(i + 1, name.len())),
            },
            None => Name {
                base: name.clone(),
                qual: Qual::Unqualified,
            },
        }
    }
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
