use std::collections::HashMap;

use lsp_types::{Diagnostic, DiagnosticSeverity, Url};

use crate::{
    analysis::integrate::AWorkspaceAnalysis,
    assists::{loc_to_range, to_lsp_range},
    lang_service::docs::Docs,
    source::DocId,
};

pub(crate) fn diagnose(
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Vec<(Url, Option<i64>, Vec<Diagnostic>)> {
    let mut dd = vec![];
    wa.diagnose(&mut dd);

    let mut lints = vec![];
    wa.diagnose_syntax_lints(&mut lints);

    let mut map: HashMap<DocId, Vec<Diagnostic>> = HashMap::new();
    for (message, loc) in dd {
        let d = Diagnostic {
            message,
            severity: Some(DiagnosticSeverity::Error),
            range: to_lsp_range(loc.range),
            source: source(),
            ..Default::default()
        };
        map.entry(loc.doc).or_default().push(d);
    }
    for (lint, loc) in lints {
        let d = Diagnostic {
            message: lint.as_str().to_string(),
            severity: Some(DiagnosticSeverity::Warning),
            range: loc_to_range(loc),
            source: source(),
            ..Default::default()
        };
        map.entry(loc.doc).or_default().push(d);
    }

    let mut doc_diagnostics = vec![];
    for (doc, diagnostics) in map {
        let uri = match docs.get_uri(doc) {
            Some(it) => it.clone().into_url(),
            None => continue,
        };
        let version = docs.get_version(doc);

        doc_diagnostics.push((uri, version, diagnostics));
    }

    doc_diagnostics
}

fn source() -> Option<String> {
    Some(env!("CARGO_PKG_NAME").to_string())
}
