use super::*;
use crate::{analysis::*, parse::p_param_ty::PParamCategory};

// SemanticTokensLegend を参照
fn to_semantic_token_kind(symbol: &SymbolRc) -> Option<(u32, u32)> {
    let (ty, modifiers) = match symbol.kind {
        HspSymbolKind::Param(Some(param)) => match param.category() {
            PParamCategory::ByValue => (1, 0b01), // readonly variable
            PParamCategory::ByRef => (0, 0),      // parameter
            PParamCategory::Local => (1, 0),      // variable
            PParamCategory::Auto => return None,
        },
        HspSymbolKind::StaticVar => (1, 0b10), // static variable
        HspSymbolKind::Const | HspSymbolKind::Enum => (1, 0b01), // readonly variable
        HspSymbolKind::DefFunc
        | HspSymbolKind::DefCFunc
        | HspSymbolKind::ModFunc
        | HspSymbolKind::ModCFunc
        | HspSymbolKind::LibFunc
        | HspSymbolKind::ComFunc => (2, 0), // function
        HspSymbolKind::Macro { .. } => (3, 0), // macro
        HspSymbolKind::Module => (4, 0),       // namespace
        HspSymbolKind::PluginCmd => (5, 0),    // keyword

        // Not supported:
        // HspSymbolKind::ComInterface => ?,
        _ => return None,
    };
    Some((ty, modifiers))
}

pub(crate) fn full(
    uri: Url,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let doc = docs.find_by_uri(&CanonicalUri::from_url(&uri))?;

    // force compute
    let _project = wa.require_project_for_doc(doc);

    let mut symbols = vec![];
    collect_symbol_occurrences_in_doc(wa, doc, &mut symbols);

    let mut tokens: Vec<lsp_types::SemanticToken> = vec![];

    for (symbol, loc) in symbols {
        assert_eq!(loc.doc, doc);

        let (token_type, token_modifiers_bitset) = match to_semantic_token_kind(&symbol) {
            Some(it) => it,
            None => continue,
        };

        let location = loc_to_location(loc, docs)?;

        let Position {
            line: y1,
            character: x1,
        } = location.range.start;

        tokens.push(lsp_types::SemanticToken {
            delta_line: y1,
            delta_start: x1,
            length: symbol.name().encode_utf16().count() as u32,
            token_type,
            token_modifiers_bitset,
        });
    }

    // Compute delta.
    tokens.sort_by_key(|t| (t.delta_line, t.delta_start + t.length));

    for i in (1..tokens.len()).rev() {
        let y1 = tokens[i - 1].delta_line;
        let x1 = tokens[i - 1].delta_start;
        if tokens[i].delta_line == y1 {
            tokens[i].delta_line = 0;
            tokens[i].delta_start = tokens[i].delta_start.saturating_sub(x1);
        } else {
            tokens[i].delta_line -= y1;
        }
    }

    Some(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang_service::{docs::NO_VERSION, LangService};
    use std::fmt::Write as _;

    fn dummy_url(s: &str) -> Url {
        let dummy_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".no_exist");
        Url::from_file_path(&dummy_root.join(s)).unwrap()
    }

    #[test]
    fn tokens_test() {
        let mut ls = LangService::new_standalone();

        let uri = dummy_url("semantic_tokens.hsp");
        ls.open_doc(
            uri.clone(),
            NO_VERSION,
            r#"
#const k = 42
#define macro1
#define ctype macro2(%1) (%1)
#enum e = 10
#cmd kw

#module m1
#defcfunc f int n, var v, local l
    return
#global

    s = 0
"#
            .into(),
        );

        let tokens = ls.semantic_tokens(uri.clone());
        let mut sb = String::new();
        let mut y = 1;
        let mut x = 1;
        for t in tokens.data {
            if t.delta_line > 0 {
                y += t.delta_line;
                x = t.delta_start + 1;
            } else {
                x += t.delta_start;
            }

            write!(
                sb,
                "{}:{} {}/{}\n",
                y, x, t.token_type, t.token_modifiers_bitset
            )
            .unwrap();
        }
        assert_eq!(sb, "2:8 1/1\n3:9 3/0\n4:15 3/0\n5:7 1/1\n6:6 5/0\n8:9 4/0\n9:11 2/0\n9:17 1/1\n9:24 0/0\n9:33 1/0\n13:5 1/2\n");
    }
}
