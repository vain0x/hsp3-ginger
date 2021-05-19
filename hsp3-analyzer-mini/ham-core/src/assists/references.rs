use super::*;

pub(crate) fn references(
    uri: Url,
    position: Position,
    include_definition: bool,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    let project = wa.require_project_for_doc(doc);
    let (symbol, _) = project.locate_symbol(doc, pos)?;

    let mut locs = vec![];
    if include_definition {
        project.collect_symbol_defs(&symbol, &mut locs);
    }
    project.collect_symbol_uses(&symbol, &mut locs);

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(loc, docs))
            .collect(),
    )
}
