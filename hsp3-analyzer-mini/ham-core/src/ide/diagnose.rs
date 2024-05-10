use super::*;
use crate::{
    analysis,
    analyzer::{doc_interner::DocInterner, docs::Docs},
    ide::{loc_to_range, to_lsp_range},
    lsp_server::TextDocumentVersion,
};
use lsp_types::DiagnosticSeverity;

// -----------------------------------------------
// Computation
// -----------------------------------------------

pub(crate) fn diagnose_syntax_lints(an: &AnalyzerRef<'_>, lints: &mut Vec<(SyntaxLint, Loc)>) {
    for (&doc, da) in an.doc_analysis_map.iter() {
        if !an.is_active_doc(doc) {
            continue;
        }

        // if !da.syntax_lint_done {
        //     debug_assert_eq!(da.syntax_lints.len(), 0);
        //     let tree = or!(da.tree_opt.as_ref(), continue);
        //     crate::analysis::syntax_linter::syntax_lint(&tree, &mut da.syntax_lints);
        //     da.syntax_lint_done = true;
        // }
        // lints.extend(da.syntax_lints.iter().cloned());

        let tree = match &da.tree_opt {
            Some(it) => it,
            None => continue,
        };
        analysis::syntax_linter::syntax_lint(&tree, lints);
    }
}

pub(crate) fn diagnose_precisely(an: &AnalyzerRef<'_>, diagnostics: &mut Vec<(String, Loc)>) {
    let use_site_map = an
        .use_sites
        .iter()
        .map(|(symbol, loc)| ((loc.doc, loc.start()), symbol.clone()))
        .collect::<HashMap<_, _>>();

    let mut ctx = SemaLinter {
        use_site_map,
        diagnostics: vec![],
    };

    for (&doc, da) in an.doc_analysis_map.iter() {
        if !an.is_active_doc(doc) {
            continue;
        }

        let root = match &da.tree_opt {
            Some(it) => it,
            None => continue,
        };

        ctx.on_root(root);
    }

    diagnostics.extend(ctx.diagnostics.into_iter().map(|(d, loc)| {
        let msg = match d {
            Diagnostic::Undefined => "定義が見つかりません",
            Diagnostic::VarRequired => "変数か配列の要素が必要です。",
        }
        .to_string();
        (msg, loc)
    }));
}

// -----------------------------------------------
// Filtering
// -----------------------------------------------

#[derive(Default)]
pub(crate) struct DiagnosticsCache {
    map1: HashMap<Url, String>,
    map2: HashMap<Url, String>,
}

pub(crate) fn filter_diagnostics(
    cache: &mut DiagnosticsCache,
    diagnostics: &mut Vec<(Url, Option<TextDocumentVersion>, Vec<lsp_types::Diagnostic>)>,
) {
    let mut map = take(&mut cache.map1);
    let mut backup = take(&mut cache.map2);

    for (_, _, new) in diagnostics.iter_mut() {
        new.sort_by_key(|d| (d.range.start, d.range.end));
    }

    diagnostics.retain(|&(ref uri, _v, ref new)| {
        let old_opt = map.remove(&uri);
        let new_opt = if !new.is_empty() {
            serde_json::to_string(&new).ok()
        } else {
            None
        };

        let retain = match (&old_opt, &new_opt) {
            (Some(old), Some(new)) => old != new,
            (Some(_), None) | (None, Some(_)) => true,
            (None, None) => false,
        };

        if let Some(new) = new_opt {
            backup.insert(uri.clone(), new);
        }

        retain
    });

    // diagnosticsのなくなったドキュメントからdiagnosticsをクリアする。
    diagnostics.extend(map.drain().map(|(uri, _)| (uri, None, vec![])));

    cache.map1 = backup;
    cache.map2 = map;
}

// ===============================================

