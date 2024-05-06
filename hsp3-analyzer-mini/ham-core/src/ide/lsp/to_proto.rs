//! LSPの型への変換

use crate::source;
use lsp_types as lsp;

// pub(crate) fn url(an: &Analyzer, doc: DocId) -> Option<lsp::Url> {
//     Some(DocDb::get_doc_uri(an, doc)?.clone().into_url())
// }

#[allow(unused)]
pub(crate) fn pos(pos: source::Pos) -> lsp::Position {
    lsp::Position::new(pos.row, pos.column16)
}

#[allow(unused)]
pub(crate) fn range(range: source::Range) -> lsp::Range {
    lsp::Range::new(pos(range.start()), pos(range.end()))
}

// pub(crate) fn location(an: &Analyzer, loc: source::Loc) -> Option<lsp::Location> {
//     let url = url(an, loc.doc)?;
//     let range = range(loc.range);
//     Some(lsp::Location::new(url, range))
// }
