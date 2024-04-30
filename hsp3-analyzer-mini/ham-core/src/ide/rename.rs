use super::*;
use lsp_types::{
    DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Position,
    PrepareRenameResponse, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

pub(crate) fn prepare_rename(
    wa: &AnalysisRef<'_>,
    docs: &Docs,
    uri: Url,
    position: Position,
) -> Option<PrepareRenameResponse> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    let project = wa.require_project_for_doc(doc);

    // FIXME: カーソル直下に識別子があって、それの定義がワークスペース内のファイル (commonやhsphelpでない) にあったときだけSomeを返す。

    let (_, loc) = project.locate_symbol(doc, pos)?;
    let range = loc_to_range(loc);
    Some(PrepareRenameResponse::Range(range))
}

pub(crate) fn rename(
    wa: &AnalysisRef<'_>,
    docs: &Docs,
    uri: Url,
    position: Position,
    new_name: String,
) -> Option<WorkspaceEdit> {
    // カーソルの下にある識別子と同一のシンボルの出現箇所 (定義箇所および使用箇所) を列挙する。
    let locs = {
        let (doc, pos) = from_document_position(&uri, position, docs)?;
        let project = wa.require_project_for_doc(doc);

        let (symbol, _) = project.locate_symbol(doc, pos)?;

        let include_graph = IncludeGraph::generate(wa, docs);
        let mut locs = vec![];
        collect_symbol_defs(wa, &include_graph, doc, &symbol, &mut locs);
        collect_symbol_uses(wa, &include_graph, doc, &symbol, &mut locs);
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
