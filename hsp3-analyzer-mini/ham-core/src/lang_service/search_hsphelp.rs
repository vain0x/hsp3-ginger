use super::*;

pub(crate) fn search_hsphelp(
    hsp3_home: &Path,
    docs: &mut Docs,
    builtin_env: &mut SymbolEnv,
    hsphelp_symbols: &mut Vec<CompletionItem>,
) {
    // info!("hsphelpディレクトリにあるhsファイルを開きます。");

    let mut file_count = 0;
    let mut symbols = vec![];
    let mut warnings = vec![];
    collect_all_symbols(hsp3_home, &mut file_count, &mut symbols, &mut warnings)
        .map_err(|e| warn!("{}", e))
        .ok();
    for w in warnings {
        warn!("{}", w);
    }

    let hsphelp_doc = docs.fresh_doc();

    hsphelp_symbols.extend(symbols.into_iter().map(|symbol| {
        let kind = CompletionItemKind::Function;
        let HsSymbol {
            name,
            description,
            documentation,
            params_opt,
            builtin,
        } = symbol;

        let name_rc = RcStr::from(name.clone());

        let signature_opt = params_opt.map(|params| {
            let params = params
                .into_iter()
                .map(|p| (None, Some(p.name.into()), p.details_opt))
                .collect();

            Rc::new(ASignatureData {
                name: name_rc.clone(),
                params,
            })
        });

        let symbol = SymbolRc::from(ASymbolData {
            doc: hsphelp_doc,
            kind: HspSymbolKind::Unknown,
            name: name_rc.clone(),
            leader_opt: None,
            scope_opt: None,
            ns_opt: None,
            details_opt: Some(ASymbolDetails {
                desc: description.clone().map(RcStr::from),
                docs: documentation.clone(),
            }),
            preproc_def_site_opt: None,
            signature_opt: RefCell::new(signature_opt),
        });
        builtin_env.insert(name_rc.clone(), symbol);

        // 補完候補の順番を制御するための文字。(標準命令を上に出す。)
        let sort_prefix = if builtin { 'x' } else { 'y' };

        // '#' なし
        let word = if name.as_str().starts_with("#") {
            Some(name.as_str().chars().skip(1).collect::<String>())
        } else {
            None
        };

        CompletionItem {
            kind: Some(kind),
            label: name,
            detail: description,
            documentation: if documentation.is_empty() {
                None
            } else {
                Some(Documentation::String(documentation.join("\r\n\r\n")))
            },
            sort_text: Some(format!("{}{}", sort_prefix, name_rc)),
            filter_text: word.clone(),
            insert_text: word,
            ..Default::default()
        }
    }));
}
