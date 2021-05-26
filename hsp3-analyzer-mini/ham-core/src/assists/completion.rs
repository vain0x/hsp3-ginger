use super::*;
use crate::{
    analysis::{HspSymbolKind, LocalScope, Scope, SymbolRc},
    assists::from_document_position,
    lang_service::docs::Docs,
    parse::{p_param_ty::PParamCategory, PToken},
    source::*,
    token::TokenKind,
};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, Documentation, Position, Url};

pub(crate) enum ACompletionItem {
    Symbol(SymbolRc),
}

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

fn collect_local_completion_items(
    symbols: &[SymbolRc],
    local: &LocalScope,
    completion_items: &mut Vec<ACompletionItem>,
) {
    for s in symbols {
        let scope = match &s.scope_opt {
            Some(it) => it,
            None => continue,
        };
        if scope.is_visible_to(local) {
            completion_items.push(ACompletionItem::Symbol(s.clone()));
        }
    }
}

fn collect_global_completion_items(
    symbols: &[SymbolRc],
    completion_items: &mut Vec<ACompletionItem>,
) {
    for s in symbols {
        if let Some(Scope::Global) = s.scope_opt {
            completion_items.push(ACompletionItem::Symbol(s.clone()));
        }
    }
}

pub(crate) fn collect_symbols_as_completion_items(
    doc: DocId,
    scope: LocalScope,
    doc_symbols: &[(DocId, &[SymbolRc])],
    completion_items: &mut Vec<ACompletionItem>,
) {
    if let Some((_, symbols)) = doc_symbols.iter().find(|&&(d, _)| d == doc) {
        collect_local_completion_items(symbols, &scope, completion_items);
    }

    if scope.is_outside_module() {
        for &(d, symbols) in doc_symbols {
            if d == doc {
                continue;
            }

            collect_local_completion_items(symbols, &scope, completion_items);
        }
    }

    for &(_, symbols) in doc_symbols {
        collect_global_completion_items(symbols, completion_items);
    }
}

pub(crate) fn incomplete_completion_list() -> CompletionList {
    CompletionList {
        is_incomplete: true,
        items: vec![],
    }
}

pub(crate) fn completion(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<CompletionList> {
    let mut items = vec![];

    let (doc, pos) = from_document_position(&uri, position, docs)?;

    if wa.in_str_or_comment(doc, pos).unwrap_or(true) {
        return None;
    }

    if wa.in_preproc(doc, pos).unwrap_or(false) {
        wa.require_project_for_doc(doc)
            .collect_preproc_completion_items(&mut items);
        return Some(CompletionList {
            is_incomplete: false,
            items,
        });
    }

    let mut completion_items = vec![];
    let p = wa.require_project_for_doc(doc);
    p.collect_completion_items(doc, pos, &mut completion_items);

    for item in completion_items {
        match item {
            ACompletionItem::Symbol(symbol) => {
                let details = symbol.compute_details();

                // textDocument/documentSymbol, workspace/symbol も参照
                use CompletionItemKind as K;

                let kind = match symbol.kind {
                    HspSymbolKind::Unresolved | HspSymbolKind::Unknown => K::Text,
                    HspSymbolKind::Label => K::Value,
                    HspSymbolKind::StaticVar => K::Variable,
                    HspSymbolKind::Const => K::Constant,
                    HspSymbolKind::Enum => K::EnumMember,
                    HspSymbolKind::Macro { ctype: false } => K::Value,
                    HspSymbolKind::Macro { ctype: true } => K::Function,
                    HspSymbolKind::DefFunc => K::Method,
                    HspSymbolKind::DefCFunc => K::Function,
                    HspSymbolKind::ModFunc => K::Method,
                    HspSymbolKind::ModCFunc => K::Function,
                    HspSymbolKind::Param(None) => K::Variable,
                    HspSymbolKind::Param(Some(param)) => match param.category() {
                        PParamCategory::ByValue => K::Value,
                        PParamCategory::ByRef => K::Property,
                        PParamCategory::Local => K::Variable,
                        PParamCategory::Auto => K::Text,
                    },
                    HspSymbolKind::Module => K::Module,
                    HspSymbolKind::Field => K::Field,
                    HspSymbolKind::LibFunc => K::Function,
                    HspSymbolKind::PluginCmd => K::Keyword,
                    HspSymbolKind::ComInterface => K::Interface,
                    HspSymbolKind::ComFunc => K::Method,
                };

                // 候補の順番を制御するための文字。(スコープが狭いものを上に出す。)
                let sort_prefix = match (&symbol.scope_opt, symbol.kind) {
                    (Some(Scope::Local(local)), _) => {
                        match (&local.module_opt, local.deffunc_opt) {
                            (Some(_), Some(_)) => 'a',
                            (Some(_), None) => 'b',
                            (None, None) => 'c',
                            (None, Some(_)) => 'd',
                        }
                    }
                    (_, HspSymbolKind::Module) => 'f',
                    (Some(Scope::Global), _) => 'e',
                    (None, _) => 'g',
                };

                let detail = details.desc.map(|s| s.to_string());
                let documentation = if details.docs.is_empty() {
                    None
                } else {
                    Some(Documentation::String(details.docs.join("\r\n\r\n")))
                };

                items.push(CompletionItem {
                    kind: Some(kind),
                    label: symbol.name.to_string(),
                    detail,
                    documentation,
                    sort_text: Some(format!("{}{}", sort_prefix, symbol.name)),
                    ..CompletionItem::default()
                });
            }
        }
    }

    p.collect_hsphelp_completion_items(&mut items);

    Some(CompletionList {
        is_incomplete: false,
        items,
    })
}
