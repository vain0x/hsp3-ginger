use super::*;
use crate::analyzer::AnalyzerRef;

pub(crate) type DocAnalysisMap = HashMap<DocId, DocAnalysis>;

impl AnalyzerRef<'_> {
    pub(crate) fn hsphelp_info(&self) -> &HspHelpInfo {
        &self.owner.hsphelp_info
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
        let da = self.doc_analysis_map.get(&doc)?;
        Some(DocSyntax {
            text: da.text.clone(),
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

    pub(crate) fn locate_symbol(&self, doc: DocId, pos: Pos16) -> Option<(SymbolRc, Loc)> {
        self.def_sites
            .iter()
            .chain(self.use_sites)
            .find(|&(_, loc)| loc.is_touched(doc, pos))
            .cloned()
    }

    // pub(crate) fn diagnose(&self, diagnostics: &mut Vec<(String, Loc)>) {
    //     self.diagnose_precisely(diagnostics);
    // }

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

    pub(crate) fn diagnose_precisely(&self, diagnostics: &mut Vec<(String, Loc)>) {
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
    pub(crate) fn generate(an: &AnalyzerRef<'_>, doc: DocId) -> Self {
        let use_site_map = an
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
    an: &AnalyzerRef<'_>,
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
        an.hsphelp_info()
            .doc_symbols
            .iter()
            .filter(|(&doc, _)| an.is_active_help_doc(doc))
            .flat_map(|(_, symbols)| symbols.iter().filter(|s| s.label.starts_with("#")))
            .cloned(),
    );
}

/// 指定位置のスコープに属するシンボルを列挙する (入力補完用)
pub(crate) fn collect_symbols_in_scope(
    an: &AnalyzerRef<'_>,
    doc: DocId,
    pos: Pos16,
    out_symbols: &mut Vec<SymbolRc>,
) {
    let scope = match an.doc_analysis_map.get(&doc) {
        Some(da) => resolve_scope_at(da, pos),
        None => return,
    };

    let doc_symbols = an
        .doc_symbols_map
        .iter()
        .filter_map(|(&d, symbols)| {
            if d == doc || an.is_active_doc(d) {
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
    an: &AnalyzerRef<'_>,
    doc: DocId,
    symbols: &mut Vec<(SymbolRc, Loc)>,
) {
    let doc_symbols = match an.doc_symbols_map.get(&doc) {
        Some(it) => it,
        None => return,
    };

    let def_site_map = an
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DefOrUse {
    Def,
    Use,
}

/// 指定したドキュメントに含まれる、指定したシンボルの出現箇所をすべて列挙する
///
/// (`documentHighlight` 用)
///
/// - 指定したシンボルの、そのドキュメント内での出現箇所、それぞれにつき `on_site` 関数が呼ばれる
/// - 出現箇所が複数ある場合、位置が前にあるものから順に呼び出しが行われる。
///     同じ位置に対して複数回の呼び出しが行われることはない
pub(crate) fn collect_highlights(
    an: &AnalyzerRef<'_>,
    doc: DocId,
    symbol: &SymbolRc,
    mut on_site: impl FnMut(DefOrUse, Loc),
) {
    let mut sites: Vec<(Loc, DefOrUse)> = vec![];
    for (s, loc) in an.def_sites {
        if loc.doc == doc && s == symbol {
            sites.push((*loc, DefOrUse::Def));
        }
    }
    for (s, loc) in an.use_sites {
        if loc.doc == doc && s == symbol {
            sites.push((*loc, DefOrUse::Use));
        }
    }

    // 位置でソートする。同じ位置に定義・使用箇所が複数ある場合、定義だけ残して重複を除去する (dedup)
    sites.sort();
    sites.dedup_by_key(|(loc, _)| *loc);

    for (loc, kind) in sites {
        on_site(kind, loc);
    }
}

/// 指定したドキュメント内のすべてのシンボルの出現箇所 (定義・使用両方) を列挙する
/// (セマンティックトークン用)
pub(crate) fn collect_symbol_occurrences_in_doc<'a>(
    an: &AnalyzerRef<'a>,
    doc: DocId,
    symbols: &mut Vec<(&'a SymbolRc, Loc)>,
) {
    for (symbol, loc) in an.def_sites.iter().chain(an.use_sites) {
        if loc.doc == doc {
            symbols.push((symbol, *loc));
        }
    }
}

pub(crate) struct CollectSymbolOptions {
    pub(crate) include_def: bool,
    pub(crate) include_use: bool,
}

/// 指定したシンボルの定義箇所・使用箇所を列挙する (順不同、重複あり)
pub(crate) fn collect_symbol_occurrences(
    an: &AnalyzerRef<'_>,
    options: CollectSymbolOptions,
    symbol: &SymbolRc,
    locs: &mut Vec<Loc>,
) {
    if options.include_def {
        for (s, loc) in an.def_sites {
            if *s == *symbol {
                locs.push(*loc);
            }
        }
    }
    if options.include_use {
        for (s, loc) in an.use_sites {
            if *s == *symbol {
                locs.push(*loc);
            }
        }
    }
}

pub(crate) fn collect_workspace_symbols(
    an: &AnalyzerRef<'_>,
    query: &str,
    symbols: &mut Vec<(SymbolRc, Loc)>,
) {
    let name_filter = query.trim().to_ascii_lowercase();

    let map = an
        .def_sites
        .iter()
        .filter(|(symbol, _)| symbol.name.contains(&name_filter))
        .map(|(symbol, loc)| (symbol.clone(), *loc))
        .collect::<HashMap<_, _>>();

    for (&doc, doc_symbols) in an.doc_symbols_map.iter() {
        if !an.active_docs.contains(&doc) {
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
pub(crate) fn find_include_target(an: &AnalyzerRef<'_>, doc: DocId, pos: Pos16) -> Option<DocId> {
    // FIXME: 再実装
    // (include_resolutionが機能停止中のため無効化)
    // let (_, dest_doc) = *an
    //     .include_resolution
    //     .iter()
    //     .find(|&(loc, _)| loc.is_touched(doc, pos))?;

    // Some(dest_doc)
    None
}
