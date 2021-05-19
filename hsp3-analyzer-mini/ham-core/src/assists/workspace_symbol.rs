use super::*;
use crate::analysis::*;
use lsp_types::SymbolInformation;

// completion, textDocument/documentSymbol も参照
fn to_lsp_symbol_kind(kind: ASymbolKind) -> Option<lsp_types::SymbolKind> {
    use lsp_types::SymbolKind as K;
    let it = match kind {
        // パラメータなどの単一ファイルにだけ属するシンボルはworkspace/symbolリクエストの結果には含めない。
        ASymbolKind::Unresolved
        | ASymbolKind::Unknown
        | ASymbolKind::Param(_)
        | ASymbolKind::Module
        | ASymbolKind::Field => return None,

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
        ASymbolKind::ComInterface => K::Interface,
    };
    Some(it)
}

pub(crate) fn symbol(
    query: &str,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
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

            Some(SymbolInformation {
                name: name.to_string(),
                kind,
                location,
                container_name: None,
                deprecated: None,
            })
        })
        .collect()
}
