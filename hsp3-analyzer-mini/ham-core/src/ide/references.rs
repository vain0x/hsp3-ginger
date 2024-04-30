use super::*;

pub(crate) fn references(
    wa: &AnalysisRef<'_>,
    docs: &Docs,
    uri: Url,
    position: Position,
    include_definition: bool,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    let (symbol, _) = wa.locate_symbol(doc, pos)?;

    let include_graph = IncludeGraph::generate(wa, docs);
    let mut locs = vec![];
    if include_definition {
        collect_symbol_defs(wa, &include_graph, doc, &symbol, &mut locs);
    }
    collect_symbol_uses(wa, &include_graph, doc, &symbol, &mut locs);

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(loc, docs))
            .collect(),
    )
}
