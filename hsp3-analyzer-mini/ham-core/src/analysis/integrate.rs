use super::{a_symbol::AWsSymbol, analyze::AAnalysis, ADoc, ALoc, APos};
use crate::utils::rc_str::RcStr;
use std::{
    collections::{HashMap, HashSet},
    mem::replace,
};

#[derive(Default)]
pub(crate) struct AWorkspaceAnalysis {
    pub(crate) dirty_docs: HashSet<ADoc>,
    pub(crate) doc_texts: HashMap<ADoc, RcStr>,
    pub(crate) doc_analysis_map: HashMap<ADoc, AAnalysis>,

    // すべてのドキュメントの解析結果を使って構築される情報
    pub(crate) global_env: HashMap<RcStr, AWsSymbol>,
    pub(crate) use_sites: Vec<(AWsSymbol, ALoc)>,
    pub(crate) use_site_resolution_count: usize,
}

impl AWorkspaceAnalysis {
    pub(crate) fn update_doc(&mut self, doc: ADoc, text: RcStr) {
        self.dirty_docs.insert(doc);
        self.doc_texts.insert(doc, text);
        self.doc_analysis_map.remove(&doc);
    }

    pub(crate) fn close_doc(&mut self, doc: ADoc) {
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

            let analysis = {
                let tokens = crate::token::tokenize(doc, text.clone());
                let tokens = crate::parse::PToken::from_tokens(tokens);
                let root = crate::parse::parse_root(tokens);
                super::analyze::analyze(&root)
            };
            self.doc_analysis_map.insert(doc, analysis);
        }

        self.global_env.clear();
        for (&doc, analysis) in &self.doc_analysis_map {
            analysis.collect_global_symbols(doc, &mut self.global_env);
        }

        self.use_sites.clear();
        self.use_site_resolution_count = 0;

        // eprintln!("global_env={:?}", &self.global_env);
        // eprintln!("analysis_map={:#?}", &self.doc_analysis_map);
    }

    pub(crate) fn locate_symbol(&mut self, doc: ADoc, pos: APos) -> Option<(AWsSymbol, ALoc)> {
        self.compute();

        let analysis = self.doc_analysis_map.get_mut(&doc)?;

        // resolve uses
        let ok = analysis.resolve_symbol_use(&self.global_env, &mut self.use_sites);
        if ok {
            self.use_site_resolution_count += 1;
        }

        // eprintln!("symbol_uses={:?}", &self.use_sites);

        self.use_sites
            .iter()
            .find(|&(_, loc)| loc.is_touched(doc, pos))
            .cloned()
    }

    fn symbol_name(&self, ws_symbol: AWsSymbol) -> Option<&str> {
        let analysis = &self.doc_analysis_map.get(&ws_symbol.doc)?;
        analysis.symbol_name(ws_symbol.symbol)
    }

    pub(crate) fn get_symbol_uses(&mut self, ws_symbol: AWsSymbol, locs: &mut Vec<ALoc>) {
        self.compute();

        // resolve all uses
        let doc_count = self.doc_analysis_map.len();
        if replace(&mut self.use_site_resolution_count, doc_count) < doc_count {
            for (_, analysis) in &mut self.doc_analysis_map {
                analysis.resolve_symbol_use(&self.global_env, &mut self.use_sites);
            }
        }

        for &(s, loc) in &self.use_sites {
            if s == ws_symbol {
                locs.push(loc);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AWorkspaceAnalysis;
    use crate::analysis::{ADoc, APos};
    use std::collections::HashMap;

    /// `<|x|>` のようなマーカーを含む文字列を受け取る。間に挟まれている x の部分をマーカーの名前と呼ぶ。
    /// 文字列からマーカーを取り除いた文字列 text と、text の中でマーカーが指している位置のリストを返す。
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
    fn test_it_works() {
        let mut wa = AWorkspaceAnalysis::default();

        let doc = ADoc::new(1);
        let text = r#"
            #module
            #deffunc hello
                mes "Hello, world!"
                return
            #global

                <|B|>hello
                hello<|C|> <|D|>
        "#;
        let expected_map = vec![("B", Some("hello")), ("C", Some("hello")), ("D", None)]
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
