use self::lang_service::RootDb;
use super::*;

pub(crate) fn references(
    uri: Url,
    position: Position,
    include_definition: bool,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    let project = wa.require_project_for_doc(doc);
    let (symbol, _) = project.locate_symbol(doc, pos)?;

    let mut locs = vec![];
    if include_definition {
        project.collect_symbol_defs(&symbol, &mut locs);
    }
    project.collect_symbol_uses(&symbol, &mut locs);

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(loc, docs))
            .collect(),
    )
}

pub(crate) fn references2(
    db: &RootDb,
    doc: DocId,
    pos: Pos16,
    include_definition: bool,
) -> Vec<Loc> {
    (|| {
        // let (doc, pos) = from_proto::doc_pos(db, &uri, position)?;
        let (symbol, _) = db.locate_symbol(doc, pos)?;

        let mut locs = vec![];
        if include_definition {
            db.collect_symbol_defs(&symbol, &mut locs);
        }
        db.collect_symbol_uses(&symbol, &mut locs);

        Some(locs)
    })()
    .unwrap_or_default()
}

pub(crate) mod from_proto {
    use crate::{assists::CanonicalUri, lang_service::RootDb, source};
    use lsp_types as lsp;

    pub(crate) fn doc(db: &RootDb, uri: &lsp::Url) -> Option<source::DocId> {
        let uri = CanonicalUri::from_url(uri);
        db.docs.find_by_uri(&uri)
    }

    pub(crate) fn pos16(position: lsp::Position) -> source::Pos16 {
        let row = position.line as u32;
        let column = position.character as u32;
        source::Pos16::new(row, column)
    }

    pub(crate) fn doc_pos(
        db: &RootDb,
        url: &lsp::Url,
        position: lsp::Position,
    ) -> Option<(source::DocId, source::Pos16)> {
        Some((doc(db, url)?, pos16(position)))
    }
}

pub(crate) mod to_proto {
    use crate::{
        lang_service::RootDb,
        source::{self, DocId},
    };
    use lsp_types as lsp;

    pub(crate) fn url(db: &RootDb, doc: DocId) -> Option<lsp::Url> {
        Some(db.docs.get_uri(doc)?.clone().into_url())
    }

    pub(crate) fn pos(pos: source::Pos) -> lsp::Position {
        lsp::Position::new(pos.row, pos.column16)
    }

    pub(crate) fn range(range: source::Range) -> lsp::Range {
        lsp::Range::new(pos(range.start()), pos(range.end()))
    }

    pub(crate) fn location(db: &RootDb, loc: source::Loc) -> Option<lsp::Location> {
        let url = url(db, loc.doc)?;
        let range = range(loc.range);
        Some(lsp::Location::new(url, range))
    }
}
