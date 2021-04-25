use super::loc_to_location;
use crate::{analysis::integrate::AWorkspaceAnalysis, assists::to_loc, lang_service::docs::Docs};
use lsp_types::{Location, Position, Url};

pub(crate) fn definitions(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Option<Vec<Location>> {
    let loc = to_loc(&uri, position, docs)?;
    let (ws_symbol, _) = wa.locate_symbol(loc.doc, loc.start())?;

    let mut locs = vec![];
    wa.collect_symbol_defs(ws_symbol, &mut locs);

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(loc, docs))
            .collect(),
    )
}
