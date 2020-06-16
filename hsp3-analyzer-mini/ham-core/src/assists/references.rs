use super::{loc_to_location, to_loc};
use crate::{lang_service::docs::Docs, sem::ProjectSem};
use lsp_types::{Location, Position, Url};

pub(crate) fn references(
    uri: Url,
    position: Position,
    include_definition: bool,
    docs: &Docs,
    sem: &mut ProjectSem,
) -> Option<Vec<Location>> {
    let loc = to_loc(&uri, position, docs)?;
    let (symbol, _) = sem.locate_symbol(loc.doc, loc.start())?;
    let symbol_id = symbol.symbol_id;

    let mut locs = vec![];

    if include_definition {
        sem.get_symbol_defs(symbol_id, &mut locs);
    }
    sem.get_symbol_uses(symbol_id, &mut locs);

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(loc, docs))
            .collect(),
    )
}
