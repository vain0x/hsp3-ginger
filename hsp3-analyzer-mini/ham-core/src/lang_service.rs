pub(crate) mod doc_change;
pub(crate) mod doc_interner;
pub(crate) mod docs;
mod search_common;
pub(crate) mod search_hsphelp;

use super::*;
use crate::{
    analysis::*,
    help_source::HsSymbol,
    ide::{self, diagnose::DiagnosticsCache},
    lang::Lang,
    lang_service::{
        doc_change::{DocChange, DocChangeOrigin},
        doc_interner::DocInterner,
        docs::Docs,
        search_common::search_common,
        search_hsphelp::{search_hsphelp, HspHelpInfo},
    },
    source::{DocId, Loc},
    utils::read_file::read_file,
};
use lsp_types::*;

pub(crate) struct LangServiceOptions {
    pub(crate) lint_enabled: bool,
}

impl LangServiceOptions {
    #[cfg(test)]
    pub(crate) fn minimal() -> Self {
        Self {
            lint_enabled: false,
        }
    }
}

impl Default for LangServiceOptions {
    fn default() -> Self {
        Self { lint_enabled: true }
    }
}

#[derive(Default)]
pub(super) struct LangService {
    // 入力 (起動時):
    hsp3_root: PathBuf,
    root_uri_opt: Option<CanonicalUri>,
    options: LangServiceOptions,

    // 状態 (ファイルスキャンの結果):
    common_docs: Rc<HashMap<String, DocId>>,
    hsphelp_info: Rc<HspHelpInfo>,

    // 状態 (ドキュメント):
    doc_interner: DocInterner,
    docs: Docs,

    // 状態 (ドキュメントごとの解析処理の計算結果):
    doc_analysis_map: DocAnalysisMap,

    // 状態 (ドキュメント全体の解析処理の計算結果):
    active_docs: HashSet<DocId>,
    active_help_docs: HashSet<DocId>,
    help_docs: HashMap<DocId, DocId>,
    /// (loc, doc): locにあるincludeがdocに解決されたことを表す (FIXME: 再実装)
    include_resolution: Vec<(Loc, DocId)>,
    public_env: PublicEnv,
    ns_env: HashMap<RcStr, SymbolEnv>,
    def_sites: Vec<(SymbolRc, Loc)>,
    use_sites: Vec<(SymbolRc, Loc)>,

    // 状態 (上記の計算結果をさらに加工したもの):
    doc_symbols_map: HashMap<DocId, Vec<SymbolRc>>,
    module_map: ModuleMap,

    // 状態 (計算結果の差分を生成するため):
    diagnostics_cache: RefCell<DiagnosticsCache>,
}

