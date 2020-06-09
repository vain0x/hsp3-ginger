use crate::{
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

fn canonicalize_uri(uri: Url) -> Url {
    uri.to_file_path()
        .ok()
        .and_then(|path| path.canonicalize().ok())
        .and_then(|path| Url::from_file_path(path).ok())
        .unwrap_or(uri)
}

fn loc_to_range(loc: syntax::Loc) -> Range {
    // FIXME: UTF-8 から UTF-16 基準のインデックスへの変換
    Range::new(
        Position::new(loc.start.row as u64, loc.start.col as u64),
        Position::new(loc.end.row as u64, loc.end.col as u64),
    )
}

fn plain_text_to_marked_string(text: String) -> MarkedString {
    const PLAIN_LANG_ID: &str = "plaintext";

    MarkedString::LanguageString(LanguageString {
        language: PLAIN_LANG_ID.to_string(),
        value: text,
    })
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
        let doc = self.docs_opt.as_ref()?.find_by_uri(uri)?;

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
        let uri = self.docs_opt.as_ref()?.get_uri(loc.doc)?.clone();
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
        let uri = canonicalize_uri(uri);

        if let Some(docs) = self.docs_opt.as_mut() {
            docs.open_doc(uri, version, text);
        }

        self.notify_doc_changes_to_sem();
        self.poll();
    }

    pub(super) fn change_doc(&mut self, uri: Url, version: i64, text: String) {
        let uri = canonicalize_uri(uri);

        if let Some(docs) = self.docs_opt.as_mut() {
            docs.change_doc(uri, version, text);
        }

        self.notify_doc_changes_to_sem();
        self.poll();
    }

    pub(super) fn close_doc(&mut self, uri: Url) {
        let uri = canonicalize_uri(uri);

        if let Some(docs) = self.docs_opt.as_mut() {
            docs.close_doc(uri);
        }

        self.notify_doc_changes_to_sem();
        self.poll();
    }

    fn do_completion(&mut self, uri: &Url, position: Position) -> Option<CompletionList> {
        let mut items = vec![];
        let mut symbols = vec![];

        let loc = self.to_loc(uri, position)?;

        self.sem.get_symbol_list(loc.doc, loc.start, &mut symbols);

        for symbol in symbols {
            let kind = match symbol.kind {
                sem::SymbolKind::Macro { ctype: true, .. }
                | sem::SymbolKind::Command { ctype: true, .. } => CompletionItemKind::Function,
                sem::SymbolKind::Label | sem::SymbolKind::Macro { .. } => {
                    CompletionItemKind::Constant
                }
                sem::SymbolKind::Command { .. } => CompletionItemKind::Method, // :thinking_face:
                sem::SymbolKind::Param { .. } | sem::SymbolKind::Static => {
                    CompletionItemKind::Variable
                }
            };

            items.push(CompletionItem {
                kind: Some(kind),
                label: symbol.name.to_string(),
                detail: symbol.details.description.as_ref().map(|s| s.to_string()),
                documentation: if symbol.details.documentation.is_empty() {
                    None
                } else {
                    Some(Documentation::String(
                        symbol.details.documentation.join("\r\n\r\n"),
                    ))
                },
                filter_text: if symbol.name.as_str().starts_with("#") {
                    Some(symbol.name.as_str().chars().skip(1).collect::<String>())
                } else {
                    None
                },
                data: Some(serde_json::to_value(&symbol.symbol_id).unwrap()),
                ..CompletionItem::default()
            })
        }

        Some(CompletionList {
            is_incomplete: false,
            items,
        })
    }

    pub(super) fn completion(&mut self, uri: Url, position: Position) -> CompletionList {
        self.poll();
        let uri = canonicalize_uri(uri);

        self.do_completion(&uri, position)
            .unwrap_or(CompletionList {
                is_incomplete: true,
                items: vec![],
            })
    }

    fn do_definitions(&mut self, uri: Url, position: Position) -> Option<Vec<Location>> {
        let loc = self.to_loc(&uri, position)?;
        let (symbol, _) = self.sem.locate_symbol(loc.doc, loc.start)?;
        let symbol_id = symbol.symbol_id;

        let mut locs = vec![];

        self.sem.get_symbol_defs(symbol_id, &mut locs);

        Some(
            locs.into_iter()
                .filter_map(|loc| self.loc_to_location(loc))
                .collect(),
        )
    }

    pub(super) fn definitions(&mut self, uri: Url, position: Position) -> Vec<Location> {
        self.poll();
        let uri = canonicalize_uri(uri);

        self.do_definitions(uri, position).unwrap_or(vec![])
    }

    fn do_highlights(&mut self, uri: Url, position: Position) -> Option<Vec<DocumentHighlight>> {
        let loc = self.to_loc(&uri, position)?;
        let doc = loc.doc;
        let (symbol, _) = self.sem.locate_symbol(loc.doc, loc.start)?;
        let symbol_id = symbol.symbol_id;

        let mut locs = vec![];
        let mut highlights = vec![];

        self.sem.get_symbol_defs(symbol_id, &mut locs);
        highlights.extend(
            locs.drain(..)
                .map(|loc| (DocumentHighlightKind::Write, loc)),
        );

        self.sem.get_symbol_uses(symbol_id, &mut locs);
        highlights.extend(locs.drain(..).map(|loc| (DocumentHighlightKind::Read, loc)));

        highlights.retain(|(_, loc)| loc.doc == doc);

        Some(
            highlights
                .into_iter()
                .map(|(kind, loc)| DocumentHighlight {
                    kind: Some(kind),
                    range: loc_to_range(loc),
                })
                .collect(),
        )
    }

    pub(super) fn highlights(&mut self, uri: Url, position: Position) -> Vec<DocumentHighlight> {
        self.poll();
        let uri = canonicalize_uri(uri);

        self.do_highlights(uri, position).unwrap_or(vec![])
    }

    pub(super) fn hover(&mut self, uri: Url, position: Position) -> Option<Hover> {
        self.poll();
        let uri = canonicalize_uri(uri);

        let loc = self.to_loc(&uri, position)?;
        let (symbol, symbol_loc) = self.sem.locate_symbol(loc.doc, loc.start)?;
        let symbol_id = symbol.symbol_id;

        let mut contents = vec![];
        contents.push(plain_text_to_marked_string(symbol.name.to_string()));

        if let Some(description) = symbol.details.description.as_ref() {
            contents.push(plain_text_to_marked_string(description.to_string()));
        }

        contents.extend(
            symbol
                .details
                .documentation
                .iter()
                .map(|text| plain_text_to_marked_string(text.to_string())),
        );

        {
            let mut locs = vec![];
            self.sem.get_symbol_defs(symbol_id, &mut locs);
            let def_links = locs
                .iter()
                .filter_map(|&loc| {
                    let location = self.loc_to_location(loc)?;
                    let uri = location
                        .uri
                        .to_string()
                        .replace("%3A", ":")
                        .replace("\\", "/");
                    let Position { line, character } = location.range.start;
                    Some(format!("- [{}:{}:{}]({})", uri, line, character, uri))
                })
                .collect::<Vec<_>>();
            if !def_links.is_empty() {
                contents.push(MarkedString::from_markdown(def_links.join("\r\n")));
            }
        }

        Some(Hover {
            contents: HoverContents::Array(contents),
            range: Some(loc_to_range(symbol_loc)),
        })
    }

    fn do_references(
        &mut self,
        uri: Url,
        position: Position,
        include_definition: bool,
    ) -> Option<Vec<Location>> {
        let loc = self.to_loc(&uri, position)?;
        let (symbol, _) = self.sem.locate_symbol(loc.doc, loc.start)?;
        let symbol_id = symbol.symbol_id;

        let mut locs = vec![];

        if include_definition {
            self.sem.get_symbol_defs(symbol_id, &mut locs);
        }
        self.sem.get_symbol_uses(symbol_id, &mut locs);

        Some(
            locs.into_iter()
                .filter_map(|loc| self.loc_to_location(loc))
                .collect(),
        )
    }

    pub(super) fn references(
        &mut self,
        uri: Url,
        position: Position,
        include_definition: bool,
    ) -> Vec<Location> {
        self.poll();
        let uri = canonicalize_uri(uri);

        self.do_references(uri, position, include_definition)
            .unwrap_or(vec![])
    }

    pub(super) fn prepare_rename(
        &mut self,
        uri: Url,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        self.poll();
        let uri = canonicalize_uri(uri);

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
        let uri = canonicalize_uri(uri);

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
