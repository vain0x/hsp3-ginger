use super::*;
use crate::source::DocId;

/// テキストドキュメントのバージョン番号
/// (エディタ上で編集されるたびに変わる番号。
///  いつの状態のテキストドキュメントを指しているかを明確にするためのもの。)
type TextDocumentVersion = i32;

pub(crate) const NO_VERSION: TextDocumentVersion = 1;

pub(crate) enum DocChange {
    Opened { doc: DocId, origin: DocChangeOrigin },
    Changed { doc: DocId, origin: DocChangeOrigin },
    Closed { doc: DocId },
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
    pub(crate) fn get_docs_in(&self, uri: &CanonicalUri) -> HashMap<String, DocId> {
        let mut map = HashMap::new();
        if let Some(root) = uri.to_file_path() {
            for (&doc, uri) in &self.doc_to_uri {
                let path = match uri.to_file_path() {
                    Some(it) => it,
                    None => continue,
                };

                let result = path.strip_prefix(&root);
                let entry = result
                    .as_ref()
                    .ok()
                    .map(|path| (path.to_string_lossy().replace("\\", "/"), doc));
                map.extend(entry);
            }
        }
        map
    }

    pub(crate) fn drain_doc_changes(&mut self, changes: &mut Vec<DocChange>) {
        changes.extend(self.doc_changes.drain(..));
    }

    fn do_open_doc(&mut self, doc: DocId, version: i32, origin: DocChangeOrigin) -> DocId {
        self.doc_versions.insert(doc, version);
        self.doc_changes.push(DocChange::Opened { doc, origin });
        doc
    }

    fn do_change_doc(&mut self, doc: DocId, version: i32, origin: DocChangeOrigin) {
        self.doc_versions.insert(doc, version);
        self.doc_changes.push(DocChange::Changed { doc, origin });
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
            self.do_open_doc(doc, version, DocChangeOrigin::Editor(text));
        } else {
            self.do_change_doc(doc, version, DocChangeOrigin::Editor(text));
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
            self.do_open_doc(doc, version, DocChangeOrigin::Editor(text));
        } else {
            self.do_change_doc(doc, version, DocChangeOrigin::Editor(text));
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

    pub(crate) fn change_file(&mut self, path: &Path) -> Option<DocId> {
        let uri = match CanonicalUri::from_file_path(path) {
            Some(uri) => uri,
            None => return None,
        };

        let (created, doc) = self.touch_uri(uri);

        let open_in_editor = !created && self.editor_docs.contains(&doc);
        if open_in_editor {
            #[cfg(trace_docs)]
            trace!("ファイルは開かれているのでロードされません。");
            return Some(doc);
        }

        let origin = DocChangeOrigin::Path(path.to_path_buf());
        let opened = self.file_docs.insert(doc);
        if opened {
            self.do_open_doc(doc, NO_VERSION, origin);
        } else {
            self.do_change_doc(doc, NO_VERSION, origin);
        }

        Some(doc)
    }

    pub(crate) fn close_file(&mut self, path: &Path) {
        let uri = match CanonicalUri::from_file_path(path) {
            Some(uri) => uri,
            None => return,
        };

        let doc = match self.uri_to_doc.get(&uri) {
            Some(&doc) => doc,
            None => return,
        };

        self.file_docs.remove(&doc);

        if !self.editor_docs.contains(&doc) {
            self.do_close_doc(doc, &uri);
        }
    }

    pub(crate) fn close_all_files(&mut self) {
        for doc in take(&mut self.file_docs) {
            let uri = match self.doc_to_uri.get(&doc).cloned() {
                Some(uri) => uri,
                None => continue,
            };

            if !self.editor_docs.contains(&doc) {
                self.do_close_doc(doc, &uri);
            }
        }
    }

    /// ファイルとDocIdの対応付けを行う。
    pub(crate) fn ensure_file_opened(&mut self, path: &Path) -> Option<DocId> {
        self.change_file(path)
    }
}
