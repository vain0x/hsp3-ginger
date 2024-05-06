use super::*;
use crate::{
    lsp_server::{TextDocumentVersion, NO_VERSION},
    source::DocId,
};

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
    doc_versions: HashMap<DocId, TextDocumentVersion>,

    /// エディタで開かれているドキュメント
    editor_docs: HashSet<DocId>,

    /// ファイルとして保存されているドキュメント
    file_docs: HashSet<DocId>,

    /// 最近の更新
    doc_changes: Vec<DocChange>,
}

impl Docs {
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

    fn do_close_doc(&mut self, doc: DocId) {
        self.doc_versions.remove(&doc);
        self.doc_changes.push(DocChange::Closed { doc });
    }

    pub(crate) fn open_doc_in_editor(&mut self, doc: DocId, version: i32, text: RcStr) {
        #[cfg(trace_docs)]
        trace!(
            "クライアントでファイルが開かれました ({:?} version={}, len={})",
            uri,
            version,
            text.len()
        );

        assert!(!self.editor_docs.contains(&doc));
        self.do_open_doc(doc, version, Lang::Hsp3, DocChangeOrigin::Editor(text));
        self.editor_docs.insert(doc);
    }

    pub(crate) fn change_doc_in_editor(&mut self, doc: DocId, version: i32, text: RcStr) {
        #[cfg(trace_docs)]
        trace!(
            "クライアントでファイルが変更されました ({:?} version={}, len={})",
            uri,
            version,
            text.len()
        );

        assert!(self.editor_docs.contains(&doc));
        self.do_change_doc(doc, version, Lang::Hsp3, DocChangeOrigin::Editor(text));
        self.editor_docs.insert(doc);
    }

    pub(crate) fn close_doc_in_editor(&mut self, doc: DocId) -> bool {
        #[cfg(trace_docs)]
        trace!("クライアントでファイルが閉じられました ({:?})", uri);

        assert!(self.editor_docs.contains(&doc));
        self.editor_docs.remove(&doc);

        if !self.file_docs.contains(&doc) {
            self.do_close_doc(doc);
            true
        } else {
            false
        }
    }

    pub(crate) fn change_file(&mut self, doc: DocId, path: &Path) -> Option<DocId> {
        let open_in_editor = self.editor_docs.contains(&doc);
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

    pub(crate) fn close_file(&mut self, doc: DocId) -> bool {
        self.file_docs.remove(&doc);

        if !self.editor_docs.contains(&doc) {
            self.do_close_doc(doc);
            true
        } else {
            false
        }
    }

    /// ファイルとDocIdの対応付けを行う。
    pub(crate) fn ensure_file_opened(&mut self, doc: DocId, abs_path: &Path) -> Option<DocId> {
        self.change_file(doc, abs_path)
    }
}

/// `#include` のファイルパスを解決する
///
/// - ドキュメント `base_doc` に `#include` が含まれていて、そこにファイルパス `included_name` が指定されているとする。
///     そのincludeされるファイルへの `DocId` があれば返す
/// - この関数では `common` やその他のパスが通ったディレクトリは探索されない
pub(crate) fn resolve_included_name(
    doc_interner: &DocInterner,
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
            if let Some(d) = doc_interner.get_doc(&u) {
                return Some(d);
            }
        }
    }

    let src_file = match doc_interner
        .get_uri(base_doc)
        .and_then(|uri| uri.to_file_path())
    {
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
    doc_interner.get_doc(&resolved_uri)
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

    fn f(doc_interner: &mut DocInterner, docs: &mut Docs, s: &str) -> DocId {
        let path = p(s);
        let (_, doc) = doc_interner.intern(&CanonicalUri::from_abs_path(&path).unwrap());
        docs.ensure_file_opened(doc, &path);
        doc
    }

    #[test]
    fn test_resolve_included_name() {
        let mut di = DocInterner::default();
        let mut docs = Docs::default();
        let a = f(&mut di, &mut docs, "a.hsp");
        let b = f(&mut di, &mut docs, "b.hsp");
        let c = f(&mut di, &mut docs, "x/c.hsp");
        let d = f(&mut di, &mut docs, "x/d.hsp");
        let e = f(&mut di, &mut docs, "y/e.hsp");

        // aから兄弟・子孫に位置するファイルを参照できること
        assert_eq!(resolve_included_name(&di, "b.hsp", a), Some(b));
        assert_eq!(resolve_included_name(&di, "./b.hsp", a), Some(b));
        assert_eq!(resolve_included_name(&di, "x/c.hsp", a), Some(c));

        // c (入れ子のディレクトリに含まれるファイル) から相対パスを使って参照できること
        assert_eq!(resolve_included_name(&di, "d.hsp", c), Some(d));
        assert_eq!(resolve_included_name(&di, "../a.hsp", c), Some(a));
        assert_eq!(resolve_included_name(&di, "../y/e.hsp", c), Some(e));

        // 存在しないファイルは解決されないこと
        assert_eq!(resolve_included_name(&di, "c.hsp", a), None);

        // 妙なケース: aからa自身への参照
        assert_eq!(resolve_included_name(&di, "a.hsp", a), Some(a));
        assert_eq!(resolve_included_name(&di, "./a.hsp", a), Some(a));

        // 不正なinclude名の例
        assert_eq!(resolve_included_name(&di, "", a), None);
        assert_eq!(resolve_included_name(&di, "*", a), None);
        assert_eq!(resolve_included_name(&di, "/a.hsp", a), None);
    }
}
