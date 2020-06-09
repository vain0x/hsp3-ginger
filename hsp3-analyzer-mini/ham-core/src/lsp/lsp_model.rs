use super::features;
use crate::{
    canonical_uri::CanonicalUri,
    docs::{DocChange, Docs},
    help_source::collect_all_symbols,
    rc_str::RcStr,
    sem::{self, ProjectSem},
    syntax,
};
use lsp_types::*;
use std::{mem::take, path::PathBuf, rc::Rc};

const NO_VERSION: i64 = 1;

#[derive(Default)]
pub(super) struct LspModel {
    sem: ProjectSem,
    hsp_root: PathBuf,
    docs_opt: Option<Docs>,
    doc_changes: Vec<DocChange>,
}

fn loc_to_range(loc: syntax::Loc) -> Range {
    // FIXME: UTF-8 から UTF-16 基準のインデックスへの変換
    Range::new(
        Position::new(loc.start.row as u64, loc.start.col as u64),
        Position::new(loc.end.row as u64, loc.end.col as u64),
    )
}

impl LspModel {
    pub(super) fn new(hsp_root: PathBuf) -> Self {
        Self {
            hsp_root,
            sem: sem::ProjectSem::new(),
            ..Default::default()
        }
    }

    fn to_loc(&self, uri: &Url, position: Position) -> Option<syntax::Loc> {
        let uri = CanonicalUri::from_url(uri)?;
        let doc = self.docs_opt.as_ref()?.find_by_uri(&uri)?;

        // FIXME: position は UTF-16 ベース、pos は UTF-8 ベースなので、マルチバイト文字が含まれている場合は変換が必要
        let pos = syntax::Pos {
            row: position.line as usize,
            col: position.character as usize,
        };

        Some(syntax::Loc {
            doc,
            start: pos,
            end: pos,
        })
    }

    fn loc_to_location(&self, loc: syntax::Loc) -> Option<Location> {
        let uri = self
            .docs_opt
            .as_ref()?
            .get_uri(loc.doc)?
            .clone()
            .into_url()?;
        let range = loc_to_range(loc);
        Some(Location { uri, range })
    }

    pub(super) fn did_initialize(&mut self) {
        let mut docs = Docs::new(self.hsp_root.clone());

        debug!("hsphelp ファイルからシンボルを探索します。");
        let mut file_count = 0;
        let mut symbols = vec![];
        let mut warnings = vec![];
        collect_all_symbols(&self.hsp_root, &mut file_count, &mut symbols, &mut warnings)
            .map_err(|e| warn!("{}", e))
            .ok();
        for w in warnings {
            warn!("{}", w);
        }

        let doc = docs.fresh_doc();

        let symbols = symbols
            .into_iter()
            .enumerate()
            .map(|(i, symbol)| {
                Rc::new(sem::Symbol {
                    symbol_id: self.sem.last_symbol_id + i + 1,
                    name: symbol.name.into(),
                    kind: sem::SymbolKind::Command {
                        local: false,
                        ctype: false,
                    },
                    details: sem::SymbolDetails {
                        description: symbol.description.map(|s| s.into()),
                        documentation: symbol.documentation.clone(),
                    },
                    scope: sem::Scope::new_global(doc),
                })
            })
            .collect::<Vec<_>>();

        self.sem.last_symbol_id += symbols.len();

        self.sem.add_hs_symbols(doc, symbols);

        docs.did_initialize();

        self.docs_opt = Some(docs);
    }

    fn poll(&mut self) {
        if let Some(docs) = self.docs_opt.as_mut() {
            docs.poll();
        }
    }

    fn notify_doc_changes_to_sem(&mut self) -> Option<()> {
        let mut doc_changes = take(&mut self.doc_changes);

        self.docs_opt.as_mut()?.drain_doc_changes(&mut doc_changes);
        for change in doc_changes.drain(..) {
            match change {
                DocChange::Opened { doc, text } | DocChange::Changed { doc, text } => {
                    self.sem.update_doc(doc, RcStr::from(text));
                }
                DocChange::Closed { doc } => self.sem.close_doc(doc),
            }
        }

        assert!(doc_changes.is_empty());
        self.doc_changes = doc_changes;
        Some(())
    }

    pub(super) fn shutdown(&mut self) {
        if let Some(mut docs) = self.docs_opt.take() {
            docs.shutdown();
        }
    }

