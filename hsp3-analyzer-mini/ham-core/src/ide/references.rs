use super::*;

pub(crate) fn references(
    wa: &AnalysisRef<'_>,
    doc_interner: &DocInterner,
    uri: Url,
    position: Position,
    include_definition: bool,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(doc_interner, &uri, position)?;
    let (symbol, _) = wa.locate_symbol(doc, pos)?;

    let mut locs = vec![];
    collect_symbol_occurrences(
        wa,
        CollectSymbolOptions {
            include_def: include_definition,
            include_use: true,
        },
        &symbol,
        &mut locs,
    );

    // ソートして重複を取り除く
    locs.sort();
    locs.dedup();

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(doc_interner, loc))
            .collect(),
    )
}
