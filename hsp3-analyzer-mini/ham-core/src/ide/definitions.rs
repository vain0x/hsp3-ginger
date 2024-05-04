use super::*;
use crate::ide::from_document_position;
use lsp_types::{Location, Position, Url};

fn goto_symbol_definition(
    wa: &AnalysisRef<'_>,
    doc_interner: &DocInterner,
    doc: DocId,
    pos: Pos16,
    locs: &mut Vec<Loc>,
) -> Option<()> {
    let (symbol, _) = wa.locate_symbol(doc, pos)?;
    let include_graph = IncludeGraph::generate(wa, doc_interner);
    collect_symbol_defs(wa, &include_graph, doc, &symbol, locs);
    Some(())
}

fn goto_include_target(
    wa: &AnalysisRef<'_>,
    doc: DocId,
    pos: Pos16,
    locs: &mut Vec<Loc>,
) -> Option<()> {
    let dest_doc = find_include_target(wa, doc, pos)?;
    locs.push(Loc::from_doc(dest_doc));
    Some(())
}

pub(crate) fn definitions(
    wa: &AnalysisRef<'_>,
    doc_interner: &DocInterner,
    uri: Url,
    position: Position,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(doc_interner, &uri, position)?;
    let mut locs = vec![];

    let ok = goto_symbol_definition(wa, doc_interner, doc, pos, &mut locs).is_some()
        || goto_include_target(wa, doc, pos, &mut locs).is_some();
    if !ok {
        debug_assert_eq!(locs.len(), 0);
        return None;
    }

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(doc_interner, loc))
            .collect(),
    )
}
