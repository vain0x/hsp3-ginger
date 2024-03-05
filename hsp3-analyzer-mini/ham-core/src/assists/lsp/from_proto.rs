//! LSPの型からほかの形式への変換

use crate::{
    assists::CanonicalUri,
    lang_service::{DocDb, LangService},
    source,
};
use lsp_types as lsp;

pub(crate) fn doc(ls: &LangService, uri: &lsp::Url) -> Option<source::DocId> {
    let uri = CanonicalUri::from_url(uri);
    DocDb::find_doc_by_uri(ls, &uri)
}

pub(crate) fn pos16(position: lsp::Position) -> source::Pos16 {
    let row = position.line as u32;
    let column = position.character as u32;
    source::Pos16::new(row, column)
}

#[allow(unused)]
pub(crate) fn doc_pos(
    ls: &LangService,
    url: &lsp::Url,
    position: lsp::Position,
) -> Option<(source::DocId, source::Pos16)> {
    Some((doc(ls, url)?, pos16(position)))
}
