use super::*;
use crate::source::DocId;

/// テキストドキュメントのバージョン番号
/// (エディタ上で編集されるたびに変わる番号。
///  いつの状態のテキストドキュメントを指しているかを明確にするためのもの。)
type TextDocumentVersion = i32;

pub(crate) const NO_VERSION: TextDocumentVersion = 1;

pub(crate) enum DocChange {
    Opened {
        doc: DocId,
        lang: Lang,
        origin: DocChangeOrigin,
    },
    Changed {
        doc: DocId,
        lang: Lang,
        origin: DocChangeOrigin,
    },
    Closed {
        doc: DocId,
    },
}

pub(crate) enum DocChangeOrigin {
    Editor(RcStr),
    Path(PathBuf),
}

/// テキストドキュメントを管理するもの。
///
/// - テキストドキュメントにはIDを振って管理する。(`DocId`)
/// - テキストドキュメントは2種類ある: エディタで開かれているものと、ファイルとして保存されているもの。
/// - エディタで開かれているドキュメントはURIで識別される。バージョン番号と内容は与えられる。
/// - ファイルであるドキュメントはファイルパスで指定される。内容は必要に応じて読み込む。
/// - ドキュメントがファイルであり、しかもエディタで開かれていることもある。
///     - これは同じIDを振る。ファイルの内容は無視する。
#[derive(Default)]
pub(crate) struct Docs {
    /// 最後に振ったID
    last_doc: usize,

    // ドキュメントの情報:
    doc_to_uri: HashMap<DocId, CanonicalUri>,
    uri_to_doc: HashMap<CanonicalUri, DocId>,
    doc_versions: HashMap<DocId, TextDocumentVersion>,

    /// エディタで開かれているドキュメント
    editor_docs: HashSet<DocId>,

    /// ファイルとして保存されているドキュメント
    file_docs: HashSet<DocId>,

    /// 最近の更新
    doc_changes: Vec<DocChange>,
}

impl Docs {
    pub(crate) fn fresh_doc(&mut self) -> DocId {
        self.last_doc += 1;
        self.last_doc
    }

    /// URIに対応するDocIdを探す。なければ作り、trueを返す。
    fn touch_uri(&mut self, uri: CanonicalUri) -> (bool, DocId) {
        match self.uri_to_doc.get(&uri) {
            Some(&doc) => (false, doc),
            None => {
                let doc = self.fresh_doc();
                self.doc_to_uri.insert(doc, uri.clone());
                self.uri_to_doc.insert(uri, doc);
                (true, doc)
            }
        }
    }

    pub(crate) fn find_by_uri(&self, uri: &CanonicalUri) -> Option<DocId> {
        self.uri_to_doc.get(uri).cloned()
    }

    pub(crate) fn get_uri(&self, doc: DocId) -> Option<&CanonicalUri> {
        self.doc_to_uri.get(&doc)
    }

    pub(crate) fn get_version(&self, doc: DocId) -> Option<TextDocumentVersion> {
        self.doc_versions.get(&doc).copied()
    }

    /// 指定したURIが指すディレクトリの子孫であるドキュメントを探す。
    pub(crate) fn get_docs_in(&self, uri: &CanonicalUri) -> ProjectDocs {
        // ファイル名 -> 同じ名前を持つドキュメントのIDのリスト
        let mut doc_env: HashMap<String, Vec<DocId>> = HashMap::new();
        // ディレクトリへの相対パス -> ディレクトリID
        let mut dir_env: HashMap<String, usize> = HashMap::new();
        // ドキュメント -> 親ディレクトリのID
        let mut doc_dirs: HashMap<DocId, usize> = HashMap::new();
        // ディレクトリID -> ディレクトリに含まれるドキュメントのIDのリスト
        let mut dirs: Vec<Vec<DocId>> = vec![];

        let base_dir = match uri.to_file_path() {
            Some(it) => it,
            None => return ProjectDocs::default(),
        };
        for (&doc, uri) in &self.doc_to_uri {
            (|| -> Option<()> {
                let absolute_path = uri.to_file_path()?;
                let relative_path = absolute_path.strip_prefix(&base_dir).ok()?;
                let dir = relative_path.parent()?.to_string_lossy().replace("\\", "/");
                let name = relative_path.file_name()?.to_string_lossy().to_string();

                let dir_id = *dir_env.entry(dir).or_insert_with(|| {
                    dirs.push(vec![]);
                    dirs.len() - 1
                });
                dirs[dir_id].push(doc);
                doc_dirs.insert(doc, dir_id);
                doc_env.entry(name).or_default().push(doc);
                Some(())
            })();
        }

        let dirs = dirs.into_iter().map(Rc::new).collect::<Vec<_>>();
        ProjectDocs {
            doc_dirs: doc_dirs
                .into_iter()
                .map(|(doc, dir_id)| (doc, dirs[dir_id].clone()))
                .collect(),
            doc_env,
        }
    }

    pub(crate) fn has_changes(&self) -> bool {
        !self.doc_changes.is_empty()
    }

    pub(crate) fn drain_doc_changes(&mut self, changes: &mut Vec<DocChange>) {
        changes.extend(self.doc_changes.drain(..));
    }

    fn do_open_doc(
        &mut self,
        doc: DocId,
        version: i32,
        lang: Lang,
        origin: DocChangeOrigin,
    ) -> DocId {
        self.doc_versions.insert(doc, version);
        self.doc_changes
            .push(DocChange::Opened { doc, lang, origin });
        doc
    }

