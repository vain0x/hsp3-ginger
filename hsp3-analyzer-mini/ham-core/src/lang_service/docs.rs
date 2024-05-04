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
    interner: DocInterner,
    doc_versions: HashMap<DocId, TextDocumentVersion>,

    /// エディタで開かれているドキュメント
    editor_docs: HashSet<DocId>,

    /// ファイルとして保存されているドキュメント
    file_docs: HashSet<DocId>,

    /// 最近の更新
    doc_changes: Vec<DocChange>,
}

impl Docs {
    pub(crate) fn find_by_uri(&self, uri: &CanonicalUri) -> Option<DocId> {
        self.interner.get_doc(uri)
    }

    pub(crate) fn get_uri(&self, doc: DocId) -> Option<&CanonicalUri> {
        self.interner.get_uri(doc)
    }

    pub(crate) fn get_version(&self, doc: DocId) -> Option<TextDocumentVersion> {
        self.doc_versions.get(&doc).copied()
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
        self.interner.remove(doc, uri);
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

        let (created, doc) = self.interner.intern(&uri);
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

        let (created, doc) = self.interner.intern(&uri);
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

        let doc = match self.interner.get_doc(&uri) {
            Some(doc) => doc,
            None => return,
        };

        self.editor_docs.remove(&doc);

        if !self.file_docs.contains(&doc) {
            self.do_close_doc(doc, &uri);
        }
    }

    fn do_change_file(&mut self, uri: CanonicalUri, path: &Path) -> Option<DocId> {
        let (created, doc) = self.interner.intern(&uri);

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
        let abs_path = uri.to_file_path()?;
        self.do_change_file(uri, &abs_path)
    }

    pub(crate) fn change_file(&mut self, abs_path: &Path) -> Option<DocId> {
        let uri = CanonicalUri::from_abs_path(abs_path)?;
        self.do_change_file(uri, &abs_path)
    }

    pub(crate) fn close_file_by_uri(&mut self, uri: CanonicalUri) {
        let doc = match self.interner.get_doc(&uri) {
            Some(doc) => doc,
            None => return,
        };

        self.file_docs.remove(&doc);

        if !self.editor_docs.contains(&doc) {
            self.do_close_doc(doc, &uri);
        }
    }

    /// ファイルとDocIdの対応付けを行う。
    pub(crate) fn ensure_file_opened(&mut self, abs_path: &Path) -> Option<DocId> {
        self.change_file(abs_path)
    }
}

/// `#include` のファイルパスを解決する
///
/// - ドキュメント `base_doc` に `#include` が含まれていて、そこにファイルパス `included_name` が指定されているとする。
///     そのincludeされるファイルへの `DocId` があれば返す
/// - この関数では `common` やその他のパスが通ったディレクトリは探索されない
pub(crate) fn resolve_included_name(
    docs: &Docs,
    included_name: &str,
    base_doc: DocId,
) -> Option<DocId> {
    let i_path = match PathBuf::try_from(included_name) {
        Ok(it) => it,
        Err(_) => {
            trace!("{:?} isn't a path", included_name);
            return None;
        }
    };

    // absolute path?
    if i_path.is_absolute() {
        if let Some(u) = CanonicalUri::from_abs_path(&i_path) {
            if let Some(d) = docs.find_by_uri(&u) {
                return Some(d);
            }
        }
    }

    let src_file = match docs.get_uri(base_doc).and_then(|uri| uri.to_file_path()) {
        Some(it) => it,
        None => {
            trace!("base_doc isn't open: {}", base_doc);
            return None;
        }
    };
    let src_dir = src_file.parent()?;

    let resolved_path = src_dir.join(i_path);
    let resolved_uri = CanonicalUri::from_abs_path(&resolved_path)?;
    // debug!("resolved_uri = {:?}", resolved_uri.to_file_path());
    docs.find_by_uri(&resolved_uri)
}

// ===============================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::dummy_path;
    use std::{path::PathBuf, str::FromStr};

    fn p(s: &str) -> PathBuf {
        let p = PathBuf::from_str(s).unwrap();
        dummy_path().join(p)
    }

    #[test]
    fn test_resolve_included_name() {
        let mut docs = Docs::default();
        let a = docs.ensure_file_opened(&p("a.hsp")).unwrap();
        let b = docs.ensure_file_opened(&p("b.hsp")).unwrap();
        let c = docs.ensure_file_opened(&p("x/c.hsp")).unwrap();
        let d = docs.ensure_file_opened(&p("x/d.hsp")).unwrap();
        let e = docs.ensure_file_opened(&p("y/e.hsp")).unwrap();

        // aから兄弟・子孫に位置するファイルを参照できること
        assert_eq!(resolve_included_name(&docs, "b.hsp", a), Some(b));
        assert_eq!(resolve_included_name(&docs, "./b.hsp", a), Some(b));
        assert_eq!(resolve_included_name(&docs, "x/c.hsp", a), Some(c));

        // c (入れ子のディレクトリに含まれるファイル) から相対パスを使って参照できること
        assert_eq!(resolve_included_name(&docs, "d.hsp", c), Some(d));
        assert_eq!(resolve_included_name(&docs, "../a.hsp", c), Some(a));
        assert_eq!(resolve_included_name(&docs, "../y/e.hsp", c), Some(e));

        // 存在しないファイルは解決されないこと
        assert_eq!(resolve_included_name(&docs, "c.hsp", a), None);

        // 妙なケース: aからa自身への参照
        assert_eq!(resolve_included_name(&docs, "a.hsp", a), Some(a));
        assert_eq!(resolve_included_name(&docs, "./a.hsp", a), Some(a));

        // 不正なinclude名の例
        assert_eq!(resolve_included_name(&docs, "", a), None);
        assert_eq!(resolve_included_name(&docs, "*", a), None);
        assert_eq!(resolve_included_name(&docs, "/a.hsp", a), None);
    }
}
