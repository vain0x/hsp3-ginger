use super::*;
use crate::{assists::from_document_position, lang_service::docs::Docs};
use lsp_types::{Location, Position, Url};

pub(crate) fn definitions(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    let project = wa.require_project_for_doc(doc);
    let (symbol, _) = project.locate_symbol(doc, pos)?;

    let mut locs = vec![];
    project.collect_symbol_defs(&symbol, &mut locs);

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(loc, docs))
            .collect(),
    )
}
