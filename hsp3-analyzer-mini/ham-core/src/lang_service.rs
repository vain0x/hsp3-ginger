pub(crate) mod docs;
pub(crate) mod file_watcher;

use self::{
    docs::{DocChange, Docs},
    file_watcher::FileWatcher,
};
use crate::{
    analysis::{
        a_symbol::AWsSymbol, integrate::AWorkspaceAnalysis, preproc::ASignatureData, ASymbol,
    },
    assists,
    help_source::{collect_all_symbols, HsSymbol},
    utils::canonical_uri::CanonicalUri,
};
use lsp_types::*;
use std::{mem::take, path::PathBuf, rc::Rc};

pub(crate) struct LangServiceOptions {
    pub(crate) lint_enabled: bool,
    pub(crate) watcher_enabled: bool,
}

impl LangServiceOptions {
    #[cfg(test)]
    pub(crate) fn minimal() -> Self {
        Self {
            lint_enabled: false,
            watcher_enabled: false,
        }
    }
}

impl Default for LangServiceOptions {
    fn default() -> Self {
        Self {
            lint_enabled: true,
            watcher_enabled: true,
        }
    }
}

#[derive(Default)]
pub(super) struct LangService {
    wa: AWorkspaceAnalysis,
    hsp3_home: PathBuf,
    root_uri_opt: Option<CanonicalUri>,
    options: LangServiceOptions,
    docs: Docs,
    hsphelp_symbols: Vec<CompletionItem>,
    file_watcher_opt: Option<FileWatcher>,
}

