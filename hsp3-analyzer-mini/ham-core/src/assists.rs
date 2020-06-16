use crate::{
    analysis::{ALoc, APos},
    lang_service::docs::Docs,
    utils::canonical_uri::CanonicalUri,
};
use lsp_types::{LanguageString, Location, MarkedString, Position, Range, Url};

pub(crate) mod completion;
pub(crate) mod definitions;
pub(crate) mod document_highlight;
pub(crate) mod hover;
pub(crate) mod references;
pub(crate) mod rename;

fn plain_text_to_marked_string(text: String) -> MarkedString {
    const PLAIN_LANG_ID: &str = "plaintext";

    MarkedString::LanguageString(LanguageString {
        language: PLAIN_LANG_ID.to_string(),
        value: text,
    })
}

fn loc_to_range(loc: ALoc) -> Range {
    // FIXME: UTF-8 から UTF-16 基準のインデックスへの変換
    Range::new(
        Position::new(loc.start_row() as u64, loc.start_column() as u64),
        Position::new(loc.end_row() as u64, loc.end_column() as u64),
    )
}

fn loc_to_location(loc: ALoc, docs: &Docs) -> Option<Location> {
    let uri = docs.get_uri(loc.doc)?.clone().into_url();
    let range = loc_to_range(loc);
    Some(Location { uri, range })
}

fn to_loc(uri: &Url, position: Position, docs: &Docs) -> Option<ALoc> {
    let uri = CanonicalUri::from_url(uri);
    let doc = docs.find_by_uri(&uri)?;

    // FIXME: position は UTF-16 ベース、pos は UTF-8 ベースなので、マルチバイト文字が含まれている場合は変換が必要
    let pos = APos {
        row: position.line as usize,
        column: position.character as usize,
    };

    Some(ALoc::new3(doc, pos, pos))
}
