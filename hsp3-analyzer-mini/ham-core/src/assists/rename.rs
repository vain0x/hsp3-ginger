use super::{loc_to_location, loc_to_range, to_loc};
use crate::{
    lang_service::docs::{self, Docs},
    sem::ProjectSem,
};
use lsp_types::{
    DocumentChanges, Position, PrepareRenameResponse, TextDocumentEdit, TextEdit, Url,
    VersionedTextDocumentIdentifier, WorkspaceEdit,
};

pub(crate) fn prepare_rename(
    uri: Url,
    position: Position,
    docs: &Docs,
    sem: &mut ProjectSem,
) -> Option<PrepareRenameResponse> {
    let loc = to_loc(&uri, position, docs)?;

    // カーソル直下にシンボルがなければ変更しない。
    if sem.locate_symbol(loc.doc, loc.start()).is_none() {
        return None;
    }

    let range = loc_to_range(loc);
    Some(PrepareRenameResponse::Range(range))
}

pub(crate) fn rename(
    uri: Url,
    position: Position,
    new_name: String,
    docs: &Docs,
    sem: &mut ProjectSem,
) -> Option<WorkspaceEdit> {
    // カーソルの下にある識別子と同一のシンボルの出現箇所 (定義箇所および使用箇所) を列挙する。
    let locs = {
        let loc = to_loc(&uri, position, docs)?;
        let (symbol, _) = sem.locate_symbol(loc.doc, loc.start())?;
        let symbol_id = symbol.symbol_id;

        let mut locs = vec![];
        sem.get_symbol_defs(symbol_id, &mut locs);
        sem.get_symbol_uses(symbol_id, &mut locs);
        if locs.is_empty() {
            return None;
        }

        // 1つの出現箇所が定義と使用の両方にカウントされてしまうケースがあるようなので、重複を削除する。
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
