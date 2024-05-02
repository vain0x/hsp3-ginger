use super::*;

/// アクティブドキュメントを計算する
///
/// 解析対象となるドキュメントを **アクティブドキュメント** と呼ぶ。
/// (アクティブでないドキュメントは使われていないから解析しても無駄なので省くということ。)
/// (common以外にある少なくとも1つの) スクリプトファイルから `include` されているファイルをアクティブドキュメントとみなす。
/// ヘルプファイルは、それとリンクしているモジュールがアクティブである場合にアクティブとみなす。
/// (`hsphelp` と `common` にある同じ名前のファイルをリンクしているとみなす。
///  `hsphelp/foo.hs` は `common/foo.as` がアクティブである場合にアクティブとみなされる。)
pub(crate) fn compute_active_docs(
    doc_analysis_map: &HashMap<DocId, DocAnalysis>,
    common_docs: &HashMap<String, DocId>,
    hsphelp_info: &HspHelpInfo,
    active_docs: &mut HashSet<DocId>,
    active_help_docs: &mut HashSet<DocId>,
    help_docs: &mut HashMap<DocId, DocId>,
    #[allow(unused)] include_resolution: &mut Vec<(Loc, DocId)>,
) {
    // common以外にあるすべてのファイルと、
    // それらのファイルからincludeされているcommonのファイルはアクティブとする
    {
        let mut included_docs = HashSet::new();
        let in_common = common_docs.values().cloned().collect::<HashSet<_>>();

        for (&doc, da) in doc_analysis_map.iter() {
            if in_common.contains(&doc) {
                continue;
            }

            for (include, _) in &da.includes {
                let doc_opt = common_docs.get(include.as_str()).cloned();
                included_docs.extend(doc_opt);
            }
        }

        active_docs.extend(
            doc_analysis_map
                .keys()
                .cloned()
                .filter(|doc| !in_common.contains(&doc) || included_docs.contains(&doc)),
        );
    }

    // hsphelp
    {
        trace!("active_help_docs.len={}", active_help_docs.len());
        active_help_docs.extend(hsphelp_info.builtin_docs.iter().cloned());

        for (&common_doc, &hs_doc) in &hsphelp_info.linked_docs {
            if active_docs.contains(&common_doc) {
                active_help_docs.insert(hs_doc);
                help_docs.insert(common_doc, hs_doc);
            }
        }
    }
}
