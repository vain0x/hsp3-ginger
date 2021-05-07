use std::collections::HashMap;

use lsp_types::{Diagnostic, DiagnosticSeverity, Url};

use crate::{
    analysis::{integrate::AWorkspaceAnalysis, syntax_linter::syntax_lint},
    assists::{loc_to_range, to_lsp_range},
    lang_service::docs::Docs,
    source::{DocId, Range},
};

pub(crate) fn diagnose(
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Vec<(Url, Option<i64>, Vec<Diagnostic>)> {
    let mut dd = vec![];
    wa.diagnose(&mut dd);
    let mut map: HashMap<DocId, Vec<(String, Range)>> = HashMap::new();
    for (msg, loc) in dd {
        map.entry(loc.doc).or_default().push((msg, loc.range));
    }

    let mut doc_diagnostics = vec![];

    for (&doc, syntax) in &wa.doc_syntax_map {
        let uri = match docs.get_uri(doc) {
            Some(it) => it.clone().into_url(),
            None => continue,
        };
        let version = docs.get_version(doc);

        let lints = syntax_lint(&syntax.tree);
        let mut diagnostics = lints
            .into_iter()
            .map(|(lint, loc)| Diagnostic {
                message: lint.as_str().to_string(),
                severity: Some(DiagnosticSeverity::Warning),
                range: loc_to_range(loc),
                source: source(),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        diagnostics.extend(map.remove(&doc).unwrap_or(vec![]).into_iter().map(
            |(message, range)| Diagnostic {
                message,
                severity: Some(DiagnosticSeverity::Error),
                range: to_lsp_range(range),
                source: source(),
                ..Default::default()
            },
        ));

        doc_diagnostics.push((uri, version, diagnostics));
    }

    doc_diagnostics
}

fn source() -> Option<String> {
    Some(env!("CARGO_PKG_NAME").to_string())
}
