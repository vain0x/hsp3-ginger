pub(crate) mod docs;

use crate::{
    analysis::integrate::AWorkspaceAnalysis,
    assists,
    help_source::collect_all_symbols,
    sem::{self, ProjectSem},
    utils::{canonical_uri::CanonicalUri, rc_str::RcStr},
};
use docs::{DocChange, Docs, NO_VERSION};
use lsp_types::*;
use std::{mem::take, path::PathBuf, rc::Rc};

#[derive(Default)]
pub(super) struct LangService {
    sem: ProjectSem,
    wa: AWorkspaceAnalysis,
    hsp3_home: PathBuf,
    docs_opt: Option<Docs>,
    doc_changes: Vec<DocChange>,
    hsphelp_symbols: Vec<CompletionItem>,
}

impl LangService {
    pub(super) fn new(hsp3_home: PathBuf) -> Self {
        Self {
            hsp3_home,
            sem: sem::ProjectSem::new(),
            ..Default::default()
        }
    }

    pub(super) fn did_initialize(&mut self) {
        let mut docs = Docs::new(self.hsp3_home.clone());

        debug!("hsphelp ファイルからシンボルを探索します。");
        let mut file_count = 0;
        let mut symbols = vec![];
        let mut warnings = vec![];
        collect_all_symbols(
            &self.hsp3_home,
            &mut file_count,
            &mut symbols,
            &mut warnings,
        )
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

        self.hsphelp_symbols = symbols
            .iter()
            .map(|symbol| {
                let kind = match symbol.kind {
                    sem::SymbolKind::Label => CompletionItemKind::Value,
                    sem::SymbolKind::Static | sem::SymbolKind::Param { .. } => {
                        CompletionItemKind::Variable
                    }
                    sem::SymbolKind::Macro { .. } | sem::SymbolKind::Command { .. } => {
                        CompletionItemKind::Function
                    }
                };

                // 補完候補の順番を制御するための文字。(標準命令を上に出す。)
                let sort_prefix = if symbol.name.starts_with("#") || symbol.name.starts_with("_") {
                    'y'
                } else if symbol
                    .details
                    .documentation
                    .last()
                    .map_or(false, |s| s.contains("標準命令") || s.contains("標準関数"))
                {
                    'x'
                } else {
                    'z'
                };

                CompletionItem {
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
                    sort_text: Some(format!("{}{}", sort_prefix, symbol.name)),
                    filter_text: if symbol.name.as_str().starts_with("#") {
                        Some(symbol.name.as_str().chars().skip(1).collect::<String>())
                    } else {
                        None
                    },
                    ..Default::default()
                }
            })
            .collect();

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
                    let text = RcStr::from(text);
                    self.sem.update_doc(doc, text.clone());
                    self.wa.update_doc(doc, text);
                }
                DocChange::Closed { doc } => {
                    self.sem.close_doc(doc);
                    self.wa.close_doc(doc);
                }
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
        let uri = CanonicalUri::from_url(&uri);

        if let Some(docs) = self.docs_opt.as_mut() {
            docs.open_doc(uri, version, text);
        }

        self.notify_doc_changes_to_sem();
        self.poll();
    }

    pub(super) fn change_doc(&mut self, uri: Url, version: i64, text: String) {
        let uri = CanonicalUri::from_url(&uri);

        if let Some(docs) = self.docs_opt.as_mut() {
            docs.change_doc(uri, version, text);
        }

        self.notify_doc_changes_to_sem();
        self.poll();
    }

    pub(super) fn close_doc(&mut self, uri: Url) {
        let uri = CanonicalUri::from_url(&uri);

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
            assists::completion::completion(
                uri,
                position,
                docs,
                &mut self.wa,
                &self.hsphelp_symbols,
            )
        };
        go().unwrap_or_else(assists::completion::incomplete_completion_list)
    }

    pub(super) fn definitions(&mut self, uri: Url, position: Position) -> Vec<Location> {
        self.poll();

        let go = || {
            let docs = self.docs_opt.as_ref()?;
            assists::definitions::definitions(uri, position, docs, &mut self.sem)
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
            assists::document_highlight::document_highlight(uri, position, docs, &mut self.wa)
        };
        go().unwrap_or(vec![])
    }

    pub(super) fn hover(&mut self, uri: Url, position: Position) -> Option<Hover> {
        self.poll();

        let docs = self.docs_opt.as_ref()?;
        assists::hover::hover(uri, position, docs, &mut self.sem)
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
            assists::references::references(uri, position, include_definition, docs, &mut self.sem)
        };
        go().unwrap_or(vec![])
    }

    pub(super) fn prepare_rename(
        &mut self,
        uri: Url,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        self.poll();

        let docs = self.docs_opt.as_ref()?;
        assists::rename::prepare_rename(uri, position, docs, &mut self.sem)
    }

    pub(super) fn rename(
        &mut self,
        uri: Url,
        position: Position,
        new_name: String,
    ) -> Option<WorkspaceEdit> {
        self.poll();

        let docs = self.docs_opt.as_ref()?;
        assists::rename::rename(uri, position, new_name, docs, &mut self.sem)
    }

    pub(super) fn validate(&mut self, _uri: &Url) -> (Option<i64>, Vec<Diagnostic>) {
        // FIXME: 実装
        (Some(NO_VERSION), vec![])
    }
}
