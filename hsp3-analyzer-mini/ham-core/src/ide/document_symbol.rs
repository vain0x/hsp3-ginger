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
    an: &AnalyzerRef<'_>,
    doc_interner: &DocInterner,
    uri: Url,
) -> Option<DocumentSymbolResponse> {
    let doc = doc_interner.get_doc(&CanonicalUri::from_url(&uri))?;

    let mut symbols = vec![];
    collect_doc_symbols(an, doc, &mut symbols);

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
    use crate::{analyzer::Analyzer, lsp_server::NO_VERSION, test_utils::set_test_logger};
    use expect_test::expect;
    use std::fmt::Write as _;

    fn dummy_url(s: &str) -> Url {
        let workspace_dir = crate::test_utils::dummy_path().join("ws");
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
        let mut an = Analyzer::new_standalone();

        let main_uri = dummy_url("main.hsp");
        an.open_doc(
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
    goto *my_label
    return
            "#
            .into(),
        );
        let res = an.compute_ref().document_symbol(main_uri).unwrap();
        let mut formatted = String::new();
        format_response(&mut formatted, &res);

        expect![[r#"
            main.hsp:2:9 m1 Module
            main.hsp:3:10 f Function
            main.hsp:3:16 a Constant
            main.hsp:3:23 b Constant
            main.hsp:9:1 *my_label Constant
            main.hsp:10:5 s1 Variable
        "#]]
        .assert_eq(&formatted);
    }

    // #var 系命令
    #[test]
    fn test_var() {
        set_test_logger();
        let mut an = Analyzer::new_standalone();

        let main_url = dummy_url("main.hsp");
        an.open_doc(
            main_url.clone(),
            NO_VERSION,
            r#"
#var v1, 2
#varlabel l1, l2
#varstr s1, s2
#vardouble d1, d2
#varint i1, i2
"#
            .into(),
        );

        let res = an.compute_ref().document_symbol(main_url).unwrap();
        let mut formatted = String::new();
        format_response(&mut formatted, &res);

        expect![[r#"
            main.hsp:2:6 v1 Variable
            main.hsp:3:11 l1 Variable
            main.hsp:3:15 l2 Variable
            main.hsp:4:9 s1 Variable
            main.hsp:4:13 s2 Variable
            main.hsp:5:12 d1 Variable
            main.hsp:5:16 d2 Variable
            main.hsp:6:9 i1 Variable
            main.hsp:6:13 i2 Variable
        "#]]
        .assert_eq(&formatted);
    }
}
