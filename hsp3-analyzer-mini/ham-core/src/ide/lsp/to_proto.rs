//! LSPの型への変換

use crate::{
    lang_service::{DocDb, LangService},
    source::{self, DocId},
};
use lsp_types as lsp;

pub(crate) fn url(ls: &LangService, doc: DocId) -> Option<lsp::Url> {
    Some(DocDb::get_doc_uri(ls, doc)?.clone().into_url())
}

pub(crate) fn pos(pos: source::Pos) -> lsp::Position {
    lsp::Position::new(pos.row, pos.column16)
}

pub(crate) fn range(range: source::Range) -> lsp::Range {
    lsp::Range::new(pos(range.start()), pos(range.end()))
}

#[allow(unused)]
pub(crate) fn location(ls: &LangService, loc: source::Loc) -> Option<lsp::Location> {
    let url = url(ls, loc.doc)?;
    let range = range(loc.range);
    Some(lsp::Location::new(url, range))
}
