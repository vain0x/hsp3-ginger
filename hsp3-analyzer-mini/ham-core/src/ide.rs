pub(crate) mod completion;
pub(crate) mod definitions;
pub(crate) mod diagnose;
pub(crate) mod document_highlight;
pub(crate) mod document_symbol;
pub(crate) mod formatting;
pub(crate) mod hover;
pub(crate) mod references;
pub(crate) mod rename;
pub(crate) mod semantic_tokens;
pub(crate) mod signature_help;
pub(crate) mod workspace_symbol;

pub(crate) mod lsp {
    pub(crate) mod from_proto;
    pub(crate) mod to_proto;
}

pub(crate) mod code_actions {
    use super::*;

    pub(crate) mod flip_comma;
    pub(crate) mod generate_include_guard;
}

use super::*;
use crate::{
    analysis::*,
    lang_service::{doc_interner::DocInterner, docs::Docs},
    source::*,
    token::TokenKind,
};
use lsp_types::{LanguageString, Location, MarkedString, Position, SymbolInformation, Url};

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
    Position::new(pos.row as u32, pos.column16 as u32)
}

fn to_lsp_range(range: crate::source::Range) -> lsp_types::Range {
    lsp_types::Range::new(to_position(range.start()), to_position(range.end()))
}

fn loc_to_range(loc: Loc) -> lsp_types::Range {
    to_lsp_range(loc.range)
}

fn loc_to_location(doc_interner: &DocInterner, loc: Loc) -> Option<Location> {
    let uri = doc_interner.get_uri(loc.doc)?.clone().into_url();
    let range = loc_to_range(loc);
    Some(Location { uri, range })
}

fn from_document_position(
    doc_interner: &DocInterner,
    uri: &Url,
    position: Position,
) -> Option<(DocId, Pos16)> {
    let uri = CanonicalUri::from_url(uri);
    let doc = doc_interner.get_doc(&uri)?;

    let pos = {
        let row = position.line as u32;
        let column = position.character as u32;
        Pos16::new(row, column)
    };

    Some((doc, pos))
}

fn new_lsp_symbol_information(
    name: String,
    kind: lsp_types::SymbolKind,
    location: Location,
) -> SymbolInformation {
    #[allow(deprecated)]
    SymbolInformation {
        name,
        kind,
        location,
        tags: None,
        deprecated: None,
        container_name: None,
    }
}
