use super::*;

pub(crate) fn references(
    an: &AnalyzerRef<'_>,
    doc_interner: &DocInterner,
    uri: Url,
    position: Position,
    include_definition: bool,
) -> Option<Vec<Location>> {
    let (doc, pos) = from_document_position(doc_interner, &uri, position)?;
    let (symbol, _) = an.locate_symbol(doc, pos)?;

    let mut locs = vec![];
    collect_symbol_occurrences(
        an,
        CollectSymbolOptions {
            include_def: include_definition,
            include_use: true,
        },
        &symbol,
        &mut locs,
    );

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
    use crate::{analyzer::Analyzer, lsp_server::NO_VERSION, test_utils::set_test_logger};
    use expect_test::expect;
    use std::fmt::Write as _;

    fn dummy_url(s: &str) -> lsp_types::Url {
        let workspace_dir = crate::test_utils::dummy_path().join("ws");
        lsp_types::Url::from_file_path(&workspace_dir.join(s)).unwrap()
    }

    fn file_name(url: &lsp_types::Url) -> Option<&str> {
        url.path_segments().and_then(|segments| segments.last())
    }

    fn format_loc(w: &mut String, l: &lsp_types::Location) {
        let start = l.range.start;
        write!(
            w,
            "{}:{}:{}",
            file_name(&l.uri).unwrap_or("???"),
            start.line + 1,
            start.character + 1
        )
        .unwrap();
    }

    fn format_response(w: &mut String, res: &[lsp_types::Location]) {
        for l in res {
            format_loc(w, l);
            *w += "\n";
        }
    }

    #[test]
    fn test_label() {
        set_test_logger();
        let mut an = Analyzer::new_standalone();

        let main_url = dummy_url("main.hsp");
        an.open_doc(
            main_url.clone(),
            NO_VERSION,
            r#"
    gosub *foo

*foo
    return

#module m1
#deffunc my_func
*foo
    gosub *foo
    gosub *foo@
#global

    gosub *foo
"#
            .into(),
        );

        an.open_doc(
            dummy_url("other.hsp"),
            NO_VERSION,
            r#"
#include "main.hsp"

    gosub *foo
    gosub *foo@
"#
            .into(),
        );
        let an = an.compute_ref();

        let mut formatted = String::new();
        formatted += "[*foo@]\n";
        format_response(
            &mut formatted,
            // 最初の `*foo`
            &an.references(main_url.clone(), lsp_types::Position::new(1, 8), true),
        );

        formatted += "[*foo@m1]\n";
        format_response(
            &mut formatted,
            // モジュールの中の `*foo`
            &an.references(main_url, lsp_types::Position::new(8, 1), true),
        );

        expect![[r#"
            [*foo@]
            main.hsp:2:5
            main.hsp:14:5
            other.hsp:4:5
            other.hsp:5:5
            [*foo@m1]
            main.hsp:9:1
            main.hsp:10:11
        "#]]
        .assert_eq(&formatted);
    }
}
