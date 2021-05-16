pub(crate) mod completion;
pub(crate) mod definitions;
pub(crate) mod diagnose;
pub(crate) mod document_highlight;
pub(crate) mod formatting;
pub(crate) mod hover;
pub(crate) mod references;
pub(crate) mod rename;
pub(crate) mod signature_help;

pub(crate) mod rewrites {
    use super::*;

    pub(crate) mod flip_comma;
}

use super::*;
use crate::{lang_service::docs::Docs, source::*, token::TokenKind};
use lsp_types::{LanguageString, Location, MarkedString, Position, Url};

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

fn to_position(pos: Pos) -> Position {
    Position::new(pos.row as u64, pos.column16 as u64)
}

fn to_lsp_range(range: crate::source::Range) -> lsp_types::Range {
    lsp_types::Range::new(to_position(range.start()), to_position(range.end()))
}

fn loc_to_range(loc: Loc) -> lsp_types::Range {
    to_lsp_range(loc.range)
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
