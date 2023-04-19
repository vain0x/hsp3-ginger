use super::*;
use crate::{assists::from_document_position, lang_service::docs::Docs};
use lsp_types::{DocumentHighlight, DocumentHighlightKind, Position, Url};

pub(crate) fn document_highlight(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<Vec<DocumentHighlight>> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    let project = wa.require_project_for_doc(doc);
    let (symbol, _) = project.locate_symbol(doc, pos)?;

    let mut locs = vec![];
    let mut highlights = vec![];

    project.collect_symbol_defs(&symbol, &mut locs);
    highlights.extend(
        locs.drain(..)
            .map(|loc| (DocumentHighlightKind::WRITE, loc)),
    );

    project.collect_symbol_uses(&symbol, &mut locs);
    highlights.extend(locs.drain(..).map(|loc| (DocumentHighlightKind::READ, loc)));

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
