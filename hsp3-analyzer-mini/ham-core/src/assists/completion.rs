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
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

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

fn to_completion_symbol_kind(kind: HspSymbolKind) -> CompletionItemKind {
    // textDocument/documentSymbol, workspace/symbol も参照
    use CompletionItemKind as K;

    match kind {
        HspSymbolKind::Unresolved | HspSymbolKind::Unknown => K::TEXT,
        HspSymbolKind::Label => K::VALUE,
        HspSymbolKind::StaticVar => K::VARIABLE,
        HspSymbolKind::Const => K::CONSTANT,
        HspSymbolKind::Enum => K::ENUM_MEMBER,
        HspSymbolKind::Macro { ctype: false } => K::VALUE,
        HspSymbolKind::Macro { ctype: true } => K::FUNCTION,
        HspSymbolKind::DefFunc => K::METHOD,
        HspSymbolKind::DefCFunc => K::FUNCTION,
        HspSymbolKind::ModFunc => K::METHOD,
        HspSymbolKind::ModCFunc => K::FUNCTION,
        HspSymbolKind::Param(None) => K::VARIABLE,
        HspSymbolKind::Param(Some(param)) => match param.category() {
            PParamCategory::ByValue => K::VALUE,
            PParamCategory::ByRef => K::PROPERTY,
            PParamCategory::Local => K::VARIABLE,
            PParamCategory::Auto => K::TEXT,
        },
        HspSymbolKind::Module => K::MODULE,
        HspSymbolKind::Field => K::FIELD,
        HspSymbolKind::LibFunc => K::FUNCTION,
        HspSymbolKind::PluginCmd => K::KEYWORD,
        HspSymbolKind::ComInterface => K::INTERFACE,
        HspSymbolKind::ComFunc => K::METHOD,
    }
}

fn to_lsp_completion_item(symbol: &SymbolRc) -> CompletionItem {
    let details = symbol.compute_details();
    let detail = details.desc.map(|s| s.to_string());
    let documentation = if details.docs.is_empty() {
        None
    } else {
        Some(Documentation::String(details.docs.join("\r\n\r\n")))
    };

    let sort_text = {
        let sort_prefix = match (&symbol.scope_opt, symbol.kind) {
            (Some(Scope::Local(local)), _) => match (&local.module_opt, local.deffunc_opt) {
                (Some(_), Some(_)) => 'a',
                (Some(_), None) => 'b',
                (None, None) => 'c',
                (None, Some(_)) => 'd',
            },
            (_, HspSymbolKind::Module) => 'f',
            (Some(Scope::Global), _) => 'e',
            (None, _) => 'g',
        };
        Some(format!("{}{}", sort_prefix, symbol.name))
    };

    CompletionItem {
        kind: Some(to_completion_symbol_kind(symbol.kind)),
        label: symbol.name.to_string(),
        detail,
        documentation,
        sort_text,
        ..CompletionItem::default()
    }
}

fn new_completion_list(items: Vec<CompletionItem>) -> CompletionList {
    CompletionList {
        is_incomplete: false,
        items,
    }
}

pub(crate) fn incomplete_completion_list() -> CompletionList {
    CompletionList {
        is_incomplete: true,
        items: vec![],
    }
}

fn do_completion(
    uri: &Url,
    position: Position,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<CompletionList> {
    let mut items = vec![];

    let (doc, pos) = from_document_position(uri, position, docs)?;

    if wa.in_str_or_comment(doc, pos).unwrap_or(true) {
        return None;
    }

    if wa.in_preproc(doc, pos).unwrap_or(false) {
        collect_preproc_completion_items(wa, &mut items);
        return Some(new_completion_list(items));
    }

    let mut completion_items = vec![];
    let p = wa.require_project_for_doc(doc);
    p.collect_completion_items(doc, pos, &mut completion_items);

    for item in completion_items {
        match item {
            ACompletionItem::Symbol(symbol) => {
                if symbol.linked_symbol_opt.borrow().is_some() {
                    continue;
                }

                items.push(to_lsp_completion_item(&symbol));
            }
        }
    }

    p.collect_hsphelp_completion_items(&mut items);

    // HACK: 不要な候補を削除する。(__hspdef__ はスクリプトの記述的にインクルードガードとみなされないので有効なシンボルとして登録されてしまう。)
    if let Some(i) = items.iter().position(|item| item.label == "__hspdef__") {
        items.swap_remove(i);
    }

    // 重複した候補を削除する。
    {
        let mut set = HashSet::new();
        let retain = items
            .iter()
            .map(|item| set.insert(item.label.as_str()))
            .collect::<Vec<_>>();
        let mut i = 0;
        items.retain(|_| {
            i += 1;
            retain[i - 1]
        });
    }

    Some(new_completion_list(items))
}

#[derive(Serialize, Deserialize)]
struct CompletionData {
    // completionの結果を復元するためのデータ:
    uri: Url,
    position: Position,

    // 元の項目のdata
    data_opt: Option<Value>,
}

pub(crate) fn completion(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<CompletionList> {
    let mut completion_list = do_completion(&uri, position, docs, wa)?;

    for item in &mut completion_list.items {
        if item.documentation.is_none() && item.data.is_none() {
            continue;
        }

        // すべての候補のdocumentationを送信すると重たいので、削る。
        // この情報はresolveで復元する。
        item.documentation = None;

        // resolveリクエストで使うための情報を付与する。
        let data_opt = item.data.take();
        let data = CompletionData {
            uri: uri.clone(),
            position,
            data_opt,
        };
        item.data = Some(serde_json::to_value(&data).unwrap());
    }

    Some(completion_list)
}

pub(crate) fn completion_resolve(
    mut resolved_item: CompletionItem,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<CompletionItem> {
    let data: CompletionData = match resolved_item
        .data
        .take()
        .and_then(|data| serde_json::from_value(data).ok())
    {
        Some(it) => it,
        None => {
            // 復元すべきデータはもともとない。
            return Some(resolved_item);
        }
    };

    // completionの計算を再試行して情報を復元する。(重い)

    let CompletionData {
        uri,
        position,
        data_opt,
    } = data;

    let list = do_completion(&uri, position, docs, wa)?;
    let item = list
        .items
        .into_iter()
        .find(|i| i.label == resolved_item.label)?;
    resolved_item.documentation = item.documentation;
    resolved_item.data = data_opt;
    Some(resolved_item)
}
