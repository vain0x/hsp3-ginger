use super::{loc_to_range, to_loc};
use crate::{analysis::integrate::AWorkspaceAnalysis, lang_service::docs::Docs, sem::ProjectSem};
use lsp_types::{DocumentHighlight, DocumentHighlightKind, Position, Url};

// FIXME: ファイルウォッチャーを所有する Docs ではなく URI と Doc のマッピングだけを渡してほしい
pub(crate) fn document_highlight(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Option<Vec<DocumentHighlight>> {
    let loc = to_loc(&uri, position, docs)?;
    let doc = loc.doc;

    let (ws_symbol, _) = wa.locate_symbol(doc, loc.start())?;

    let mut locs = vec![];
    let mut highlights = vec![];

    // sem.get_symbol_defs(symbol_id, &mut locs);
    // highlights.extend(
    //     locs.drain(..)
    //         .map(|loc| (DocumentHighlightKind::Write, loc)),
    // );

    wa.get_symbol_uses(ws_symbol, &mut locs);

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
