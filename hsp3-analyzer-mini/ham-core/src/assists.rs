use crate::{
    lang_service::docs::Docs,
    source::{DocId, Loc, Pos16},
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
    info!(
        "loc_to_range: loc={},{}..{} -> {}:{}..{}:{}",
        loc.doc,
        loc.start(),
        loc.end(),
        loc.start_row(),
        loc.start().column16,
        loc.end_row(),
        loc.end().column16
    );

    Range::new(
        Position::new(loc.start_row() as u64, loc.start().column16 as u64),
        Position::new(loc.end_row() as u64, loc.end().column16 as u64),
    )
}

fn loc_to_location(loc: Loc, docs: &Docs) -> Option<Location> {
    let uri = docs.get_uri(loc.doc)?.clone().into_url();
    let range = loc_to_range(loc);
    Some(Location { uri, range })
}

fn from_document_position(uri: &Url, position: Position, docs: &Docs) -> Option<(DocId, Pos16)> {
    let uri = CanonicalUri::from_url(uri);
    let doc = docs.find_by_uri(&uri)?;

    let pos = {
        let row = position.line as u32;
        let column = position.character as u32;
        Pos16::new(row, column)
    };

    Some((doc, pos))
}
