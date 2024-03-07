use super::*;
use crate::{
    analysis::{HspSymbolKind, Scope, SymbolRc},
    assists::from_document_position,
    lang_service::docs::Docs,
    parse::p_param_ty::PParamCategory,
};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, Documentation, Position, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

/// `hsphelp` を参照して入力補完候補を列挙する (プリプロセッサ関連は除く)
fn collect_hsphelp_completion_items(
    wa: &AnalysisRef<'_>,
    completion_items: &mut Vec<lsp_types::CompletionItem>,
) {
    completion_items.extend(
        wa.hsphelp_info()
            .doc_symbols
            .iter()
            .filter(|(&doc, _)| wa.is_active_help_doc(doc))
            .flat_map(|(_, symbols)| symbols.iter().filter(|s| !s.label.starts_with("#")))
            .cloned(),
    );
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
    wa: &AnalysisRef<'_>,
    uri: &Url,
    position: Position,
    docs: &Docs,
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

    {
        let mut symbols = vec![];
        collect_symbols_in_scope(wa, doc, pos, &mut symbols);

        for symbol in symbols {
            // `hsphelp` に記載されているシンボルは除く
            if symbol.linked_symbol_opt.borrow().is_some() {
                continue;
            }

            items.push(to_lsp_completion_item(&symbol));
        }
    }

    collect_hsphelp_completion_items(wa, &mut items);

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
    wa: &AnalysisRef<'_>,
    uri: Url,
    position: Position,
    docs: &Docs,
) -> Option<CompletionList> {
    let mut completion_list = do_completion(wa, &uri, position, docs)?;

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
    wa: &AnalysisRef<'_>,
    mut resolved_item: CompletionItem,
    docs: &Docs,
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

    let list = do_completion(wa, &uri, position, docs)?;
    let item = list
        .items
        .into_iter()
        .find(|i| i.label == resolved_item.label)?;
    resolved_item.documentation = item.documentation;
    resolved_item.data = data_opt;
    Some(resolved_item)
}
