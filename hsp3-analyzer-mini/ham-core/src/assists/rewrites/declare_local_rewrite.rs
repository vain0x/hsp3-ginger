use lsp_types::{
    CodeAction, CodeActionContext, DocumentChanges, Range, TextDocumentEdit, TextEdit, Url,
    VersionedTextDocumentIdentifier, WorkspaceEdit,
};

use crate::{assists::from_document_position, lang_service::docs::Docs};

pub(crate) fn declare_local_rewrite(
    uri: Url,
    range: Range,
    _context: CodeActionContext,
    docs: &Docs,
) -> Option<Vec<CodeAction>> {
    let (doc, pos) = from_document_position(&uri, range.start, &docs)?;
    let version = docs.get_version(doc);

    Some(vec![CodeAction {
        title: "declare local".into(),
        kind: Some("refactor.rewrite".into()),
        edit: Some(WorkspaceEdit {
            document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: version,
                },
                edits: vec![TextEdit {
                    range,
                    new_text: format!("{}:{}", doc, pos),
                }],
            }])),
            ..WorkspaceEdit::default()
        }),
        ..Default::default()
    }])
}