impl LangService {
    pub(super) fn new(hsp3_home: PathBuf, options: LangServiceOptions) -> Self {
        Self {
            hsp3_home,
            options,
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn new_standalone() -> Self {
        let hsp3_home = PathBuf::from("/tmp/.not_exist");
        let options = LangServiceOptions::minimal();
        Self {
            hsp3_home,
            options,
            ..Default::default()
        }
    }

    pub(super) fn initialize(&mut self, root_uri_opt: Option<Url>) {
        if let Some(uri) = root_uri_opt {
            self.root_uri_opt = Some(CanonicalUri::from_url(&uri));
        }
    }

    pub(super) fn did_initialize(&mut self) {
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

        let hsphelp_doc = self.docs.fresh_doc();

        self.hsphelp_symbols = symbols
            .into_iter()
            .enumerate()
            .map(|(i, symbol)| {
                let kind = CompletionItemKind::Function;
                let HsSymbol {
                    name,
                    description,
                    documentation,
                    signature_opt,
                    mut param_info,
                } = symbol;

                let wa_symbol = AWsSymbol {
                    doc: hsphelp_doc,
                    symbol: ASymbol::new(i),
                };
                self.wa
                    .public_env
                    .builtin
                    .insert(name.clone().into(), wa_symbol);

                if let Some(s) = signature_opt {
                    let params = {
                        let mut s = s.as_str().trim();

                        if s.starts_with('(') {
                            s = s[1..].trim_end_matches(')').trim();
                        }

                        s.split(",")
                            .map(|name| {
                                let name = name.trim().to_string();
                                let info_opt = param_info
                                    .iter_mut()
                                    .find(|s| s.starts_with(&name))
                                    .map(take);

                                (None, Some(name.into()), info_opt)
                            })
                            .collect::<Vec<_>>()
                    };

                    let signature_data = ASignatureData {
                        name: name.clone().into(),
                        params,
                    };
                    self.wa
                        .builtin_signatures
                        .insert(wa_symbol, Rc::new(signature_data));
                }

                // 補完候補の順番を制御するための文字。(標準命令を上に出す。)
                let sort_prefix = if name.starts_with("#") || name.starts_with("_") {
                    'y'
                } else if documentation
                    .last()
                    .map_or(false, |s| s.contains("標準命令") || s.contains("標準関数"))
                {
                    'x'
                } else {
                    'z'
                };

                // '#' なし
                let word = if name.as_str().starts_with("#") {
                    Some(name.as_str().chars().skip(1).collect::<String>())
                } else {
                    None
                };

                CompletionItem {
                    kind: Some(kind),
                    label: name.to_string(),
                    detail: description,
                    documentation: if documentation.is_empty() {
                        None
                    } else {
                        Some(Documentation::String(documentation.join("\r\n\r\n")))
                    },
                    sort_text: Some(format!("{}{}", sort_prefix, name)),
                    filter_text: word.clone(),
                    insert_text: word,
                    ..Default::default()
                }
            })
            .collect();

        if self.options.watcher_enabled {
            if let Some(watched_dir) = self
                .root_uri_opt
                .as_ref()
                .and_then(|uri| uri.to_file_path())
            {
                let mut watcher = FileWatcher::new(watched_dir);
                watcher.start_watch();
                self.file_watcher_opt = Some(watcher);
            }
        }
    }

    /// ドキュメントの変更を集積して、解析器の状態を更新する。
    fn poll(&mut self) {
        self.poll_watcher();
        self.apply_doc_changes();
    }

    fn poll_watcher(&mut self) {
        let watcher = match self.file_watcher_opt.as_mut() {
            Some(it) => it,
            None => return,
        };

        let mut rescan = false;
        watcher.poll(&mut rescan);

        if rescan {
            self.docs.close_all_files();
        }

        let mut changed_files = vec![];
        let mut closed_files = vec![];
        watcher.drain_changes(&mut changed_files, &mut closed_files);

        for path in changed_files {
            self.docs.change_file(&path);
        }

        for path in closed_files {
            self.docs.close_file(&path);
        }
    }

    fn apply_doc_changes(&mut self) {
        let mut doc_changes = vec![];
        self.docs.drain_doc_changes(&mut doc_changes);

        for change in doc_changes.drain(..) {
            match change {
                DocChange::Opened { doc, text } | DocChange::Changed { doc, text } => {
                    self.wa.update_doc(doc, text);
                }
                DocChange::Closed { doc } => {
                    self.wa.close_doc(doc);
                }
            }
        }
    }

    pub(super) fn shutdown(&mut self) {
        if let Some(mut watcher) = self.file_watcher_opt.take() {
            watcher.stop_watch();
        }
    }

    pub(super) fn open_doc(&mut self, uri: Url, version: i64, text: String) {
        let uri = CanonicalUri::from_url(&uri);

        self.docs.open_doc_in_editor(uri, version, text);
    }

    pub(super) fn change_doc(&mut self, uri: Url, version: i64, text: String) {
        let uri = CanonicalUri::from_url(&uri);

        self.docs.change_doc_in_editor(uri, version, text);
    }

    pub(super) fn close_doc(&mut self, uri: Url) {
        let uri = CanonicalUri::from_url(&uri);

        self.docs.close_doc_in_editor(uri);
    }

    pub(super) fn completion(&mut self, uri: Url, position: Position) -> CompletionList {
        self.poll();

        assists::completion::completion(
            uri,
            position,
            &self.docs,
            &mut self.wa,
            &self.hsphelp_symbols,
        )
        .unwrap_or_else(assists::completion::incomplete_completion_list)
    }

    pub(super) fn definitions(&mut self, uri: Url, position: Position) -> Vec<Location> {
        self.poll();

        assists::definitions::definitions(uri, position, &self.docs, &mut self.wa).unwrap_or(vec![])
    }

    pub(super) fn document_highlight(
        &mut self,
        uri: Url,
        position: Position,
    ) -> Vec<DocumentHighlight> {
        self.poll();

        assists::document_highlight::document_highlight(uri, position, &self.docs, &mut self.wa)
            .unwrap_or(vec![])
    }

    pub(super) fn hover(&mut self, uri: Url, position: Position) -> Option<Hover> {
        self.poll();

        assists::hover::hover(
            uri,
            position,
            &self.docs,
            &mut self.wa,
            &self.hsphelp_symbols,
        )
    }

    pub(super) fn references(
        &mut self,
        uri: Url,
        position: Position,
        include_definition: bool,
    ) -> Vec<Location> {
        self.poll();

        assists::references::references(uri, position, include_definition, &self.docs, &mut self.wa)
            .unwrap_or(vec![])
    }

    pub(super) fn prepare_rename(
        &mut self,
        uri: Url,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        self.poll();

        assists::rename::prepare_rename(uri, position, &self.docs, &mut self.wa)
    }

    pub(super) fn rename(
        &mut self,
        uri: Url,
        position: Position,
        new_name: String,
    ) -> Option<WorkspaceEdit> {
        self.poll();

        assists::rename::rename(uri, position, new_name, &self.docs, &mut self.wa)
    }

    pub(super) fn signature_help(&mut self, uri: Url, position: Position) -> Option<SignatureHelp> {
        self.poll();

        assists::signature_help::signature_help(uri, position, &self.docs, &mut self.wa)
    }

    pub(super) fn diagnose(&mut self) -> Vec<(Url, Option<i64>, Vec<Diagnostic>)> {
        if !self.options.lint_enabled {
            return vec![];
        }

        self.poll();

        assists::diagnose::diagnose(&self.docs, &mut self.wa)
    }
}
