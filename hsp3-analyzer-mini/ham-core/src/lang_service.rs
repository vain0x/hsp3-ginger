pub(crate) mod docs;
mod search_common;
pub(crate) mod search_hsphelp;

use self::{
    docs::{DocChange, Docs},
    source::DocId,
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
}

/// `LangService` の解析処理を完了した状態への参照
pub(super) struct LangServiceRef<'a> {
    wa: AnalysisRef<'a>,
    docs: &'a Docs,
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
        let mut ls = Self {
            // no_exist/hsp3
            hsp3_root: test_util::test_root_path().join("hsp3"),
            // no_exist/ws
            root_uri_opt: Some(
                CanonicalUri::from_file_path(&test_util::test_root_path().join("ws")).unwrap(),
            ),
            options: LangServiceOptions::minimal(),
            ..Default::default()
        };
        ls.wa.initialize(WorkspaceHost::default());
        ls
    }

    #[cfg(test)]
    pub(crate) fn analyze_for_test(&mut self) -> (AnalysisRef<'_>, &Docs) {
        self.process_changes();
        (self.wa.get_analysis(), &self.docs)
    }

    pub(super) fn watcher_enabled(&self) -> bool {
        self.options.watcher_enabled
    }

    pub(super) fn set_watchable(&mut self, watchable: bool) {
        if self.options.watcher_enabled {
            self.options.watcher_enabled = watchable;
        }
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

        debug!("scan_manifest_files");
        if let Some(root_dir) = self.root_uri_opt.as_ref().and_then(|x| x.to_file_path()) {
            project_model::scan::scan_manifest_files(&root_dir, |script_path| {
                let doc = match self.docs.ensure_file_opened(&script_path) {
                    Some(it) => it,
                    None => {
                        warn!("ファイルをopenできません。{:?}", script_path);
                        return;
                    }
                };
                entrypoints.push(doc);
            });
        }
        trace!(
            "entrypoints={:?}",
            entrypoints
                .iter()
                .filter_map(|&doc| self.docs.get_uri(doc).and_then(|uri| uri.to_file_path()))
                .collect::<Vec<_>>()
        );

        let entrypoints = if !entrypoints.is_empty() {
            EntryPoints::Docs(entrypoints)
        } else {
            EntryPoints::NonCommon
        };

        self.wa.initialize(WorkspaceHost {
            builtin_env: Rc::new(builtin_env),
            common_docs: Rc::new(common_docs),
            hsphelp_info: Rc::new(hsphelp_info),
            entrypoints,
        });

        debug!("scan_script_files");
        if let Some(root_dir) = self.root_uri_opt.as_ref().and_then(|x| x.to_file_path()) {
            project_model::scan::scan_script_files(&root_dir, |script_path| {
                self.docs.change_file(&script_path);
            });
        }
    }

    fn is_computed(&self) -> bool {
        !self.docs.has_changes() && self.wa.is_computed()
    }

    /// ドキュメントの変更を集積して、解析器の状態を更新する。
    fn process_changes(&mut self) {
        self.apply_doc_changes();
        self.wa.compute_analysis();
        assert!(self.is_computed());
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
                self.wa.set_project_docs(project_docs);
            }
        }
    }

    fn get_ref(&mut self) -> LangServiceRef<'_> {
        assert!(self.is_computed());
        LangServiceRef {
            wa: self.wa.get_analysis(),
            docs: &self.docs,
        }
    }

    /// 未実行の解析処理があれば処理し、解析処理を行うための参照を作る
    pub(crate) fn compute_ref(&mut self) -> LangServiceRef<'_> {
        self.process_changes();
        self.get_ref()
    }

    pub(super) fn shutdown(&mut self) {}

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
}

impl<'a> LangServiceRef<'a> {
    pub(super) fn code_action(
        &self,
        uri: Url,
        range: Range,
        _context: CodeActionContext,
    ) -> Vec<CodeAction> {
        let mut actions = vec![];
        actions.extend(
            assists::rewrites::flip_comma::flip_comma(&self.wa, &uri, range, &self.docs)
                .unwrap_or_default(),
        );
        actions.extend(
            assists::rewrites::generate_include_guard::generate_include_guard(
                &self.wa, &uri, range, &self.docs,
            )
            .unwrap_or_default(),
        );
        actions
    }

