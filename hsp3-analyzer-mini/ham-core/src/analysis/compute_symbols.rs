use self::workspace_analysis::DocAnalysisMap;
use super::*;

pub(crate) fn compute_symbols(
    hsphelp_info: &HspHelpInfo,
    active_docs: &HashSet<DocId>,
    help_docs: &HashMap<DocId, DocId>,
    doc_analysis_map: &DocAnalysisMap,
    module_map: &ModuleMap,
    public_env: &mut PublicEnv,
    ns_env: &mut HashMap<RcStr, SymbolEnv>,
    doc_symbols_map: &mut HashMap<DocId, Vec<SymbolRc>>,
    def_sites: &mut Vec<(SymbolRc, Loc)>,
    use_sites: &mut Vec<(SymbolRc, Loc)>,
) {
    // 複数ファイルに渡る環境を構築する。
    for (&doc, da) in doc_analysis_map.iter() {
        if !active_docs.contains(&doc) {
            continue;
        }

        extend_public_env_from_symbols(&da.preproc_symbols, public_env, ns_env);
    }

    // 変数の定義箇所を決定する。
    doc_symbols_map.extend(
        doc_analysis_map
            .iter()
            .filter(|(&doc, _)| active_docs.contains(&doc))
            .map(|(&doc, da)| (doc, da.preproc_symbols.clone())),
    );

    for (&doc, da) in doc_analysis_map.iter() {
        if !active_docs.contains(&doc) {
            continue;
        }

        let symbols = doc_symbols_map.get_mut(&doc).unwrap();

        def_sites.extend(symbols.iter().filter_map(|symbol| {
            let loc = symbol.preproc_def_site_opt?;
            Some((symbol.clone(), loc))
        }));

        crate::analysis::var::analyze_var_def(
            doc,
            da.tree_opt.as_ref().unwrap(),
            &module_map,
            symbols,
            public_env,
            ns_env,
            def_sites,
            use_sites,
        );

        // ヘルプファイルの情報をシンボルに統合する。
        if let Some(hs_doc) = help_docs.get(&doc) {
            if let Some(hs_symbols) = hsphelp_info.doc_symbols.get(&hs_doc) {
                let mut hs_symbols_map = hs_symbols
                    .iter()
                    .map(|s| (s.label.as_str(), s.clone()))
                    .collect::<HashMap<_, _>>();

                for symbol in symbols {
                    let mut link_opt = symbol.linked_symbol_opt.borrow_mut();
                    if link_opt.is_some() {
                        continue;
                    }

                    let s = match hs_symbols_map.remove(symbol.name.as_str()) {
                        Some(it) => it,
                        None => continue,
                    };

                    *link_opt = Some(s.clone());
                }
            }
        }
    }
}