/// `LangService` の解析処理を完了した状態への参照
pub(super) struct LangServiceRef<'a> {
    wa: AnalysisRef<'a>,
    doc_interner: &'a DocInterner,
    docs: &'a Docs,
    owner: &'a LangService,
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
        let root = crate::test_utils::dummy_path();
        let ls = Self {
            // no_exist/hsp3
            hsp3_root: root.clone().join("hsp3"),
            // no_exist/ws
            root_uri_opt: Some(CanonicalUri::from_abs_path(&root.join("ws")).unwrap()),
            options: LangServiceOptions::minimal(),
            ..Default::default()
        };

        // 既定値を使う:
        // self.common_docs = common_docs;
        // self.hsphelp_info = hsphelp_info;
        // self.public_env.builtin = builtin_env;

        ls
    }

    #[cfg(test)]
    pub(crate) fn analyze_for_test(&mut self) -> (AnalysisRef<'_>, &DocInterner, &Docs) {
        if !self.is_computed() {
            self.process_changes();
        }
        (self.get_analysis(), &self.doc_interner, &self.docs)
    }

    fn get_analysis(&self) -> AnalysisRef<'_> {
        assert!(self.is_computed());
        AnalysisRef {
            common_docs: &self.common_docs,
            hsphelp_info: &self.hsphelp_info,
            active_docs: &self.active_docs,
            active_help_docs: &self.active_help_docs,
            doc_symbols_map: &self.doc_symbols_map,
            def_sites: &self.def_sites,
            use_sites: &self.use_sites,
            doc_analysis_map: &self.doc_analysis_map,
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

        search_common(
            &self.hsp3_root,
            &mut self.doc_interner,
            &mut self.docs,
            &mut common_docs,
        );

        let hsphelp_info = search_hsphelp(
            &self.hsp3_root,
            &common_docs,
            &mut self.doc_interner,
            &mut self.docs,
            &mut builtin_env,
        )
        .unwrap_or_default();

        self.public_env.builtin = Rc::new(builtin_env);
        self.common_docs = Rc::new(common_docs);
        self.hsphelp_info = Rc::new(hsphelp_info);

        debug!("scan_script_files");
        if let Some(root_dir) = self.root_uri_opt.as_ref().and_then(|x| x.to_file_path()) {
            project_model::scan::scan_script_files(&root_dir, |script_path| {
                if let Some(uri) = CanonicalUri::from_abs_path(&script_path) {
                    let (_, doc) = self.doc_interner.intern(&uri);
                    self.docs.change_file(doc, &script_path);
                }
            });
        }
    }

    fn is_computed(&self) -> bool {
        !self.docs.has_changes()
    }

    /// ドキュメントの変更を集積して、解析器の状態を更新する。
    fn process_changes(&mut self) {
        debug_assert!(!self.is_computed());

        let mut doc_changes = vec![];
        self.docs.drain_doc_changes(&mut doc_changes);

        // let opened_or_closed = doc_changes.iter().any(|change| match change {
        //     DocChange::Opened { .. } | DocChange::Closed { .. } => true,
        //     _ => false,
        // });

        // 同じドキュメントに対する変更をまとめる
        let mut change_map = HashMap::new();
        for change in doc_changes.drain(..) {
            let doc = match change {
                DocChange::Opened { doc, .. }
                | DocChange::Changed { doc, .. }
                | DocChange::Closed { doc } => doc,
            };
            change_map.insert(doc, change);
        }

        // ドキュメントごとの変更を適用する
        // (開かれた・変更されたドキュメントに対して再解析を行い、
        //  閉じられたドキュメントに関するデータを削除する)
        for (_, change) in change_map.drain() {
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

                    match lang {
                        // .hs ファイルの変更追跡は未実装
                        Lang::HelpSource => continue,
                        Lang::Hsp3 => {}
                    }

                    let da = self.doc_analysis_map.entry(doc).or_default();
                    da.compute(doc, text);
                }
                DocChange::Closed { doc } => {
                    self.doc_analysis_map.remove(&doc);
                }
            }
        }

        // invalidate include graph
        // if opened_or_closed {
        //     if let Some(root_uri) = &self.root_uri_opt {
        //         ...
        //     }
        // }

        // ドキュメント全体に対する解析処理を再実行する
        {
            self.active_docs.clear();
            self.active_help_docs.clear();
            self.help_docs.clear();
            self.include_resolution.clear();
            self.public_env.clear();
            self.ns_env.clear();
            self.doc_symbols_map.clear();
            self.def_sites.clear();
            self.use_sites.clear();
            self.module_map.clear();

            for da in self.doc_analysis_map.values() {
                self.module_map
                    .extend(da.module_map.iter().map(|(&m, rc)| (m, rc.clone())));
            }

            compute_active_docs::compute_active_docs(
                &self.doc_analysis_map,
                &self.common_docs,
                &self.hsphelp_info,
                &mut self.active_docs,
                &mut self.active_help_docs,
                &mut self.help_docs,
                &mut self.include_resolution,
            );

            compute_symbols::compute_symbols(
                &self.hsphelp_info,
                &self.active_docs,
                &self.help_docs,
                &self.doc_analysis_map,
                &self.module_map,
                &mut self.public_env,
                &mut self.ns_env,
                &mut self.doc_symbols_map,
                &mut self.def_sites,
                &mut self.use_sites,
            );

            // デバッグ用: 集計を出す。
            {
                let total_symbol_count = self
                    .doc_symbols_map
                    .values()
                    .map(|symbols| symbols.len())
                    .sum::<usize>();
                trace!(
                    "computed: active_docs={} def_sites={} use_sites={} symbols={}",
                    self.active_docs.len(),
                    self.def_sites.len(),
                    self.use_sites.len(),
                    total_symbol_count
                );
            }
        }

        debug_assert!(self.is_computed());
    }

    fn get_ref(&self) -> LangServiceRef<'_> {
        assert!(self.is_computed());
        LangServiceRef {
            owner: self,
            wa: self.get_analysis(),
            doc_interner: &self.doc_interner,
            docs: &self.docs,
        }
    }

    /// 未実行の解析処理があれば処理し、解析処理を行うための参照を作る
    pub(crate) fn compute_ref(&mut self) -> LangServiceRef<'_> {
        if !self.is_computed() {
            self.process_changes();
        }
        self.get_ref()
    }

    pub(super) fn shutdown(&mut self) {}

    pub(super) fn open_doc(&mut self, uri: Url, version: i32, text: String) {
        let c_uri = CanonicalUri::from_url(&uri);
        let (_, doc) = self.doc_interner.intern(&c_uri);
        self.docs.open_doc_in_editor(doc, version, text.into());
    }

    pub(super) fn change_doc(&mut self, uri: Url, version: i32, text: String) {
        let c_uri = CanonicalUri::from_url(&uri);
        let (_, doc) = self.doc_interner.intern(&c_uri);
        self.docs.change_doc_in_editor(doc, version, text.into());
    }

    pub(super) fn close_doc(&mut self, uri: Url) {
        let c_uri = CanonicalUri::from_url(&uri);
        if let Some(doc) = self.doc_interner.get_doc(&c_uri) {
            if self.docs.close_doc_in_editor(doc) {
                self.doc_interner.remove(doc, &c_uri);
            }
        }
    }

    pub(super) fn on_file_created(&mut self, url: Url) {
        let c_uri = CanonicalUri::from_url(&url);
        let (_, doc) = self.doc_interner.intern(&c_uri);
        if let Some(path) = c_uri.to_file_path() {
            self.docs.change_file(doc, &path);
        }
    }

    pub(super) fn on_file_changed(&mut self, uri: Url) {
        let c_uri = CanonicalUri::from_url(&uri);
        let (_, doc) = self.doc_interner.intern(&c_uri);
        if let Some(path) = c_uri.to_file_path() {
            self.docs.change_file(doc, &path);
        }
    }

    pub(super) fn on_file_deleted(&mut self, uri: Url) {
        let c_uri = CanonicalUri::from_url(&uri);
        if let Some(doc) = self.doc_interner.get_doc(&c_uri) {
            if self.docs.close_file(doc) {
                self.doc_interner.remove(doc, &c_uri);
            }
        }
    }
}

