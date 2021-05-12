use crate::{
    source::DocId,
    utils::{canonical_uri::CanonicalUri, rc_str::RcStr, read_file::read_file},
};
use std::{
    collections::{HashMap, HashSet},
    mem::take,
    path::Path,
};

/// テキストドキュメントのバージョン番号
/// (エディタ上で編集されるたびに変わる番号。
///  いつの状態のテキストドキュメントを指しているかを明確にするためのもの。)
type TextDocumentVersion = i64;

pub(crate) const NO_VERSION: TextDocumentVersion = 1;

pub(crate) enum DocChange {
    Opened { doc: DocId, text: RcStr },
    Changed { doc: DocId, text: RcStr },
    Closed { doc: DocId },
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
    doc_texts: HashMap<DocId, RcStr>,

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

    fn do_open_doc(&mut self, doc: DocId, version: i64, text: RcStr) -> DocId {
        self.doc_versions.insert(doc, version);
        self.doc_texts.insert(doc, text.clone());
        self.doc_changes.push(DocChange::Opened { doc, text });
        doc
    }

    fn do_change_doc(&mut self, doc: DocId, version: i64, text: RcStr) {
        self.doc_versions.insert(doc, version);
        self.doc_texts.insert(doc, text.clone());
        self.doc_changes.push(DocChange::Changed { doc, text });
    }

    fn do_close_doc(&mut self, doc: DocId, uri: &CanonicalUri) {
        self.doc_to_uri.remove(&doc);
        self.uri_to_doc.remove(&uri);
        self.doc_versions.remove(&doc);
        self.doc_texts.remove(&doc);
        self.doc_changes.push(DocChange::Closed { doc });
    }

    pub(crate) fn open_doc_in_editor(&mut self, uri: CanonicalUri, version: i64, text: String) {
        #[cfg(trace_docs)]
        trace!(
            "クライアントでファイルが開かれました ({:?} version={}, len={})",
            uri,
            version,
            text.len()
        );

        let (created, doc) = self.touch_uri(uri);
        if created {
            self.do_open_doc(doc, version, text.into());
        } else {
            // ファイルをエディタで開いたとき、内容は同じである可能性が高い。そのときは更新の報告を省略する。
            let same = self
                .doc_texts
                .get(&doc)
                .map_or(false, |current| current.as_str() == text);
            if same {
                self.doc_versions.insert(doc, version);
            } else {
                self.do_change_doc(doc, version, text.into());
            }
        }

        self.editor_docs.insert(doc);
    }

    pub(crate) fn change_doc_in_editor(&mut self, uri: CanonicalUri, version: i64, text: String) {
        #[cfg(trace_docs)]
        trace!(
            "クライアントでファイルが変更されました ({:?} version={}, len={})",
            uri,
            version,
            text.len()
        );

        let (created, doc) = self.touch_uri(uri);
        if created {
            self.do_open_doc(doc, version, text.into());
        } else {
            self.do_change_doc(doc, version, text.into());
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

    pub(crate) fn change_file(&mut self, path: &Path) {
        let uri = match CanonicalUri::from_file_path(path) {
            Some(uri) => uri,
            None => return,
        };

        let (created, doc) = self.touch_uri(uri);

        let open_in_editor = !created && self.editor_docs.contains(&doc);
        if open_in_editor {
            #[cfg(trace_docs)]
            trace!("ファイルは開かれているのでロードされません。");
            return;
        }

        // FIXME: drain_doc_changesのタイミングまで読み込みを遅延してもいい。
        let mut text = String::new();
        if !read_file(path, &mut text) {
            warn!("ファイルを開けません {:?}", path);
        }

        let opened = self.file_docs.insert(doc);
        if opened {
            self.do_open_doc(doc, NO_VERSION, text.into());
        } else {
            self.do_change_doc(doc, NO_VERSION, text.into());
        }
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
        let uri = match CanonicalUri::from_file_path(path) {
            Some(uri) => uri,
            None => return None,
        };

        self.change_file(path);
        let (_, doc) = self.touch_uri(uri);
        Some(doc)
    }
}
