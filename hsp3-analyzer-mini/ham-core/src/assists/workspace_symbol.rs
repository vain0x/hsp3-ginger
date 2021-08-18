use super::*;
use crate::analysis::*;
use lsp_types::SymbolInformation;

// completion, textDocument/documentSymbol も参照
fn to_lsp_symbol_kind(kind: HspSymbolKind) -> Option<lsp_types::SymbolKind> {
    use lsp_types::SymbolKind as K;
    let it = match kind {
        // パラメータなどの単一ファイルにだけ属するシンボルはworkspace/symbolリクエストの結果には含めない。
        HspSymbolKind::Unresolved
        | HspSymbolKind::Unknown
        | HspSymbolKind::Param(_)
        | HspSymbolKind::Module
        | HspSymbolKind::Field => return None,

        HspSymbolKind::StaticVar => K::Variable,
        HspSymbolKind::Label
        | HspSymbolKind::Const
        | HspSymbolKind::Enum
        | HspSymbolKind::Macro { ctype: false }
        | HspSymbolKind::PluginCmd => K::Constant,
        HspSymbolKind::Macro { ctype: true }
        | HspSymbolKind::DefFunc
        | HspSymbolKind::DefCFunc
        | HspSymbolKind::LibFunc => K::Function,
        HspSymbolKind::ModFunc | HspSymbolKind::ModCFunc | HspSymbolKind::ComFunc => K::Method,
        HspSymbolKind::ComInterface => K::Interface,
    };
    Some(it)
}

pub(crate) fn symbol(
    query: &str,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Vec<SymbolInformation> {
    let mut symbols = vec![];
    wa.require_some_project()
        .collect_all_symbols(query, &mut symbols);

    symbols
        .into_iter()
        .filter(|(symbol, _)| symbol.scope_opt.as_ref().map_or(false, |s| s.is_public()))
        .filter_map(|(symbol, loc)| {
            let name = symbol.name();
            let kind = to_lsp_symbol_kind(symbol.kind)?;
            let location = loc_to_location(loc, docs)?;

            Some(new_lsp_symbol_information(name.to_string(), kind, location))
        })
        .collect()
}
