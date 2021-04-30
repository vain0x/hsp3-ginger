use crate::{
    lang_service::docs::Docs,
    source::{Loc, Pos},
    utils::canonical_uri::CanonicalUri,
};
use lsp_types::{LanguageString, Location, MarkedString, Position, Range, Url};

pub(crate) mod completion;
pub(crate) mod definitions;
pub(crate) mod diagnose;
pub(crate) mod document_highlight;
pub(crate) mod hover;
pub(crate) mod references;
pub(crate) mod rename;

fn plain_text_to_marked_string(value: String) -> MarkedString {
    MarkedString::LanguageString(LanguageString {
        language: "plaintext".to_string(),
        value,
    })
}

fn markdown_marked_string(value: String) -> MarkedString {
    MarkedString::LanguageString(LanguageString {
        language: "markdown".to_string(),
        value,
    })
}

fn loc_to_range(loc: Loc) -> Range {
    // FIXME: UTF-8 から UTF-16 基準のインデックスへの変換
    Range::new(
        Position::new(loc.start_row() as u64, loc.start_column() as u64),
        Position::new(loc.end_row() as u64, loc.end_column() as u64),
    )
}

fn loc_to_location(loc: Loc, docs: &Docs) -> Option<Location> {
    let uri = docs.get_uri(loc.doc)?.clone().into_url();
    let range = loc_to_range(loc);
    Some(Location { uri, range })
}

fn to_loc(uri: &Url, position: Position, docs: &Docs) -> Option<Loc> {
    let uri = CanonicalUri::from_url(uri);
    let doc = docs.find_by_uri(&uri)?;

    // FIXME: position は UTF-16 ベース、pos は UTF-8 ベースなので、マルチバイト文字が含まれている場合は変換が必要
    let pos = Pos {
        row: position.line as usize,
        column: position.character as usize,
    };

    Some(Loc::new3(doc, pos, pos))
}
