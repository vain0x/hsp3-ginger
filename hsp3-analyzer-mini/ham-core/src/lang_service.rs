pub(crate) mod docs;
pub(crate) mod file_watcher;
mod search_common;
pub(crate) mod search_hsphelp;

use self::{
    docs::{DocChange, Docs},
    file_watcher::FileWatcher,
};
use super::*;
use crate::{
    analysis::*,
    assists::{self, diagnose::DiagnosticsCache},
    help_source::HsSymbol,
    lang::Lang,
    lang_service::{
        docs::DocChangeOrigin, search_common::search_common, search_hsphelp::search_hsphelp,
    },
    utils::read_file::read_file,
};
use lsp_types::*;

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
    wa: WorkspaceAnalysis,
    hsp3_root: PathBuf,
    root_uri_opt: Option<CanonicalUri>,
    options: LangServiceOptions,
    docs: Docs,
    diagnostics_cache: DiagnosticsCache,
    file_watcher_opt: Option<FileWatcher>,

    watchable: bool,
}

impl LangService {
    pub(super) fn new(hsp3_root: PathBuf, options: LangServiceOptions) -> Self {
        Self {
            hsp3_root,
            options,
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn new_standalone() -> Self {
        let hsp3_root = PathBuf::from("/tmp/.not_exist");
        let options = LangServiceOptions::minimal();

        let mut ls = Self {
            hsp3_root,
            options,
            ..Default::default()
        };
        ls.wa.initialize(WorkspaceHost::default());
        ls
    }

    pub(super) fn set_watchable(&mut self, watchable: bool) {
        self.watchable = watchable;
    }

    pub(super) fn initialize(&mut self, root_uri_opt: Option<Url>) {
        if let Some(uri) = root_uri_opt {
            self.root_uri_opt = Some(CanonicalUri::from_url(&uri));
        }
    }

    pub(super) fn did_initialize(&mut self) {
        let mut builtin_env = SymbolEnv::default();
        let mut common_docs = HashMap::new();
        let mut entrypoints = vec![];

        search_common(&self.hsp3_root, &mut self.docs, &mut common_docs);

        let hsphelp_info = search_hsphelp(
            &self.hsp3_root,
            &common_docs,
            &mut self.docs,
            &mut builtin_env,
        )
        .unwrap_or_default();

        info!("ルートディレクトリからgingerプロジェクトファイルを収集します。");
        {
            let root_dir_opt = self.root_uri_opt.as_ref().and_then(|x| x.to_file_path());
            let project_files = root_dir_opt
                .into_iter()
                .filter_map(|dir| glob::glob(&format!("{}/**/ginger.txt", dir.to_str()?)).ok())
                .flatten()
                .filter_map(|path_opt| path_opt.ok())
                .filter_map(|path| Some((path.clone(), fs::read_to_string(&path).ok()?)));
            for (path, contents) in project_files {
                let dir = path.parent();
                let docs = contents
                    .lines()
                    .enumerate()
                    .map(|(i, line)| (i, line.trim_end()))
                    .filter(|&(_, line)| line != "")
                    .filter_map(|(i, name)| {
                        let name = dir?.join(name);
                        if !name.exists() {
                            warn!("ファイルがありません {:?}:{}", path, i);
                            return None;
                        }

                        let doc = match self.docs.ensure_file_opened(&name) {
                            Some(it) => it,
                            None => {
                                warn!("ファイルをopenできません。{:?}", name);
                                return None;
                            }
                        };
                        Some(doc)
                    });

                entrypoints.extend(docs);
            }

            trace!(
                "entrypoints={:?}",
                entrypoints
                    .iter()
                    .map(|&doc| self.docs.get_uri(doc).ok_or(doc))
                    .collect::<Vec<_>>()
            );
        }

        self.wa.initialize(WorkspaceHost {
            builtin_env: Rc::new(builtin_env),
            common_docs: Rc::new(common_docs),
            hsphelp_info: Rc::new(hsphelp_info),
            entrypoints,
        });

        info!("ルートディレクトリからスクリプトファイルを収集します。");
        {
            let root_dir_opt = self.root_uri_opt.as_ref().and_then(|x| x.to_file_path());
            let script_files = root_dir_opt
                .into_iter()
                .filter_map(|dir| glob::glob(&format!("{}/**/*.hsp", dir.to_str()?)).ok())
                .flatten()
                .filter_map(|path_opt| path_opt.ok());
            for path in script_files {
                self.docs.change_file(&path);
            }
        }

        // if self.options.watcher_enabled {
        //     if let Some(watched_dir) = self
        //         .root_uri_opt
        //         .as_ref()
        //         .and_then(|uri| uri.to_file_path())
        //     {
        //         let mut watcher = FileWatcher::new(watched_dir);
        //         watcher.start_watch();
        //         self.file_watcher_opt = Some(watcher);
        //     }
        // }
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

        let opened_or_closed = doc_changes.iter().any(|change| match change {
            DocChange::Opened { .. } | DocChange::Closed { .. } => true,
            _ => false,
        });

        for change in doc_changes.drain(..) {
            match change {
                DocChange::Opened { doc, lang, origin }
                | DocChange::Changed { doc, lang, origin } => {
                    let text = match origin {
                        DocChangeOrigin::Editor(text) => text,
                        DocChangeOrigin::Path(path) => {
                            let mut text = String::new();
                            if !read_file(&path, &mut text) {
                                warn!("ファイルを開けません。{:?}", path);
                                continue;
                            }
                            text.into()
                        }
                    };

                    self.wa.update_doc(doc, lang, text);
                }
                DocChange::Closed { doc } => {
                    self.wa.close_doc(doc);
                }
            }
        }

        if opened_or_closed {
            if let Some(root_uri) = &self.root_uri_opt {
                let project_docs = self.docs.get_docs_in(root_uri);
                self.wa.set_project_docs(Rc::new(project_docs));
            }
        }
    }

    pub(super) fn shutdown(&mut self) {
        if let Some(mut watcher) = self.file_watcher_opt.take() {
            watcher.stop_watch();
        }
    }

    pub(super) fn open_doc(&mut self, uri: Url, version: i32, text: String) {
        let uri = CanonicalUri::from_url(&uri);

        self.docs.open_doc_in_editor(uri, version, text.into());
    }

    pub(super) fn change_doc(&mut self, uri: Url, version: i32, text: String) {
        let uri = CanonicalUri::from_url(&uri);

        self.docs.change_doc_in_editor(uri, version, text.into());
    }

    pub(super) fn close_doc(&mut self, uri: Url) {
        let uri = CanonicalUri::from_url(&uri);

        self.docs.close_doc_in_editor(uri);
    }

    pub(super) fn on_file_created(&mut self, uri: Url) {
        let uri = CanonicalUri::from_url(&uri);
        self.docs.change_file_by_uri(uri);
    }

    pub(super) fn on_file_changed(&mut self, uri: Url) {
        let uri = CanonicalUri::from_url(&uri);
        self.docs.change_file_by_uri(uri);
    }

    pub(super) fn on_file_deleted(&mut self, uri: Url) {
        let uri = CanonicalUri::from_url(&uri);
        self.docs.close_file_by_uri(uri);
    }

    pub(super) fn code_action(
        &mut self,
        uri: Url,
        range: Range,
        _context: CodeActionContext,
    ) -> Vec<CodeAction> {
        self.poll();

        let mut actions = vec![];
        actions.extend(
            assists::rewrites::flip_comma::flip_comma(&uri, range, &self.docs, &mut self.wa)
                .unwrap_or_default(),
        );
        actions.extend(
            assists::rewrites::generate_include_guard::generate_include_guard(
                &uri,
                range,
                &self.docs,
                &mut self.wa,
            )
            .unwrap_or_default(),
        );
        actions
    }

    pub(super) fn completion(&mut self, uri: Url, position: Position) -> CompletionList {
        self.poll();

        assists::completion::completion(uri, position, &self.docs, &mut self.wa)
            .unwrap_or_else(assists::completion::incomplete_completion_list)
    }

    pub(super) fn completion_resolve(
        &mut self,
        completion_item: CompletionItem,
    ) -> Option<CompletionItem> {
        assists::completion::completion_resolve(completion_item, &self.docs, &mut self.wa)
    }

    pub(crate) fn formatting(&mut self, uri: Url) -> Option<Vec<TextEdit>> {
        self.poll();

        assists::formatting::formatting(uri, &self.docs, &mut self.wa)
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

    pub(super) fn document_symbol(&mut self, uri: Url) -> Option<DocumentSymbolResponse> {
        self.poll();

        assists::document_symbol::symbol(uri, &self.docs, &mut self.wa)
    }

    pub(super) fn hover(&mut self, uri: Url, position: Position) -> Option<Hover> {
        self.poll();

        assists::hover::hover(uri, position, &self.docs, &mut self.wa)
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

    pub(super) fn workspace_symbol(&mut self, query: String) -> Vec<SymbolInformation> {
        self.poll();

        assists::workspace_symbol::symbol(&query, &self.docs, &mut self.wa)
    }

    pub(super) fn diagnose(&mut self) -> Vec<(Url, Option<i32>, Vec<lsp_types::Diagnostic>)> {
        if !self.options.lint_enabled {
            return vec![];
        }

        self.poll();

        let mut diagnostics =
            assists::diagnose::diagnose(&self.docs, &mut self.diagnostics_cache, &mut self.wa);

        // hsp3のファイルにdiagnosticsを出さない。
        diagnostics.retain(|(uri, _, _)| {
            let ok = uri
                .to_file_path()
                .map_or(true, |path| !path.starts_with(&self.hsp3_root));

            if !ok {
                trace!(
                    "ファイルはhsp3_rootにあるので {:?} への診断は無視されます。",
                    uri
                );
            }

            ok
        });
        diagnostics
    }
}