    pub(super) fn open_doc(&mut self, uri: Url, version: i64, text: String) {
        let uri = match CanonicalUri::from_url(&uri) {
            Some(uri) => uri,
            None => return,
        };

        if let Some(docs) = self.docs_opt.as_mut() {
            docs.open_doc(uri, version, text);
        }

        self.notify_doc_changes_to_sem();
        self.poll();
    }

    pub(super) fn change_doc(&mut self, uri: Url, version: i64, text: String) {
        let uri = match CanonicalUri::from_url(&uri) {
            Some(uri) => uri,
            None => return,
        };

        if let Some(docs) = self.docs_opt.as_mut() {
            docs.change_doc(uri, version, text);
        }

        self.notify_doc_changes_to_sem();
        self.poll();
    }

    pub(super) fn close_doc(&mut self, uri: Url) {
        let uri = match CanonicalUri::from_url(&uri) {
            Some(uri) => uri,
            None => return,
        };

        if let Some(docs) = self.docs_opt.as_mut() {
            docs.close_doc(uri);
        }

        self.notify_doc_changes_to_sem();
        self.poll();
    }

    pub(super) fn completion(&mut self, uri: Url, position: Position) -> CompletionList {
        self.poll();

        let go = || {
            let docs = self.docs_opt.as_ref()?;
            features::completion::completion(uri, position, docs, &mut self.sem)
        };
        go().unwrap_or_else(features::completion::incomplete_completion_list)
    }

    pub(super) fn definitions(&mut self, uri: Url, position: Position) -> Vec<Location> {
        self.poll();

        let go = || {
            let docs = self.docs_opt.as_ref()?;
            features::definitions::definitions(uri, position, docs, &mut self.sem)
        };
        go().unwrap_or(vec![])
    }

    pub(super) fn document_highlight(
        &mut self,
        uri: Url,
        position: Position,
    ) -> Vec<DocumentHighlight> {
        self.poll();

        let go = || {
            let docs = self.docs_opt.as_ref()?;
            features::document_highlight::document_highlight(uri, position, docs, &mut self.sem)
        };
        go().unwrap_or(vec![])
    }

    pub(super) fn hover(&mut self, uri: Url, position: Position) -> Option<Hover> {
        self.poll();

        let docs = self.docs_opt.as_ref()?;
        features::hover::hover(uri, position, docs, &mut self.sem)
    }

    pub(super) fn references(
        &mut self,
        uri: Url,
        position: Position,
        include_definition: bool,
    ) -> Vec<Location> {
        self.poll();

        let go = || {
            let docs = self.docs_opt.as_ref()?;
            features::references::references(uri, position, include_definition, docs, &mut self.sem)
        };
        go().unwrap_or(vec![])
    }

    pub(super) fn prepare_rename(
        &mut self,
        uri: Url,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        self.poll();

        let loc = self.to_loc(&uri, position)?;

        // カーソル直下にシンボルがなければ変更しない。
        if self.sem.locate_symbol(loc.doc, loc.start).is_none() {
            return None;
        }

        let range = loc_to_range(loc);
        Some(PrepareRenameResponse::Range(range))
    }

    pub(super) fn rename(
        &mut self,
        uri: Url,
        position: Position,
        new_name: String,
    ) -> Option<WorkspaceEdit> {
        self.poll();

        // カーソルの下にある識別子と同一のシンボルの出現箇所 (定義箇所および使用箇所) を列挙する。
        let locs = {
            let loc = self.to_loc(&uri, position)?;
            let (symbol, _) = self.sem.locate_symbol(loc.doc, loc.start)?;
            let symbol_id = symbol.symbol_id;

            let mut locs = vec![];
            self.sem.get_symbol_defs(symbol_id, &mut locs);
            self.sem.get_symbol_uses(symbol_id, &mut locs);
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
                let location = match self.loc_to_location(loc) {
                    Some(location) => location,
                    None => continue,
                };

                let (uri, range) = (location.uri, location.range);

                // common ディレクトリのファイルは変更しない。
                if uri.as_str().contains("common") {
                    return None;
                }

                let version = self
                    .docs_opt
                    .as_ref()?
                    .get_version(loc.doc)
                    .unwrap_or(NO_VERSION);

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

    pub(super) fn validate(&mut self, _uri: Url) -> Vec<Diagnostic> {
        // self.poll();
        // let uri = canonicalize_uri(uri);

        // features::diagnostics::sem_to_diagnostics(&analysis.sem)
        vec![]
    }
}
