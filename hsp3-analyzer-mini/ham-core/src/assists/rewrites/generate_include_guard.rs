//! インクルードガードを生成するアクション

use super::*;
use lsp_types::{
    CodeAction, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Range,
    TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

pub(crate) fn generate_include_guard(
    uri: &Url,
    range: Range,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<Vec<CodeAction>> {
    let (doc, pos) = from_document_position(&uri, range.start, &docs)?;
    let version = docs.get_version(doc);

    let DocSyntax { text, tokens, .. } = wa.get_syntax(doc)?;

    // カーソルが行頭にあって、最初のトークン以前にあって、文字列やコメントの外であって、インクルードガードがまだないとき。
    let ok = pos.column == 0
        && pos <= tokens.first()?.body_pos16()
        && !wa.in_str_or_comment(doc, pos).unwrap_or(true)
        && !wa.has_include_guard(doc);
    if !ok {
        return None;
    }

    // ファイル名からシンボルを生成する。
    let name = {
        let path = docs.get_uri(doc)?.clone().into_url().to_file_path().ok()?;
        let name = path.file_name()?.to_str()?;
        name.replace(".", "_") + "_included"
    };
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let new_text = format!(
        "#ifndef {name}{eol}#define {name}{eol}{eol}#endif{eol}",
        name = name,
        eol = eol,
    );

    Some(vec![CodeAction {
        title: "インクルードガードを生成する".into(),
        kind: Some("refactor.rewrite".into()),
        edit: Some(WorkspaceEdit {
            document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                edits: vec![OneOf::Left(TextEdit { range, new_text })],
            }])),
            ..WorkspaceEdit::default()
        }),
        ..Default::default()
    }])
}
