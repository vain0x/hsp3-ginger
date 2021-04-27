use super::{
    a_symbol::{ASymbolData, AWsSymbol},
    analyze::{AAnalysis, ACompletionItem},
    comment::calculate_details,
    ADoc, ALoc, APos, ASymbolDetails,
};
use crate::{
    analysis::a_scope::ALocalScope,
    parse::{PRoot, PToken},
    token::{TokenData, TokenKind},
    utils::{rc_slice::RcSlice, rc_str::RcStr},
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct AWorkspaceAnalysis {
    pub(crate) dirty_docs: HashSet<ADoc>,
    pub(crate) doc_texts: HashMap<ADoc, RcStr>,
    pub(crate) doc_syntax_map: HashMap<ADoc, ASyntax>,

    /// ドキュメントごとの解析結果。
    pub(crate) doc_analysis_map: HashMap<ADoc, AAnalysis>,

    // すべてのドキュメントの解析結果を使って構築される情報:
    /// `global` で定義されたシンボルの名前を解決するためのマップ。
    pub(crate) global_env: HashMap<RcStr, AWsSymbol>,
    /// `global` を除く、モジュールの外でのみ使えるシンボルの名前を解決するためのマップ。
    pub(crate) toplevel_env: HashMap<RcStr, AWsSymbol>,

    pub(crate) def_sites: Vec<(AWsSymbol, ALoc)>,
    pub(crate) use_sites: Vec<(AWsSymbol, ALoc)>,
}

impl AWorkspaceAnalysis {
    pub(crate) fn update_doc(&mut self, doc: ADoc, text: RcStr) {
        self.dirty_docs.insert(doc);
        self.doc_texts.insert(doc, text);
        self.doc_syntax_map.remove(&doc);
        self.doc_analysis_map.remove(&doc);
    }

    pub(crate) fn close_doc(&mut self, doc: ADoc) {
        self.dirty_docs.insert(doc);
        self.doc_texts.remove(&doc);
        self.doc_syntax_map.remove(&doc);
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

            let (syntax, analysis) = {
                let tokens = crate::token::tokenize(doc, text.clone());
                let p_tokens: RcSlice<_> = PToken::from_tokens(tokens.into()).into();
                let root = crate::parse::parse_root(p_tokens.to_owned());
                let analysis = super::analyze::analyze(&root);

                let syntax = ASyntax {
                    tokens: p_tokens,
                    tree: root,
                };
                (syntax, analysis)
            };

            self.doc_syntax_map.insert(doc, syntax);
            self.doc_analysis_map.insert(doc, analysis);
        }

        for analysis in self.doc_analysis_map.values_mut() {
            analysis.invalidate_previous_workspace_analysis();
        }

        // 複数ファイルに渡る環境を構築する。
        self.global_env.clear();
        self.toplevel_env.clear();
        for (&doc, analysis) in &self.doc_analysis_map {
            analysis.extend_public_env(doc, &mut self.global_env, &mut self.toplevel_env);
        }

        let public_env = APublicEnv {
            global: &self.global_env,
            toplevel: &self.toplevel_env,
        };

        // シンボルの定義・参照箇所を決定する。
        self.def_sites.clear();
        self.use_sites.clear();

        for (&doc, analysis) in &mut self.doc_analysis_map {
            analysis.collect_explicit_def_sites(doc, &mut self.def_sites);
        }

        for (&doc, analysis) in &mut self.doc_analysis_map {
            analysis.resolve_symbol_def_candidates(
                doc,
                &public_env,
                &mut self.def_sites,
                &mut self.use_sites,
            );
        }

        for (_, analysis) in &mut self.doc_analysis_map {
            analysis.resolve_symbol_use_candidates(&public_env, &mut self.use_sites);
        }

        // eprintln!("global_env={:#?}", &self.global_env);
        // eprintln!("analysis_map={:#?}", &self.doc_analysis_map);
        // eprintln!("def_sites={:#?}", &self.def_sites);
        // eprintln!("use_sites={:#?}", &self.use_sites);
    }

    pub(crate) fn locate_symbol(&mut self, doc: ADoc, pos: APos) -> Option<(AWsSymbol, ALoc)> {
        self.compute();

        // eprintln!("symbol_uses={:?}", &self.use_sites);

        self.def_sites
            .iter()
            .chain(&self.use_sites)
            .find(|&(_, loc)| loc.is_touched(doc, pos))
            .cloned()
    }

    pub(crate) fn get_ident_at(&mut self, doc: ADoc, pos: APos) -> Option<(RcStr, ALoc)> {
        self.compute();

        let syntax = self.doc_syntax_map.get(&doc)?;
        let tokens = &syntax.tokens;
        let token = match tokens.binary_search_by_key(&pos, |t| t.body.loc.start()) {
            Ok(i) => tokens[i].body.as_ref(),
            Err(i) => tokens
                .iter()
                .enumerate()
                .skip(i.saturating_sub(1))
                .take(3)
                .find_map(|(i, t)| {
                    if t.body.kind == TokenKind::Ident && t.body.loc.range.is_touched(pos) {
                        Some(t.body.as_ref())
                    } else {
                        None
                    }
                })?,
        };
        Some((token.text.clone(), token.loc))
    }

    pub(crate) fn symbol_name(&self, wa_symbol: AWsSymbol) -> Option<&str> {
        self.doc_analysis_map
            .get(&wa_symbol.doc)?
            .symbol_name(wa_symbol.symbol)
    }

    pub(crate) fn get_symbol_details(
        &self,
        wa_symbol: AWsSymbol,
    ) -> Option<(RcStr, ASymbolDetails)> {
        let doc_analysis = self.doc_analysis_map.get(&wa_symbol.doc)?;
        let (name, details) = doc_analysis.get_symbol_details(wa_symbol.symbol)?;
        Some((name, details))
    }

    pub(crate) fn collect_symbol_defs(&mut self, ws_symbol: AWsSymbol, locs: &mut Vec<ALoc>) {
        self.compute();

        for &(s, loc) in &self.def_sites {
            if s == ws_symbol {
                locs.push(loc);
            }
        }
    }

    pub(crate) fn collect_symbol_uses(&mut self, ws_symbol: AWsSymbol, locs: &mut Vec<ALoc>) {
        self.compute();

        for &(s, loc) in &self.use_sites {
            if s == ws_symbol {
                locs.push(loc);
            }
        }
    }

    pub(crate) fn collect_completion_items(&mut self, loc: ALoc) -> Vec<ACompletionItem> {
        self.compute();

        let mut completion_items = vec![];

        let mut scope = ALocalScope::default();

        if let Some(doc_analysis) = self.doc_analysis_map.get(&loc.doc) {
            let pos = loc.start();
            scope = doc_analysis.resolve_scope_at(pos);
            doc_analysis.collect_local_completion_items(scope, &mut completion_items);
        }

        if scope.is_outside_module() {
            for (&doc, doc_analysis) in &self.doc_analysis_map {
                if doc != loc.doc {
                    doc_analysis.collect_local_completion_items(scope, &mut completion_items);
                }
            }
        }

        for doc_analysis in self.doc_analysis_map.values() {
            doc_analysis.collect_global_completion_items(&mut completion_items);
        }

        completion_items
    }
}

