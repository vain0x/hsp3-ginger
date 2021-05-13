use super::loc_to_location;
use crate::{
    analysis::integrate::AWorkspaceAnalysis, assists::from_document_position,
    lang_service::docs::Docs,
};
use lsp_types::{Location, Position, Url};

pub(crate) fn references(
    uri: Url,
    position: Position,
    include_definition: bool,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    let (ws_symbol, _) = wa.locate_symbol(doc, pos)?;

    let mut locs = vec![];
    if include_definition {
        wa.collect_symbol_defs(&ws_symbol, &mut locs);
    }
    wa.collect_symbol_uses(&ws_symbol, &mut locs);

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(loc, docs))
            .collect(),
    )
}
