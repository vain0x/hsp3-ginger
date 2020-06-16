use super::{loc_to_range, to_loc};
use crate::{lang_service::docs::Docs, sem::ProjectSem};
use lsp_types::{DocumentHighlight, DocumentHighlightKind, Position, Url};

pub(crate) fn document_highlight(
    uri: Url,
    position: Position,
    docs: &Docs,
    sem: &mut ProjectSem,
) -> Option<Vec<DocumentHighlight>> {
    let loc = to_loc(&uri, position, docs)?;
    let doc = loc.doc;
    let (symbol, _) = sem.locate_symbol(loc.doc, loc.start())?;
    let symbol_id = symbol.symbol_id;

    let mut locs = vec![];
    let mut highlights = vec![];

    sem.get_symbol_defs(symbol_id, &mut locs);
    highlights.extend(
        locs.drain(..)
            .map(|loc| (DocumentHighlightKind::Write, loc)),
    );

    sem.get_symbol_uses(symbol_id, &mut locs);
    highlights.extend(locs.drain(..).map(|loc| (DocumentHighlightKind::Read, loc)));

    highlights.retain(|(_, loc)| loc.doc == doc);

    Some(
        highlights
            .into_iter()
            .map(|(kind, loc)| DocumentHighlight {
                kind: Some(kind),
                range: loc_to_range(loc),
            })
            .collect(),
    )
}
