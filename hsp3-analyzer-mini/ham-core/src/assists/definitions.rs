use super::*;
use crate::{assists::from_document_position, lang_service::docs::Docs};
use lsp_types::{Location, Position, Url};

fn goto_symbol_definition(
    wa: &AnalysisRef<'_>,
    doc: DocId,
    pos: Pos16,
    locs: &mut Vec<Loc>,
) -> Option<()> {
    let project = wa.require_project_for_doc(doc);
    let (symbol, _) = project.locate_symbol(doc, pos)?;
    project.collect_symbol_defs(&symbol, locs);
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
    uri: Url,
    position: Position,
    docs: &Docs,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    let mut locs = vec![];

    let ok = goto_symbol_definition(wa, doc, pos, &mut locs).is_some()
        || goto_include_target(wa, doc, pos, &mut locs).is_some();
    if !ok {
        debug_assert_eq!(locs.len(), 0);
        return None;
    }

    Some(
        locs.into_iter()
            .filter_map(|loc| loc_to_location(loc, docs))
            .collect(),
    )
}