#[allow(unused)]
pub(crate) struct ASyntax {
    pub(crate) tokens: RcSlice<PToken>,
    pub(crate) tree: PRoot,
}

pub(crate) struct APublicEnv<'a> {
    global: &'a HashMap<RcStr, AWsSymbol>,
    toplevel: &'a HashMap<RcStr, AWsSymbol>,
}

impl<'a> APublicEnv<'a> {
    pub(crate) fn in_global(&self, name: &str) -> Option<AWsSymbol> {
        self.global.get(name).cloned()
    }

    pub(crate) fn in_toplevel(&self, name: &str) -> Option<AWsSymbol> {
        match self.toplevel.get(name) {
            Some(symbol) => Some(*symbol),
            None => self.in_global(name),
        }
    }

    pub(crate) fn resolve(&self, name: &str, is_toplevel: bool) -> Option<AWsSymbol> {
        if is_toplevel {
            self.in_toplevel(name)
        } else {
            self.in_global(name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AWorkspaceAnalysis;
    use crate::analysis::{ADoc, APos};
    use std::collections::HashMap;

    /// `<|x|>` のようなマーカーを含む文字列を受け取る。間に挟まれている x の部分をマーカーの名前と呼ぶ。
    /// マーカーを取り除いた文字列 text と、text の中でマーカーが指している位置のリストを返す。
    fn parse_cursor_string(s: &str) -> (String, Vec<(&str, APos)>) {
        let mut output = vec![];

        let mut text = String::with_capacity(s.len());
        let mut pos = APos::default();
        let mut i = 0;

        while let Some(offset) = s[i..].find("<|") {
            // カーソルを <| の手前まで進める。
            let j = i + offset;
            text += &s[i..j];
            pos = pos.add(APos::from_str(&s[i..j]));
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

        let doc = ADoc::new(1);
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
                .locate_symbol(doc, pos)
                .and_then(|(symbol, _)| wa.symbol_name(symbol));
            assert_eq!(actual, expected_map[name], "name={}", name);
        }
    }

    #[test]
    fn test_it_works() {
        let mut wa = AWorkspaceAnalysis::default();

        let doc = ADoc::new(1);
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
                .locate_symbol(doc, pos)
                .and_then(|(symbol, _)| wa.symbol_name(symbol));
            assert_eq!(actual, expected_map[name], "name={}", name);
        }
    }
}
