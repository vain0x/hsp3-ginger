use lsp_types::{Diagnostic, DiagnosticSeverity, Url};

use crate::{
    analysis::{integrate::AWorkspaceAnalysis, syntax_linter::syntax_lint},
    assists::loc_to_range,
    lang_service::docs::Docs,
};

pub(crate) fn diagnose(
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Vec<(Url, Option<i64>, Vec<Diagnostic>)> {
    wa.diagnose();

    let mut doc_diagnostics = vec![];

    // FIXME: 開いていないファイルも診断する

    for (&doc, syntax) in &wa.doc_syntax_map {
        let uri = match docs.get_uri(doc) {
            Some(it) => it.clone().into_url(),
            None => continue,
        };
        let version = docs.get_version(doc);

        let lints = syntax_lint(&syntax.tree);
        let diagnostics = lints
            .into_iter()
            .map(|(lint, loc)| Diagnostic {
                message: lint.as_str().to_string(),
                severity: Some(DiagnosticSeverity::Warning),
                range: loc_to_range(loc),
                source: Some(env!("CARGO_PKG_NAME").to_string()),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        doc_diagnostics.push((uri, version, diagnostics));
    }

    doc_diagnostics
}
