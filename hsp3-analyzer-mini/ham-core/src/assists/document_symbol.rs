use super::*;
use crate::{analysis::*, parse::p_param_ty::PParamCategory};
use lsp_types::DocumentSymbolResponse;

// completion, workspace/symbol も参照
fn to_lsp_symbol_kind(kind: HspSymbolKind) -> Option<lsp_types::SymbolKind> {
    use lsp_types::SymbolKind as K;
    let it = match kind {
        HspSymbolKind::Unresolved => return None,
        HspSymbolKind::Module => K::MODULE,
        HspSymbolKind::StaticVar => K::VARIABLE,
        HspSymbolKind::Unknown
        | HspSymbolKind::Label
        | HspSymbolKind::Const
        | HspSymbolKind::Enum
        | HspSymbolKind::Macro { ctype: false }
        | HspSymbolKind::Param(None)
        | HspSymbolKind::PluginCmd => K::CONSTANT,
        HspSymbolKind::Macro { ctype: true }
        | HspSymbolKind::DefFunc
        | HspSymbolKind::DefCFunc
        | HspSymbolKind::LibFunc => K::FUNCTION,
        HspSymbolKind::ModFunc | HspSymbolKind::ModCFunc | HspSymbolKind::ComFunc => K::METHOD,
        HspSymbolKind::Param(Some(param)) => match param.category() {
            PParamCategory::ByValue => K::CONSTANT,
            PParamCategory::ByRef => K::PROPERTY,
            PParamCategory::Local => K::VARIABLE,
            PParamCategory::Auto => return None,
        },
        HspSymbolKind::Field => K::FIELD,
        HspSymbolKind::ComInterface => K::INTERFACE,
    };
    Some(it)
}

pub(crate) fn symbol(
    uri: Url,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<DocumentSymbolResponse> {
    let doc = docs.find_by_uri(&CanonicalUri::from_url(&uri))?;

    let mut symbols = vec![];
    wa.require_project_for_doc(doc)
        .collect_doc_symbols(doc, &mut symbols);

    symbols.sort_by_key(|s| s.1.start());

    let symbol_information_list = symbols
        .into_iter()
        .filter_map(|(symbol, loc)| {
            let name = symbol.name();
            let kind = to_lsp_symbol_kind(symbol.kind)?;
            let location = loc_to_location(loc, docs)?;

            Some(new_lsp_symbol_information(name.to_string(), kind, location))
        })
        .collect();

    Some(DocumentSymbolResponse::Flat(symbol_information_list))
}
