use super::{from_document_position, loc_to_location, loc_to_range};
use crate::{
    analysis::integrate::AWorkspaceAnalysis,
    lang_service::docs::{self, Docs},
};
use lsp_types::{
    DocumentChanges, Position, PrepareRenameResponse, TextDocumentEdit, TextEdit, Url,
    VersionedTextDocumentIdentifier, WorkspaceEdit,
};

pub(crate) fn prepare_rename(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Option<PrepareRenameResponse> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;

    // FIXME: カーソル直下に識別子があって、それの定義がワークスペース内のファイル (commonやhsphelpでない) にあったときだけSomeを返す。

    let (_, loc) = wa.locate_symbol(doc, pos)?;
    let range = loc_to_range(loc);
    Some(PrepareRenameResponse::Range(range))
}

pub(crate) fn rename(
    uri: Url,
    position: Position,
    new_name: String,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Option<WorkspaceEdit> {
    // カーソルの下にある識別子と同一のシンボルの出現箇所 (定義箇所および使用箇所) を列挙する。
    let locs = {
        let (doc, pos) = from_document_position(&uri, position, docs)?;
        let (symbol, _) = wa.locate_symbol(doc, pos)?;

        let mut locs = vec![];
        wa.collect_symbol_defs(symbol, &mut locs);
        wa.collect_symbol_uses(symbol, &mut locs);
        if locs.is_empty() {
            return None;
        }

        // 1つの出現が定義と使用の両方にカウントされることもあるので、重複を削除する。
        // (重複した変更をレスポンスに含めると名前の変更に失敗する。)
        locs.sort();
        locs.dedup();

        locs
    };

    // 名前変更の編集手順を構築する。(シンボルが書かれている位置をすべて新しい名前で置き換える。)
    let changes = {
        let mut edits = vec![];
        for loc in locs {
            let location = match loc_to_location(loc, docs) {
                Some(location) => location,
                None => continue,
            };

            let (uri, range) = (location.uri, location.range);

            // common ディレクトリのファイルは変更しない。
            if uri.as_str().contains("common") {
                return None;
            }

            let version = docs.get_version(loc.doc).unwrap_or(docs::NO_VERSION);

            let text_document = VersionedTextDocumentIdentifier {
                uri,
                version: Some(version),
            };
            let text_edit = TextEdit {
                range,
                new_text: new_name.to_string(),
            };

            edits.push(TextDocumentEdit {
                text_document,
                edits: vec![text_edit],
            });
        }

        DocumentChanges::Edits(edits)
    };

    Some(WorkspaceEdit {
        document_changes: Some(changes),
        ..WorkspaceEdit::default()
    })
}
