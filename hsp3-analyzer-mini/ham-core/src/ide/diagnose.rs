use super::*;
use crate::{
    analyzer::{doc_interner::DocInterner, docs::Docs},
    ide::{loc_to_range, to_lsp_range},
};
use lsp_types::{Diagnostic, DiagnosticSeverity, Url};

#[derive(Default)]
pub(crate) struct DiagnosticsCache {
    map1: HashMap<Url, (Option<i32>, String)>,
    map2: HashMap<Url, (Option<i32>, String)>,
}

fn filter_diagnostics(
    diagnostics: &mut Vec<(Url, Option<i32>, Vec<Diagnostic>)>,
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

pub(crate) fn diagnose(
    an: &AnalyzerRef<'_>,
    doc_interner: &DocInterner,
    docs: &Docs,
    cache: &mut DiagnosticsCache,
) -> Vec<(Url, Option<i32>, Vec<Diagnostic>)> {
    let mut dd = vec![];
    an.diagnose_precisely(&mut dd);

    let mut lints = vec![];
    an.diagnose_syntax_lints(&mut lints);

    let mut map: HashMap<DocId, Vec<Diagnostic>> = HashMap::new();
    for (message, loc) in dd {
        let d = Diagnostic {
            message,
            severity: Some(DiagnosticSeverity::ERROR),
            range: to_lsp_range(loc.range),
            source: source(),
            ..Default::default()
        };
        map.entry(loc.doc).or_default().push(d);
    }
    for (lint, loc) in lints {
        let d = Diagnostic {
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
    doc_diagnostics
}

fn source() -> Option<String> {
    Some(env!("CARGO_PKG_NAME").to_string())
}
