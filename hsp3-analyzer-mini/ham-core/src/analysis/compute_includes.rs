use self::analyzer::{doc_interner::DocInterner, docs};
use super::*;

pub(crate) fn compute_includes(
    doc_interner: &DocInterner,
    doc_analysis_map: &HashMap<DocId, DocAnalysis>,
    common_docs: &HashMap<String, DocId>,
    include_resolution: &mut Vec<(Loc, DocId)>,
) {
    let get_name = |doc: DocId| match doc_interner
        .get_uri(doc)
        .and_then(|uri| uri.to_file_path())
        .and_then(|path| path.file_name().map(|s| s.to_string_lossy().to_string()))
    {
        Some(name) => format!("{}:{}", doc, name),
        None => format!("{}", doc),
    };

    for (&src_doc, da) in doc_analysis_map {
        for (included_name, loc) in &da.includes {
            let included_doc_opt =
                docs::resolve_included_name(doc_interner, included_name, src_doc)
                    .or_else(|| common_docs.get(included_name.as_str()).cloned());

            debug!(
                "include(doc:{} {}:{}) {:?} -> {:?}",
                src_doc,
                get_name(src_doc),
                loc.start(),
                included_name,
                included_doc_opt
            );

            if let Some(included_doc) = included_doc_opt {
                include_resolution.push((*loc, included_doc));
            }
        }
    }
}
