//! 定義へ移動

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

// ===============================================

#[cfg(test)]
mod tests {
    use crate::{
        analyzer::{Analyzer, DocDb},
        lsp_server::NO_VERSION,
        test_utils::set_test_logger,
        utils::canonical_uri::CanonicalUri,
    };
    use expect_test::expect;
    use std::fmt::Write as _;

    fn dummy_url(s: &str) -> lsp_types::Url {
        let workspace_dir = crate::test_utils::dummy_path().join("ws");
        lsp_types::Url::from_file_path(&workspace_dir.join(s)).unwrap()
    }

    fn dummy_common_url(s: &str) -> lsp_types::Url {
        let common_dir = crate::test_utils::dummy_path().join("hsp3").join("common");
        lsp_types::Url::from_file_path(&common_dir.join(s)).unwrap()
    }

    fn format_loc(w: &mut String, l: &lsp_types::Location) {
        let start = l.range.start;
        write!(w, "{}:{}", start.line + 1, start.character + 1).unwrap();
    }

    fn format_response(w: &mut String, res: &[lsp_types::Location]) {
        for l in res {
            format_loc(w, l);
            *w += "\n";
        }
    }

    #[test]
    fn test_include() {
        set_test_logger();
        let mut an = Analyzer::new_standalone();

        let main_url = dummy_url("main.hsp");
        an.open_doc(
            main_url.clone(),
            NO_VERSION,
            r#"
#include "a.hsp"
#include "b.hsp"
"#
            .into(),
        );
        an.open_doc(dummy_url("a.hsp"), NO_VERSION, "".into());
        let an = an.compute_ref();

        let mut formatted = String::new();
        formatted += "[a.hsp]\n";
        format_response(
            &mut formatted,
            &an.definitions(main_url.clone(), lsp_types::Position::new(1, 1)),
        );

        formatted += "[b.hsp]\n";
        format_response(
            &mut formatted,
            &an.definitions(main_url, lsp_types::Position::new(2, 1)),
        );

        expect![[r#"
            [a.hsp]
            1:1
            [b.hsp]
        "#]]
        .assert_eq(&formatted);
    }

    #[test]
    fn test_use_preproc() {
        set_test_logger();
        let mut an = Analyzer::new_standalone();

        // common 以下にダミーのファイルを生成する
        let kernel32 = dummy_common_url("kernel32.as");
        an.open_doc(kernel32.clone(), NO_VERSION, "".into());

        an.common_docs.insert(
            "kernel32.as".to_string(),
            an.find_doc_by_uri(&CanonicalUri::from_url(&kernel32))
                .unwrap(),
        );

        let d3m = dummy_common_url("d3m.hsp");
        an.open_doc(d3m, NO_VERSION, "".into());

        an.common_docs.insert(
            "d3m.hsp".to_string(),
            an.find_doc_by_uri(&CanonicalUri::from_url(&kernel32))
                .unwrap(),
        );

        let main_url = dummy_url("main.hsp");
        an.open_doc(
            main_url.clone(),
            NO_VERSION,
            r#"
#use kernel32, d3m, unknown
"#
            .into(),
        );

        let an = an.compute_ref();

        let mut formatted = String::new();
        formatted += "[kernel32]\n";
        format_response(
            &mut formatted,
            &an.definitions(
                main_url.clone(),
                // <|>kernel32
                lsp_types::Position::new(1, 5),
            ),
        );

        formatted += "[d3m]\n";
        format_response(
            &mut formatted,
            // , <|>d3m
            &an.definitions(main_url.clone(), lsp_types::Position::new(1, 15)),
        );

        // unknown は存在しないのでレスポンスは空になる
        formatted += "[unknown]\n";
        format_response(
            &mut formatted,
            // , <|>unknown
            &an.definitions(main_url, lsp_types::Position::new(1, 20)),
        );

        expect![[r#"
            [kernel32]
            1:1
            [d3m]
            1:1
            [unknown]
        "#]]
        .assert_eq(&formatted);
    }
}