    pub(super) fn completion(&self, uri: Url, position: Position) -> CompletionList {
        assists::completion::completion(&self.wa, uri, position, &self.docs)
            .unwrap_or_else(assists::completion::incomplete_completion_list)
    }

    pub(super) fn completion_resolve(
        &self,
        completion_item: CompletionItem,
    ) -> Option<CompletionItem> {
        assists::completion::completion_resolve(&self.wa, completion_item, &self.docs)
    }

    pub(crate) fn formatting(&self, uri: Url) -> Option<Vec<TextEdit>> {
        assists::formatting::formatting(&self.wa, uri, &self.docs)
    }

    pub(super) fn definitions(&self, uri: Url, position: Position) -> Vec<Location> {
        assists::definitions::definitions(&self.wa, uri, position, &self.docs).unwrap_or(vec![])
    }

    pub(super) fn document_highlight(
        &self,
        uri: Url,
        position: Position,
    ) -> Vec<DocumentHighlight> {
        assists::document_highlight::document_highlight(&self.wa, uri, position, &self.docs)
            .unwrap_or(vec![])
    }

    pub(super) fn document_symbol(&self, uri: Url) -> Option<DocumentSymbolResponse> {
        assists::document_symbol::symbol(&self.wa, uri, &self.docs)
    }

    pub(super) fn hover(&self, uri: Url, position: Position) -> Option<Hover> {
        assists::hover::hover(&self.wa, uri, position, &self.docs)
    }

    pub(super) fn references(
        &self,
        uri: Url,
        position: Position,
        include_definition: bool,
    ) -> Vec<Location> {
        assists::references::references(&self.wa, uri, position, include_definition, &self.docs)
            .unwrap_or(vec![])
    }

    pub(super) fn prepare_rename(
        &self,
        uri: Url,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        assists::rename::prepare_rename(&self.wa, uri, position, &self.docs)
    }

    pub(super) fn rename(
        &self,
        uri: Url,
        position: Position,
        new_name: String,
    ) -> Option<WorkspaceEdit> {
        assists::rename::rename(&self.wa, uri, position, new_name, &self.docs)
    }

    pub(super) fn semantic_tokens(&self, uri: Url) -> lsp_types::SemanticTokens {
        let tokens = assists::semantic_tokens::full(&self.wa, uri, &self.docs).unwrap_or(vec![]);
        SemanticTokens {
            data: tokens,
            result_id: None,
        }
    }

    pub(super) fn signature_help(&self, uri: Url, position: Position) -> Option<SignatureHelp> {
        assists::signature_help::signature_help(&self.wa, uri, position, &self.docs)
    }

    pub(super) fn workspace_symbol(&self, query: String) -> Vec<SymbolInformation> {
        assists::workspace_symbol::symbol(&self.wa, &query, &self.docs)
    }
}

// TODO: implブロックを統合する
impl LangService {
    pub(super) fn diagnose(&mut self) -> Vec<(Url, Option<i32>, Vec<lsp_types::Diagnostic>)> {
        if !self.options.lint_enabled {
            return vec![];
        }

        self.process_changes();

        let mut diagnostics = assists::diagnose::diagnose(
            &self.wa.compute_analysis(),
            &self.docs,
            &mut self.diagnostics_cache,
        );

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

/// ドキュメントの管理機能を提供するもの
///
/// (`LangService` がドキュメントDBの役割を持つことを示している)
pub(crate) trait DocDb {
    fn get_doc_uri(&self, doc: DocId) -> Option<&CanonicalUri>;
    fn find_doc_by_uri(&self, uri: &CanonicalUri) -> Option<DocId>;
}

impl DocDb for LangService {
    fn get_doc_uri(&self, doc: DocId) -> Option<&CanonicalUri> {
        self.docs.get_uri(doc)
    }

    fn find_doc_by_uri(&self, uri: &CanonicalUri) -> Option<DocId> {
        self.docs.find_by_uri(uri)
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use std::path::PathBuf;

    pub(crate) fn test_root_path() -> PathBuf {
        if cfg!(target_os = "windows") {
            PathBuf::from("Z:/no_exist")
        } else {
            PathBuf::from("/.no_exist")
        }
    }
}
