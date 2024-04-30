use super::*;
use crate::{ide::from_document_position, lang_service::docs::Docs};
use lsp_types::{DocumentHighlight, DocumentHighlightKind, Position, Url};

pub(crate) fn document_highlight(
    wa: &AnalysisRef<'_>,
    uri: Url,
    position: Position,
    docs: &Docs,
) -> Option<Vec<DocumentHighlight>> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    let (symbol, _) = wa.locate_symbol(doc, pos)?;

    let mut highlights = vec![];
    collect_highlights(wa, doc, &symbol, |kind, loc| {
        let kind = match kind {
            DefOrUse::Def => DocumentHighlightKind::WRITE,
            DefOrUse::Use => DocumentHighlightKind::READ,
        };

        highlights.push(DocumentHighlight {
            kind: Some(kind),
            range: loc_to_range(loc),
        });
    });

    Some(highlights)
}
