use super::*;
use crate::{analysis::*, parse::p_param_ty::PParamCategory};
use lsp_types::{DocumentSymbolResponse, SymbolInformation};

// completion, workspace/symbol も参照
fn to_lsp_symbol_kind(kind: ASymbolKind) -> Option<lsp_types::SymbolKind> {
    use lsp_types::SymbolKind as K;
    let it = match kind {
        ASymbolKind::Unresolved => return None,
        ASymbolKind::Unknown | ASymbolKind::Module | ASymbolKind::Param(None) => K::Unknown,
        ASymbolKind::StaticVar => K::Variable,
        ASymbolKind::Label
        | ASymbolKind::Const
        | ASymbolKind::Enum
        | ASymbolKind::Macro { ctype: false }
        | ASymbolKind::PluginCmd => K::Constant,
        ASymbolKind::Macro { ctype: true }
        | ASymbolKind::DefFunc
        | ASymbolKind::DefCFunc
        | ASymbolKind::LibFunc => K::Function,
        ASymbolKind::ModFunc | ASymbolKind::ModCFunc | ASymbolKind::ComFunc => K::Method,
        ASymbolKind::Param(Some(param)) => match param.category() {
            PParamCategory::ByValue => K::Constant,
            PParamCategory::ByRef => K::Property,
            PParamCategory::Local => K::Variable,
            PParamCategory::Auto => return None,
        },
        ASymbolKind::Field => K::Field,
        ASymbolKind::ComInterface => K::Interface,
    };
    Some(it)
}

pub(crate) fn symbol(
    uri: Url,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Option<DocumentSymbolResponse> {
    let doc = docs.find_by_uri(&CanonicalUri::from_url(&uri))?;

    let mut symbols = vec![];
    wa.collect_doc_symbols(doc, &mut symbols);

    symbols.sort_by_key(|s| s.1.start());

    let symbol_information_list = symbols
        .into_iter()
        .filter_map(|(symbol, loc)| {
            let name = symbol.name();
            let kind = to_lsp_symbol_kind(symbol.kind)?;
            let location = loc_to_location(loc, docs)?;

            Some(SymbolInformation {
                name: name.to_string(),
                kind,
                location,
                container_name: None,
                deprecated: None,
            })
        })
        .collect();

    Some(DocumentSymbolResponse::Flat(symbol_information_list))
}
