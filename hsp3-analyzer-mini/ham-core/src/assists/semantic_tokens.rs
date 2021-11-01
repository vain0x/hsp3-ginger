use super::*;
use crate::{analysis::*, parse::p_param_ty::PParamCategory};

// SemanticTokensLegend を参照
fn to_semantic_token_kind(symbol: &SymbolRc) -> Option<(u32, u32)> {
    let (ty, modifiers) = match symbol.kind {
        HspSymbolKind::Param(Some(param)) => match param.category() {
            PParamCategory::ByValue => (1, 0b01), // readonly variable
            PParamCategory::ByRef => (0, 0),      // parameter
            PParamCategory::Local => (1, 0),      // variable
            PParamCategory::Auto => return None,
        },
        HspSymbolKind::StaticVar => (1, 0b10), // static variable
        HspSymbolKind::Const | HspSymbolKind::Enum => (1, 0b01), // readonly variable
        HspSymbolKind::DefFunc
        | HspSymbolKind::DefCFunc
        | HspSymbolKind::ModFunc
        | HspSymbolKind::ModCFunc
        | HspSymbolKind::LibFunc
        | HspSymbolKind::ComFunc => (2, 0), // function
        HspSymbolKind::Macro { .. } => (3, 0), // macro
        HspSymbolKind::Module => (4, 0),       // namespace
        HspSymbolKind::PluginCmd => (5, 0),    // keyword

        // Not supported:
        // HspSymbolKind::ComInterface => ?,
        _ => return None,
    };
    Some((ty, modifiers))
}

pub(crate) fn full(
    uri: Url,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let doc = docs.find_by_uri(&CanonicalUri::from_url(&uri))?;
    let project = wa.require_project_for_doc(doc);

    let mut symbols = vec![];
    project.collect_symbol_occurrences(&mut symbols);

    let mut tokens: Vec<lsp_types::SemanticToken> = vec![];

    for (symbol, loc) in symbols {
        if loc.doc != doc {
            continue;
        }

        let (token_type, token_modifiers_bitset) = match to_semantic_token_kind(&symbol) {
            Some(it) => it,
            None => continue,
        };

        let location = loc_to_location(loc, docs)?;

        let Position {
            line: y1,
            character: x1,
        } = location.range.start;

        tokens.push(lsp_types::SemanticToken {
            delta_line: y1,
            delta_start: x1,
            length: symbol.name().encode_utf16().count() as u32,
            token_type,
            token_modifiers_bitset,
        });
    }

    // Compute delta.
    tokens.sort_by_key(|t| (t.delta_line, t.delta_start + t.length));

    for i in (1..tokens.len()).rev() {
        let y1 = tokens[i - 1].delta_line;
        let x1 = tokens[i - 1].delta_start;
        if tokens[i].delta_line == y1 {
            tokens[i].delta_line = 0;
            tokens[i].delta_start = tokens[i].delta_start.saturating_sub(x1);
        } else {
            tokens[i].delta_line -= y1;
        }
    }

    Some(tokens)
}
