use super::*;
use crate::analysis::*;

pub(crate) fn full(
    uri: Url,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let doc = docs.find_by_uri(&CanonicalUri::from_url(&uri))?;

    let mut symbols = vec![];
    wa.require_project_for_doc(doc)
        .collect_doc_symbols(doc, &mut symbols);

    symbols.sort_by_key(|s| s.1.start());

    let mut tokens: Vec<lsp_types::SemanticToken> = symbols
        .into_iter()
        .filter_map(|(symbol, loc)| {
            let token_ty = match symbol.kind {
                HspSymbolKind::Param(_) => 0,
                HspSymbolKind::StaticVar => 1,
                _ => return None,
            };
            let location = loc_to_location(loc, docs)?;

            let Position {
                line: y1,
                character: x1,
            } = location.range.start;
            let Position {
                line: y2,
                character: x2,
            } = location.range.end;
            if y1 < y2 {
                return None;
            }

            Some(lsp_types::SemanticToken {
                delta_line: y1,
                delta_start: x1,
                length: x2 - x1,
                token_type: 0,
                token_modifiers_bitset: 0,
            })
        })
        .collect();

    for i in (1..tokens.len()).rev() {
        let y1 = tokens[i - 1].delta_line;
        let x1 = tokens[i - 1].delta_start + tokens[i - 1].length;
        if tokens[i].delta_line == y1 {
            tokens[i].delta_line = 0;
        } else {
            tokens[i].delta_line -= y1;
            tokens[i].delta_start -= x1;
        }
    }
    Some(tokens)
}
