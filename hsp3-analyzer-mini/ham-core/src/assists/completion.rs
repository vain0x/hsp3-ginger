use super::to_loc;
use crate::{
    analysis::{
        comment::calculate_details,
        integrate::{ACompletionItem, AWorkspaceAnalysis},
        AScope, ASymbolKind,
    },
    lang_service::docs::Docs,
    parse::p_param_ty::PParamCategory,
};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, Documentation, Position, Url};

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
    wa: &mut AWorkspaceAnalysis,
    other_items: &[CompletionItem],
) -> Option<CompletionList> {
    let mut items = vec![];

    let loc = to_loc(&uri, position, docs)?;

    for item in wa.collect_completion_items(loc) {
        match item {
            ACompletionItem::Symbol(symbol) => {
                let details = calculate_details(&symbol.comments);

                use CompletionItemKind as K;

                let kind = match symbol.kind {
                    ASymbolKind::Unresolved => K::Text,
                    ASymbolKind::Label => K::Value,
                    ASymbolKind::StaticVar => K::Variable,
                    ASymbolKind::Const => K::Constant,
                    ASymbolKind::Enum => K::EnumMember,
                    ASymbolKind::Macro { ctype: false } => K::Value,
                    ASymbolKind::Macro { ctype: true } => K::Function,
                    ASymbolKind::DefFunc => K::Method,
                    ASymbolKind::DefCFunc => K::Function,
                    ASymbolKind::ModFunc => K::Method,
                    ASymbolKind::ModCFunc => K::Function,
                    ASymbolKind::Param(None) => K::Variable,
                    ASymbolKind::Param(Some(param)) => match param.category() {
                        PParamCategory::ByValue => K::Value,
                        PParamCategory::ByRef => K::Property,
                        PParamCategory::Local => K::Variable,
                        PParamCategory::Auto => K::Text,
                    },
                    ASymbolKind::Module => K::Module,
                    ASymbolKind::Field => K::Field,
                    ASymbolKind::LibFunc => K::Function,
                    ASymbolKind::PluginCmd => K::Keyword,
                    ASymbolKind::ComInterface => K::Interface,
                    ASymbolKind::ComFunc => K::Method,
                };

                // 候補の順番を制御するための文字。(スコープが狭いものを上に出す。)
                let sort_prefix = match (symbol.scope, symbol.kind) {
                    (AScope::Local(local), _) => match (local.module_opt, local.deffunc_opt) {
                        (Some(_), Some(_)) => 'a',
                        (Some(_), None) => 'b',
                        (None, None) => 'c',
                        (None, Some(_)) => 'd',
                    },
                    (_, ASymbolKind::Module) => 'f',
                    (AScope::Global, _) => 'e',
                };

                items.push(CompletionItem {
                    kind: Some(kind),
                    label: symbol.name.to_string(),
                    detail: details.desc.map(|s| s.to_string()),
                    documentation: if details.docs.is_empty() {
                        None
                    } else {
                        Some(Documentation::String(details.docs.join("\r\n\r\n")))
                    },
                    sort_text: Some(format!("{}{}", sort_prefix, symbol.name)),
                    ..CompletionItem::default()
                });
            }
        }
    }

    items.extend(other_items.iter().cloned());

    Some(CompletionList {
        is_incomplete: false,
        items,
    })
}
