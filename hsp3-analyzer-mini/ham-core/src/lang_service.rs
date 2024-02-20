pub(crate) mod docs;
mod search_common;
pub(crate) mod search_hsphelp;

use self::{
    docs::{DocChange, Docs},
    search_hsphelp::HspHelpInfo,
    source::{DocId, Loc, Pos16},
    workspace_analysis::DocAnalysisMap,
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

// pub(crate) enum RootType {
//     Common,
//     HspHelp,
//     Workspace,
// }

#[derive(Clone, Copy)]
pub(crate) struct IsReadOnly(bool);

/// スクリプトが入っているディレクトリ群のルート
#[derive(Clone)]
pub(crate) struct SourceRoot(Rc<(PathBuf, IsReadOnly)>);

pub(crate) struct RelPath {
    base: SourceRoot,
    pathname: Rc<PathBuf>,
}

// GlobalState = RootDb + Server state
pub(crate) struct GlobalState {}

// input + analysis
pub(crate) struct RootDb {
    // input:
    pub(crate) builtin_env: Rc<SymbolEnv>,
    pub(crate) common_docs: Rc<HashMap<String, DocId>>,
    pub(crate) hsphelp_info: Rc<HspHelpInfo>,
    pub(crate) entrypoints: Vec<DocId>,

    // input state:
    pub(crate) dirty_docs: HashSet<DocId>,
    pub(crate) doc_texts: HashMap<DocId, (Lang, RcStr)>,

    pub(crate) docs: RootDocs,

    // すべてのドキュメントの解析結果を使って構築される情報:
    doc_analysis_map: DocAnalysisMap,
    module_map: ModuleMap,
    project1: ProjectAnalysis,
    project_opt: Option<ProjectAnalysis>,

    // computed:
    computed: Rc<RefCell<ComputedData>>,
}
impl RootDb {
    pub(crate) fn locate_symbol(&self, doc: DocId, pos: Pos16) -> Option<(SymbolRc, Loc)> {
        todo!()
    }
    pub(crate) fn collect_symbol_defs(&self, symbol: &SymbolRc, output: &mut Vec<Loc>) {
        todo!()
    }
    pub(crate) fn collect_symbol_uses(&self, symbol: &SymbolRc, output: &mut Vec<Loc>) {
        todo!()
    }
}

pub(crate) struct RootDocs;
impl RootDocs {
    pub(crate) fn find_by_uri(&self, uri: &CanonicalUri) -> Option<DocId> {
        todo!()
    }

    pub(crate) fn get_uri(&self, doc: DocId) -> Option<CanonicalUri> {
        todo!()
    }
}

// pub(crate) struct Computed<T> {
//     cell_rc: Rc<RefCell<T>>,
// }

pub(crate) struct ComputedData {
    doc_analysis_map: DocAnalysisMap,
    module_map: ModuleMap,
    project1: ProjectAnalysis,
    project_opt: Option<ProjectAnalysis>,
}

// config
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
    // config(1)
    hsp3_root: PathBuf,
    // workspaces
    root_uri_opt: Option<CanonicalUri>,
    // config
    options: LangServiceOptions,
    // mem_docs
    docs: Docs,
    // diagnostics_collection
    diagnostics_cache: DiagnosticsCache,
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
        let mut entrypoints;

        search_common(&self.hsp3_root, &mut self.docs, &mut common_docs);

        let hsphelp_info = search_hsphelp(
            &self.hsp3_root,
            &common_docs,
            &mut self.docs,
            &mut builtin_env,
        )
        .unwrap_or_default();

        info!("ルートディレクトリからgingerプロジェクトファイルを収集します。");
        entrypoints = match self.root_uri_opt.as_ref().and_then(|x| x.to_file_path()) {
            Some(root_dir) => {
                let files = analysis::collect::collect_project_files(root_dir);
                files
                    .into_iter()
                    .filter_map(|name| {
                        let doc = match self.docs.ensure_file_opened(&name) {
                            Some(it) => it,
                            None => {
                                warn!("ファイルをopenできません。{:?}", name);
                                return None;
                            }
                        };
                        Some(doc)
                    })
                    .collect::<Vec<_>>()
            }
            None => vec![],
        };

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
    }

    /// ドキュメントの変更を集積して、解析器の状態を更新する。
    fn poll(&mut self) {
        self.apply_doc_changes();
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

    pub(super) fn semantic_tokens(&mut self, uri: Url) -> lsp_types::SemanticTokens {
        self.poll();

        let tokens =
            assists::semantic_tokens::full(uri, &self.docs, &mut self.wa).unwrap_or(vec![]);
        SemanticTokens {
            data: tokens,
            result_id: None,
        }
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