    fn do_change_doc(&mut self, doc: DocId, version: i32, lang: Lang, origin: DocChangeOrigin) {
        self.doc_versions.insert(doc, version);
        self.doc_changes
            .push(DocChange::Changed { doc, lang, origin });
    }

    fn do_close_doc(&mut self, doc: DocId, uri: &CanonicalUri) {
        self.doc_to_uri.remove(&doc);
        self.uri_to_doc.remove(&uri);
        self.doc_versions.remove(&doc);
        self.doc_changes.push(DocChange::Closed { doc });
    }

    pub(crate) fn open_doc_in_editor(&mut self, uri: CanonicalUri, version: i32, text: RcStr) {
        #[cfg(trace_docs)]
        trace!(
            "クライアントでファイルが開かれました ({:?} version={}, len={})",
            uri,
            version,
            text.len()
        );

        let (created, doc) = self.touch_uri(uri);
        if created {
            self.do_open_doc(doc, version, Lang::Hsp3, DocChangeOrigin::Editor(text));
        } else {
            self.do_change_doc(doc, version, Lang::Hsp3, DocChangeOrigin::Editor(text));
        }

        self.editor_docs.insert(doc);
    }

    pub(crate) fn change_doc_in_editor(&mut self, uri: CanonicalUri, version: i32, text: RcStr) {
        #[cfg(trace_docs)]
        trace!(
            "クライアントでファイルが変更されました ({:?} version={}, len={})",
            uri,
            version,
            text.len()
        );

        let (created, doc) = self.touch_uri(uri);
        if created {
            self.do_open_doc(doc, version, Lang::Hsp3, DocChangeOrigin::Editor(text));
        } else {
            self.do_change_doc(doc, version, Lang::Hsp3, DocChangeOrigin::Editor(text));
        }

        self.editor_docs.insert(doc);
    }

    pub(crate) fn close_doc_in_editor(&mut self, uri: CanonicalUri) {
        #[cfg(trace_docs)]
        trace!("クライアントでファイルが閉じられました ({:?})", uri);

        let doc = match self.uri_to_doc.get(&uri) {
            Some(&doc) => doc,
            None => return,
        };

        self.editor_docs.remove(&doc);

        if !self.file_docs.contains(&doc) {
            self.do_close_doc(doc, &uri);
        }
    }

    fn do_change_file(&mut self, uri: CanonicalUri, path: &Path) -> Option<DocId> {
        let (created, doc) = self.touch_uri(uri);

        let open_in_editor = !created && self.editor_docs.contains(&doc);
        if open_in_editor {
            #[cfg(trace_docs)]
            trace!("ファイルは開かれているのでロードされません。");
            return Some(doc);
        }

        let lang = Lang::from_path(&path)?;
        let origin = DocChangeOrigin::Path(path.to_path_buf());
        let opened = self.file_docs.insert(doc);
        if opened {
            self.do_open_doc(doc, NO_VERSION, lang, origin);
        } else {
            self.do_change_doc(doc, NO_VERSION, lang, origin);
        }

        Some(doc)
    }

    pub(crate) fn change_file_by_uri(&mut self, uri: CanonicalUri) -> Option<DocId> {
        let path = uri.to_file_path()?;
        self.do_change_file(uri, &path)
    }

    pub(crate) fn change_file(&mut self, path: &Path) -> Option<DocId> {
        let uri = CanonicalUri::from_file_path(path)?;
        self.do_change_file(uri, &path)
    }

    pub(crate) fn close_file_by_uri(&mut self, uri: CanonicalUri) {
        let doc = match self.uri_to_doc.get(&uri) {
            Some(&doc) => doc,
            None => return,
        };

        self.file_docs.remove(&doc);

        if !self.editor_docs.contains(&doc) {
            self.do_close_doc(doc, &uri);
        }
    }

    /// ファイルとDocIdの対応付けを行う。
    pub(crate) fn ensure_file_opened(&mut self, path: &Path) -> Option<DocId> {
        self.change_file(path)
    }
}

#[derive(Default)]
pub(crate) struct ProjectDocs {
    /// ドキュメント -> それが属するディレクトリ
    /// (ディレクトリはそれに入っているドキュメントのリストで表す。)
    pub(crate) doc_dirs: HashMap<DocId, Rc<Vec<DocId>>>,

    /// ファイル名 -> その名前のドキュメント
    pub(crate) doc_env: HashMap<String, Vec<DocId>>,
}

impl ProjectDocs {
    /// 2つのドキュメントが同じディレクトリにある？
    fn peer(&self, d1: DocId, d2: DocId) -> bool {
        match (self.doc_dirs.get(&d1), self.doc_dirs.get(&d2)) {
            (Some(dir1), Some(dir2)) => Rc::ptr_eq(dir1, dir2),
            _ => false,
        }
    }

    /// ファイル名からドキュメントを探す。
    ///
    /// ディレクトリは無視して名前が一致するものを探す。
    /// ただし `base_opt = Some(doc)` であり `doc` と同じディレクトリにその名前のファイルがあったら、それを使う。
    pub(crate) fn find(&self, name: &str, base_opt: Option<DocId>) -> Option<DocId> {
        debug_assert!(!name.contains('\\'));

        let basename = match name.rfind('/') {
            Some(i) => &name[i + 1..],
            None => name,
        };

        if basename == "" || basename == "." || basename == ".." {
            return None;
        }

        let docs = self.doc_env.get(basename)?;

        if let Some(base) = base_opt {
            if let it @ Some(_) = docs.iter().find(|&&d| self.peer(base, d)).cloned() {
                return it;
            }
        }

        docs.first().cloned()
    }
}
