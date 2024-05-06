use super::*;
use crate::lsp_server::NO_VERSION;
use lsp_types::{
    DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Position,
    PrepareRenameResponse, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

pub(crate) fn prepare_rename(
    wa: &AnalysisRef<'_>,
    doc_interner: &DocInterner,
    uri: Url,
    position: Position,
) -> Option<PrepareRenameResponse> {
    let (doc, pos) = from_document_position(&doc_interner, &uri, position)?;

    // FIXME: カーソル直下に識別子があって、それの定義がワークスペース内のファイル (commonやhsphelpでない) にあったときだけSomeを返す。

    let (_, loc) = wa.locate_symbol(doc, pos)?;
    let range = loc_to_range(loc);
    Some(PrepareRenameResponse::Range(range))
}

pub(crate) fn rename(
    wa: &AnalysisRef<'_>,
    doc_interner: &DocInterner,
    docs: &Docs,
    uri: Url,
    position: Position,
    new_name: String,
) -> Option<WorkspaceEdit> {
    // カーソルの下にある識別子と同一のシンボルの出現箇所 (定義箇所および使用箇所) を列挙する。
    let locs = {
        let (doc, pos) = from_document_position(doc_interner, &uri, position)?;
        let (symbol, _) = wa.locate_symbol(doc, pos)?;

        let mut locs = vec![];
        collect_symbol_occurrences(
            wa,
            CollectSymbolOptions {
                include_def: true,
                include_use: true,
            },
            &symbol,
            &mut locs,
        );
        if locs.is_empty() {
            return None;
        }

        // ソートして重複を取り除く
        // (重複した変更をレスポンスに含めると名前の変更に失敗する)
        locs.sort();
        locs.dedup();

        locs
    };

    // 名前変更の編集手順を構築する。(シンボルが書かれている位置をすべて新しい名前で置き換える。)
    let changes = {
        let mut edits = vec![];
        for loc in locs {
            let location = match loc_to_location(doc_interner, loc) {
                Some(location) => location,
                None => continue,
            };

            let (uri, range) = (location.uri, location.range);

            // common ディレクトリのファイルは変更しない。
            if uri.as_str().contains("common") {
                return None;
            }

            let version = docs.get_version(loc.doc).unwrap_or(NO_VERSION);

            let text_document = OptionalVersionedTextDocumentIdentifier {
                uri,
                version: Some(version),
            };
            let text_edit = TextEdit {
                range,
                new_text: new_name.to_string(),
            };

            edits.push(TextDocumentEdit {
                text_document,
                edits: vec![OneOf::Left(text_edit)],
            });
        }

        DocumentChanges::Edits(edits)
    };

    Some(WorkspaceEdit {
        document_changes: Some(changes),
        ..WorkspaceEdit::default()
    })
}
