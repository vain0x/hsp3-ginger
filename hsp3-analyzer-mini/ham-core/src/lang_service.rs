pub(crate) mod docs;
pub(crate) mod file_watcher;

use self::{
    docs::{DocChange, Docs},
    file_watcher::FileWatcher,
};
use super::*;
use crate::{
    analysis::integrate::HostData,
    analysis::*,
    assists::{self, diagnose::DiagnosticsCache},
    help_source::{collect_all_symbols, HsSymbol},
    lang_service::docs::DocChangeOrigin,
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
    wa: AWorkspaceAnalysis,
    hsp3_home: PathBuf,
    root_uri_opt: Option<CanonicalUri>,
    options: LangServiceOptions,
    docs: Docs,
    diagnostics_cache: DiagnosticsCache,
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
        let mut builtin_env = SymbolEnv::default();
        let mut common_docs = HashMap::new();
        let mut entrypoints = vec![];

        info!("hsphelp ファイルからシンボルを探索します。");
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
            .map(|symbol| {
                let kind = CompletionItemKind::Function;
                let HsSymbol {
                    name,
                    description,
                    documentation,
                    signature_opt,
                    mut param_info,
                } = symbol;

                let name_rc = RcStr::from(name.clone());

                let signature_opt = signature_opt.map(|s| {
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

                    Rc::new(ASignatureData {
                        name: name_rc.clone(),
                        params,
                    })
                });

                let symbol = ASymbol::from(ASymbolData {
                    doc: hsphelp_doc,
                    kind: ASymbolKind::Unknown,
                    name: name_rc.clone(),
                    leader_opt: None,
                    scope_opt: None,
                    ns_opt: None,
                    details_opt: Some(ASymbolDetails {
                        desc: description.clone().map(RcStr::from),
                        docs: documentation.clone(),
                    }),
                    preproc_def_site_opt: None,
                    signature_opt: RefCell::new(signature_opt),
                });
                builtin_env.insert(name_rc.clone(), symbol);

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
                    label: name,
                    detail: description,
                    documentation: if documentation.is_empty() {
                        None
                    } else {
                        Some(Documentation::String(documentation.join("\r\n\r\n")))
                    },
                    sort_text: Some(format!("{}{}", sort_prefix, name_rc)),
                    filter_text: word.clone(),
                    insert_text: word,
                    ..Default::default()
                }
            })
            .collect();

        info!("common ディレクトリからシンボルを探索します。");
        {
            let common_dir = self.hsp3_home.join("common");

            let patterns = match common_dir.to_str() {
                Some(dir) => vec![format!("{}/**/*.hsp", dir), format!("{}/**/*.as", dir)],
                None => vec![],
            };

            for path in patterns
                .into_iter()
                .flat_map(|pattern| glob::glob(&pattern).unwrap())
                .flat_map(|result| result.ok())
            {
                if let Some(uri) = CanonicalUri::from_file_path(&path) {
                    let mut contents = String::new();
                    if !read_file(&path, &mut contents) {
                        warn!("cannot read {:?}", path);
                        continue;
                    };
                    self.open_doc(uri.clone().into_url(), 1, contents);

                    let doc = self.docs.find_by_uri(&uri).unwrap();

                    (|| -> Option<()> {
                        let relative = path
                            .strip_prefix(&common_dir)
                            .ok()?
                            .components()
                            .map(|c| match c {
                                path::Component::Normal(s) => s.to_str(),
                                _ => None,
                            })
                            .collect::<Option<Vec<&str>>>()?
                            .join("/");

                        common_docs.insert(relative, doc);

                        None
                    })();
                }
            }
        }

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

        self.wa.initialize(HostData {
            builtin_env,
            common_docs,
            entrypoints,
        });

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
        let changed = !doc_changes.is_empty();

        for change in doc_changes.drain(..) {
            match change {
                DocChange::Opened { doc, origin } | DocChange::Changed { doc, origin } => {
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

                    self.wa.update_doc(doc, text);
                }
                DocChange::Closed { doc } => {
                    self.wa.close_doc(doc);
                }
            }
        }

        if changed {
            if let Some(root_uri) = &self.root_uri_opt {
                let project_docs = self.docs.get_docs_in(root_uri);
                self.wa.set_project_docs(project_docs);
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

        self.docs.open_doc_in_editor(uri, version, text.into());
    }

    pub(super) fn change_doc(&mut self, uri: Url, version: i64, text: String) {
        let uri = CanonicalUri::from_url(&uri);

        self.docs.change_doc_in_editor(uri, version, text.into());
    }

    pub(super) fn close_doc(&mut self, uri: Url) {
        let uri = CanonicalUri::from_url(&uri);

        self.docs.close_doc_in_editor(uri);
    }

    pub(super) fn code_action(
        &mut self,
        uri: Url,
        range: Range,
        context: CodeActionContext,
    ) -> Vec<CodeAction> {
        self.poll();

        assists::rewrites::flip_comma::flip_comma(uri, range, context, &self.docs, &mut self.wa)
            .unwrap_or_default()
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

        let mut diagnostics =
            assists::diagnose::diagnose(&self.docs, &mut self.diagnostics_cache, &mut self.wa);

        // hsp3のファイルにdiagnosticsを出さない。
        diagnostics.retain(|(uri, _, _)| {
            let ok = uri
                .to_file_path()
                .map_or(true, |path| !path.starts_with(&self.hsp3_home));

            if !ok {
                trace!(
                    "ファイルはhsp3_homeにあるので {:?} への診断は無視されます。",
                    uri
                );
            }

            ok
        });
        diagnostics
    }
}
