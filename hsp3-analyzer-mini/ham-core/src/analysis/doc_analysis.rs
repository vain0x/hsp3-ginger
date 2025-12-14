use super::*;

/// ドキュメント単位の解析結果
#[derive(Default)]
pub(crate) struct DocAnalysis {
    pub(crate) doc_opt: Option<DocId>,
    pub(crate) text: RcStr,

    // 構文:
    pub(crate) tokens: RcSlice<PToken>,
    pub(crate) tree_opt: Option<PRoot>,

    // プリプロセス:
    pub(crate) include_guard: Option<IncludeGuard>,
    pub(crate) includes: Vec<(RcStr, Loc)>,
    pub(crate) uses: Vec<(String, Loc)>,
    pub(crate) module_map: ModuleMap,
    pub(crate) deffunc_map: DefFuncMap,
    pub(crate) preproc_symbols: Vec<SymbolRc>,
    // 構文リント:
    // pub(crate) syntax_lint_done: bool,
    // pub(crate) syntax_lints: Vec<(SyntaxLint, Loc)>,
}

impl DocAnalysis {
    pub(crate) fn compute(&mut self, doc: DocId, text: RcStr) {
        let tokens = crate::token::tokenize(doc, text.clone());
        let p_tokens: RcSlice<_> = PToken::from_tokens(tokens.into()).into();
        let root = crate::parse::parse_root(p_tokens.to_owned());
        let preproc = crate::analysis::preproc::analyze_preproc(doc, &root);

        self.doc_opt = Some(doc);
        self.set_syntax(text, p_tokens, root);
        self.set_preproc(preproc);
    }

    fn set_syntax(&mut self, text: RcStr, tokens: RcSlice<PToken>, tree: PRoot) {
        self.text = text;
        self.tokens = tokens;
        self.tree_opt = Some(tree);
    }

    fn set_preproc(&mut self, preproc: PreprocAnalysisResult) {
        self.include_guard = preproc.include_guard;
        self.includes = preproc.includes;
        self.uses = preproc.uses;
        self.module_map = preproc.module_map;
        self.deffunc_map = preproc.deffunc_map;
        self.preproc_symbols = preproc.symbols;
    }
}

/// 指定した位置がコメント内か
pub(crate) fn in_str_or_comment(pos: Pos16, tokens: &[PToken]) -> bool {
    let i = match tokens.binary_search_by_key(&pos, |t| Pos16::from(t.ahead().range.start())) {
        Ok(i) | Err(i) => i.saturating_sub(1),
    };

    tokens[i..]
        .iter()
        .take_while(|t| t.ahead().start() <= pos)
        .flat_map(|t| t.iter())
        .filter(|t| t.loc.range.contains_inclusive(pos))
        .any(|t| match t.kind {
            TokenKind::Str => t.loc.range.start() < pos && pos < t.loc.range.end(),
            TokenKind::Comment => t.loc.range.start() < pos,
            _ => false,
        })
}

/// 指定した位置がプリプロセッサ行の中か
pub(crate) fn in_preproc(pos: Pos16, tokens: &[PToken]) -> bool {
    // '#' から文末の間においてプリプロセッサ関連の補完を有効化する。

    // 指定位置付近のトークンを探す。
    let mut i = match tokens.binary_search_by_key(&pos, |token| token.body_pos16()) {
        Ok(i) | Err(i) => i,
    };

    // 遡って '#' の位置を探す。ただしEOSをみつけたら終わり。
    loop {
        match tokens.get(i).map(|t| (t.kind(), t.body_pos())) {
            Some((TokenKind::Hash, p)) if p <= pos => return true,
            Some((TokenKind::Eos, p)) if p < pos => return false,
            _ if i == 0 => return false,
            _ => i -= 1,
        }
    }
}

pub(crate) fn on_include_guard(da: &DocAnalysis, pos: Pos16) -> Option<Loc> {
    let doc = da.doc_opt.unwrap();

    Some(
        da.include_guard
            .as_ref()
            .filter(|g| g.loc.is_touched(doc, pos))?
            .loc,
    )
}

pub(crate) fn get_ident_at(da: &DocAnalysis, pos: Pos16) -> Option<(RcStr, Loc)> {
    let tokens = &da.tokens;
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

pub(crate) fn resolve_scope_at(da: &DocAnalysis, pos: Pos16) -> LocalScope {
    let mut scope = LocalScope::default();

    scope.module_opt = da.module_map.iter().find_map(|(&m, module_data)| {
        if range_is_touched(&module_data.content_loc.range, pos) {
            Some(m.clone())
        } else {
            None
        }
    });

    scope.deffunc_opt = da.deffunc_map.iter().find_map(|(&d, deffunc_data)| {
        if range_is_touched(&deffunc_data.content_loc.range, pos) {
            Some(d)
        } else {
            None
        }
    });

    scope
}
