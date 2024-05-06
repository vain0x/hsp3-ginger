use super::*;
use crate::{
    analysis,
    analyzer::{doc_interner::DocInterner, docs::Docs},
    ide::{loc_to_range, to_lsp_range},
};
use lsp_types::{DiagnosticSeverity, Url};

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
    map1: HashMap<Url, (Option<i32>, String)>,
    map2: HashMap<Url, (Option<i32>, String)>,
}

fn filter_diagnostics(
    diagnostics: &mut Vec<(Url, Option<i32>, Vec<lsp_types::Diagnostic>)>,
    doc_interner: &DocInterner,
    docs: &Docs,
    cache: &mut DiagnosticsCache,
) {
    let mut map = take(&mut cache.map1);
    let mut backup = take(&mut cache.map2);

    for (_, _, new) in diagnostics.iter_mut() {
        new.sort_by_key(|d| (d.range.start, d.range.end));
    }

    diagnostics.retain(|&(ref uri, version, ref new)| {
        let old_opt = map.remove(&uri);
        let new_opt = if !new.is_empty() {
            serde_json::to_string(&new).ok()
        } else {
            None
        };

        let retain = match (&old_opt, &new_opt) {
            (Some((_, old)), Some(new)) => old != new,
            (Some(_), None) | (None, Some(_)) => true,
            (None, None) => false,
        };

        if let Some(new) = new_opt {
            backup.insert(uri.clone(), (version, new));
        }

        retain
    });

    // diagnosticsのなくなったドキュメントからdiagnosticsをクリアする。
    diagnostics.extend(map.drain().map(|(uri, (version, _))| {
        let version = doc_interner
            .get_doc(&CanonicalUri::from_url(&uri))
            .and_then(|doc| docs.get_version(doc))
            .or(version);

        (uri, version, vec![])
    }));

    cache.map1 = backup;
    cache.map2 = map;
}

// ===============================================

pub(crate) fn diagnose(
    an: &AnalyzerRef<'_>,
    hsp3_root: &Path,
    doc_interner: &DocInterner,
    docs: &Docs,
    cache: &mut DiagnosticsCache,
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

    filter_diagnostics(&mut doc_diagnostics, doc_interner, docs, cache);

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