pub(crate) fn diagnose(
    an: &AnalyzerRef<'_>,
    hsp3_root: &Path,
    doc_interner: &DocInterner,
    docs: &Docs,
) -> Vec<(Url, Option<i32>, Vec<lsp_types::Diagnostic>)> {
    let mut dd = vec![];
    diagnose_precisely(an, &mut dd);

    let mut lints = vec![];
    diagnose_syntax_lints(an, &mut lints);

    let mut map: HashMap<DocId, Vec<lsp_types::Diagnostic>> = HashMap::new();
    for (message, loc) in dd {
        let d = lsp_types::Diagnostic {
            message,
            severity: Some(DiagnosticSeverity::ERROR),
            range: to_lsp_range(loc.range),
            source: source(),
            ..Default::default()
        };
        map.entry(loc.doc).or_default().push(d);
    }
    for (lint, loc) in lints {
        let d = lsp_types::Diagnostic {
            message: lint.as_str().to_string(),
            severity: Some(DiagnosticSeverity::WARNING),
            range: loc_to_range(loc),
            source: source(),
            ..Default::default()
        };
        map.entry(loc.doc).or_default().push(d);
    }

    let mut doc_diagnostics = vec![];
    for (doc, diagnostics) in map {
        let uri = match doc_interner.get_uri(doc) {
            Some(it) => it.clone().into_url(),
            None => continue,
        };
        let version = docs.get_version(doc);

        doc_diagnostics.push((uri, version, diagnostics));
    }

    // hsp3のファイルにdiagnosticsを出さない。
    doc_diagnostics.retain(|(uri, _, _)| {
        let ok = uri
            .to_file_path()
            .map_or(true, |path| !path.starts_with(&hsp3_root));

        if !ok {
            trace!(
                "ファイルはhsp3_rootにあるので {:?} への診断は無視されます。",
                uri
            );
        }

        ok
    });

    doc_diagnostics
}

fn source() -> Option<String> {
    Some(env!("CARGO_PKG_NAME").to_string())
}

// ===============================================

#[cfg(test)]
mod tests {
    use crate::{analyzer::Analyzer, ide::lsp::from_proto, lsp_server::NO_VERSION};
    use expect_test::expect;
    use std::fmt::Write as _;

    fn dummy_url(s: &str) -> lsp_types::Url {
        let workspace_dir = crate::test_utils::dummy_path().join("ws");
        lsp_types::Url::from_file_path(&workspace_dir.join(s)).unwrap()
    }

    fn format_diagnostics(w: &mut String, diagnostics: &[lsp_types::Diagnostic]) {
        for d in diagnostics {
            let start = from_proto::pos16(d.range.start);
            write!(
                w,
                "  {:?} {:?} {:?}\n",
                start,
                d.severity.unwrap(),
                d.message
            )
            .unwrap();
        }
    }

    fn format_response(
        w: &mut String,
        res: &[(lsp_types::Url, Option<i32>, Vec<lsp_types::Diagnostic>)],
    ) {
        for (url, version_opt, diagnostics) in res {
            write!(
                w,
                "file: {:?}@{} ({})\n",
                url.to_file_path().unwrap().file_name().unwrap(),
                version_opt.unwrap_or(0),
                diagnostics.len()
            )
            .unwrap();
            format_diagnostics(w, diagnostics);
            *w += "\n";
        }
    }

    #[test]
    fn test_sema_linter() {
        let mut an = Analyzer::new_standalone();
        an.get_options_mut().lint_enabled = true;

        let main_url = dummy_url("main.hsp");
        an.open_doc(
            main_url.clone(),
            NO_VERSION,
            r#"
repeat
    return
loop
"#
            .into(),
        );

        an.open_doc(dummy_url("ok.hsp"), NO_VERSION, "; all green\n".into());

        let an = an.compute_ref();

        let mut formatted = String::new();
        format_response(&mut formatted, &an.diagnose());

        expect![[r#"
            file: "main.hsp"@1 (1)
              3:5 Warning "repeatループの中ではreturnできません。"

        "#]]
        .assert_eq(&formatted);
    }
}
