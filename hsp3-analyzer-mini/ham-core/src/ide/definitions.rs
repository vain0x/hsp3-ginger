use super::*;
use crate::ide::from_document_position;
use lsp_types::{Location, Position, Url};

// (順不同、重複あり)
fn goto_symbol_definition(
    an: &AnalyzerRef<'_>,
    doc: DocId,
    pos: Pos16,
    locs: &mut Vec<Loc>,
) -> Option<()> {
    let (symbol, _) = an.locate_symbol(doc, pos)?;
    collect_symbol_occurrences(
        an,
        CollectSymbolOptions {
            include_def: true,
            include_use: false,
        },
        &symbol,
        locs,
    );
    Some(())
}

fn goto_include_target(
    an: &AnalyzerRef<'_>,
    doc: DocId,
    pos: Pos16,
    locs: &mut Vec<Loc>,
) -> Option<()> {
    let dest_doc = find_include_target(an, doc, pos)?;
    locs.push(Loc::from_doc(dest_doc));
    Some(())
}

pub(crate) fn definitions(
    an: &AnalyzerRef<'_>,
    doc_interner: &DocInterner,
    uri: Url,
    position: Position,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(doc_interner, &uri, position)?;
    let mut locs = vec![];

    let ok = goto_symbol_definition(an, doc, pos, &mut locs).is_some()
        || goto_include_target(an, doc, pos, &mut locs).is_some();
    if !ok {
        debug_assert_eq!(locs.len(), 0);
        return None;
    }

    // ソートして重複を取り除く
    locs.sort();
    locs.dedup();

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(doc_interner, loc))
            .collect(),
    )
}