impl<'a> LangServiceRef<'a> {
    #[cfg(test)]
    pub(crate) fn get_analysis_ref(&self) -> &AnalysisRef<'_> {
        &self.wa
    }

    pub(super) fn code_action(
        &self,
        uri: Url,
        range: Range,
        _context: CodeActionContext,
    ) -> Vec<CodeAction> {
        let mut actions = vec![];
        actions.extend(
            ide::code_actions::flip_comma::flip_comma(
                &self.wa,
                self.doc_interner,
                self.docs,
                &uri,
                range,
            )
            .unwrap_or_default(),
        );
        actions.extend(
            ide::code_actions::generate_include_guard::generate_include_guard(
                &self.wa,
                self.doc_interner,
                self.docs,
                &uri,
                range,
            )
            .unwrap_or_default(),
        );
        actions
    }

    pub(super) fn completion(&self, uri: Url, position: Position) -> CompletionList {
        ide::completion::completion(&self.wa, self.doc_interner, uri, position)
            .unwrap_or_else(ide::completion::incomplete_completion_list)
    }

    pub(super) fn completion_resolve(
        &self,
        completion_item: CompletionItem,
    ) -> Option<CompletionItem> {
        ide::completion::completion_resolve(&self.wa, self.doc_interner, completion_item)
    }

    pub(crate) fn formatting(&self, uri: Url) -> Option<Vec<TextEdit>> {
        ide::formatting::formatting(&self.wa, self.doc_interner, uri)
    }

    pub(super) fn definitions(&self, uri: Url, position: Position) -> Vec<Location> {
        ide::definitions::definitions(&self.wa, self.doc_interner, uri, position).unwrap_or(vec![])
    }

    pub(super) fn document_highlight(
        &self,
        uri: Url,
        position: Position,
    ) -> Vec<DocumentHighlight> {
        ide::document_highlight::document_highlight(&self.wa, self.doc_interner, uri, position)
            .unwrap_or(vec![])
    }

    pub(super) fn document_symbol(&self, uri: Url) -> Option<DocumentSymbolResponse> {
        ide::document_symbol::symbol(&self.wa, self.doc_interner, uri)
    }

    pub(super) fn hover(&self, uri: Url, position: Position) -> Option<Hover> {
        ide::hover::hover(&self.wa, self.doc_interner, uri, position)
    }

    pub(super) fn references(
        &self,
        uri: Url,
        position: Position,
        include_definition: bool,
    ) -> Vec<Location> {
        ide::references::references(
            &self.wa,
            self.doc_interner,
            uri,
            position,
            include_definition,
        )
        .unwrap_or(vec![])
    }

    pub(super) fn prepare_rename(
        &self,
        uri: Url,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        ide::rename::prepare_rename(&self.wa, self.doc_interner, uri, position)
    }

    pub(super) fn rename(
        &self,
        uri: Url,
        position: Position,
        new_name: String,
    ) -> Option<WorkspaceEdit> {
        ide::rename::rename(
            &self.wa,
            self.doc_interner,
            self.docs,
            uri,
            position,
            new_name,
        )
    }

    pub(super) fn semantic_tokens(&self, uri: Url) -> lsp_types::SemanticTokens {
        let tokens = ide::semantic_tokens::full(&self.wa, self.doc_interner, uri).unwrap_or(vec![]);
        SemanticTokens {
            data: tokens,
            result_id: None,
        }
    }

    pub(super) fn signature_help(&self, uri: Url, position: Position) -> Option<SignatureHelp> {
        ide::signature_help::signature_help(&self.wa, self.doc_interner, uri, position)
    }

    pub(super) fn workspace_symbol(&self, query: String) -> Vec<SymbolInformation> {
        ide::workspace_symbol::symbol(&self.wa, self.doc_interner, &query)
    }

    pub(super) fn diagnose(&self) -> Vec<(Url, Option<i32>, Vec<lsp_types::Diagnostic>)> {
        if !self.owner.options.lint_enabled {
            return vec![];
        }

        let mut diagnostics = ide::diagnose::diagnose(
            &self.wa,
            &self.doc_interner,
            &self.docs,
            &mut self.owner.diagnostics_cache.borrow_mut(),
        );

        // hsp3のファイルにdiagnosticsを出さない。
        diagnostics.retain(|(uri, _, _)| {
            let ok = uri
                .to_file_path()
                .map_or(true, |path| !path.starts_with(&self.owner.hsp3_root));

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
#[allow(unused)]
pub(crate) trait DocDb {
    // fn get_doc_uri(&self, doc: DocId) -> Option<&CanonicalUri>;
    fn find_doc_by_uri(&self, uri: &CanonicalUri) -> Option<DocId>;
}

impl DocDb for LangService {
    // fn get_doc_uri(&self, doc: DocId) -> Option<&CanonicalUri> {
    //     self.doc_interner.get_uri(doc)
    // }

    fn find_doc_by_uri(&self, uri: &CanonicalUri) -> Option<DocId> {
        self.doc_interner.get_doc(uri)
    }
}
