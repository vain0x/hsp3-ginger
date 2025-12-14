//! ドキュメントハイライト

use super::*;
use crate::ide::from_document_position;
use lsp_types::{DocumentHighlight, DocumentHighlightKind, Position, Url};

pub(crate) fn document_highlight(
    an: &AnalyzerRef<'_>,
    doc_interner: &DocInterner,
    uri: Url,
    position: Position,
) -> Option<Vec<DocumentHighlight>> {
    let (doc, pos) = from_document_position(doc_interner, &uri, position)?;
    let (symbol, _) = an.locate_symbol(doc, pos)?;

    let mut highlights = vec![];
    collect_highlights(an, doc, &symbol, |kind, loc| {
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
