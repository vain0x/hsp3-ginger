//! LSPの型からほかの形式への変換

use crate::source;
use lsp_types as lsp;

// pub(crate) fn doc(an: &Analyzer, uri: &lsp::Url) -> Option<source::DocId> {
//     let uri = CanonicalUri::from_url(uri);
//     DocDb::find_doc_by_uri(an, &uri)
// }

#[allow(unused)]
pub(crate) fn pos16(position: lsp::Position) -> source::Pos16 {
    let row = position.line as u32;
    let column = position.character as u32;
    source::Pos16::new(row, column)
}

// pub(crate) fn doc_pos(
//     an: &Analyzer,
//     url: &lsp::Url,
//     position: lsp::Position,
// ) -> Option<(source::DocId, source::Pos16)> {
//     Some((doc(an, url)?, pos16(position)))
// }
