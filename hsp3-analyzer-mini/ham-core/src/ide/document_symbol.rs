use super::*;
use crate::{analysis::*, parse::p_param_ty::PParamCategory};
use lsp_types::DocumentSymbolResponse;

// completion, workspace/symbol も参照
fn to_lsp_symbol_kind(kind: HspSymbolKind) -> Option<lsp_types::SymbolKind> {
    use lsp_types::SymbolKind as K;
    let it = match kind {
        HspSymbolKind::Unresolved => return None,
        HspSymbolKind::Module => K::MODULE,
        HspSymbolKind::StaticVar => K::VARIABLE,
        HspSymbolKind::Unknown
        | HspSymbolKind::Label
        | HspSymbolKind::Const
        | HspSymbolKind::Enum
        | HspSymbolKind::Macro { ctype: false }
        | HspSymbolKind::Param(None)
        | HspSymbolKind::PluginCmd => K::CONSTANT,
        HspSymbolKind::Macro { ctype: true }
        | HspSymbolKind::DefFunc
        | HspSymbolKind::DefCFunc
        | HspSymbolKind::LibFunc => K::FUNCTION,
        HspSymbolKind::ModFunc | HspSymbolKind::ModCFunc | HspSymbolKind::ComFunc => K::METHOD,
        HspSymbolKind::Param(Some(param)) => match param.category() {
            PParamCategory::ByValue => K::CONSTANT,
            PParamCategory::ByRef => K::PROPERTY,
            PParamCategory::Local => K::VARIABLE,
            PParamCategory::Auto => return None,
        },
        HspSymbolKind::Field => K::FIELD,
        HspSymbolKind::ComInterface => K::INTERFACE,
    };
    Some(it)
}

pub(crate) fn symbol(
    wa: &AnalysisRef<'_>,
    doc_interner: &DocInterner,
    uri: Url,
) -> Option<DocumentSymbolResponse> {
    let doc = doc_interner.get_doc(&CanonicalUri::from_url(&uri))?;

    let mut symbols = vec![];
    collect_doc_symbols(wa, doc, &mut symbols);

    // 空のシンボルを除去する (名前が空のシンボルがどこかで登録されている(?))
    symbols.retain(|(s, _)| !s.name().is_empty());

    symbols.sort_by_key(|(_, loc)| loc.start());

    let symbol_information_list = symbols
        .into_iter()
        .filter_map(|(symbol, loc)| {
            let name = symbol.name();
            let kind = to_lsp_symbol_kind(symbol.kind)?;
            let location = loc_to_location(doc_interner, loc)?;

            Some(new_lsp_symbol_information(name.to_string(), kind, location))
        })
        .collect();

    Some(DocumentSymbolResponse::Flat(symbol_information_list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lang_service::LangService, lsp_server::NO_VERSION};
    use std::fmt::Write as _;

    fn dummy_url(s: &str) -> Url {
        let workspace_dir = crate::test_utils::dummy_path().join("ws").join(s);
        Url::from_file_path(&workspace_dir.join(s)).unwrap()
    }

    fn format_symbol(w: &mut String, symbol: &SymbolInformation) {
        let location = &symbol.location;
        let start = location.range.start;
        write!(
            w,
            "{}:{}:{} {} {:?}",
            location.uri.path_segments().unwrap().last().unwrap(),
            start.line + 1,
            start.character + 1,
            symbol.name,
            symbol.kind
        )
        .unwrap();
    }

    fn format_response(w: &mut String, res: &DocumentSymbolResponse) {
        match res {
            DocumentSymbolResponse::Flat(symbols) => {
                for symbol in symbols {
                    format_symbol(w, symbol);
                    *w += "\n"
                }
            }
            DocumentSymbolResponse::Nested(_) => panic!("no use"),
        }
    }

    #[test]
    fn test() {
        let mut ls = LangService::new_standalone();

        let main_uri = dummy_url("main.hsp");
        ls.open_doc(
            main_uri.clone(),
            NO_VERSION,
            r#"
#module m1
#deffunc f int a, str b
    return
#global

    goto *my_label

*my_label
    s1 = 0
    f 1, 2
    return
            "#
            .into(),
        );
        let res = ls.compute_ref().document_symbol(main_uri).unwrap();
        let mut formatted = String::new();
        format_response(&mut formatted, &res);
        // FIXME: my_labelが重複している
        debug_assert_eq!(formatted, "main.hsp:2:9 m1 Module\nmain.hsp:3:10 f Function\nmain.hsp:3:16 a Constant\nmain.hsp:3:23 b Constant\nmain.hsp:9:2 my_label Constant\nmain.hsp:9:2 my_label Constant\nmain.hsp:10:5 s1 Variable\n");
    }
}
