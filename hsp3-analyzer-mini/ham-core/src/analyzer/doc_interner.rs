use super::CanonicalUri;
use crate::source::DocId;
use std::collections::HashMap;

/// `DocId` と `URI` の対応を記録するもの
#[derive(Default)]
pub(crate) struct DocInterner {
    /// 最後に振ったID
    last_doc: usize,

    // ドキュメントの情報:
    doc_to_uri: HashMap<DocId, CanonicalUri>,
    uri_to_doc: HashMap<CanonicalUri, DocId>,
}

impl DocInterner {
    // pub(crate) fn new() -> Self {
    //     Self::default()
    // }

    /// URIに対応するDocIdを探す。なければ作り、trueを返す。
    pub(crate) fn intern(&mut self, uri: &CanonicalUri) -> (bool, DocId) {
        match self.uri_to_doc.get(&uri) {
            Some(&doc) => (false, doc),
            None => {
                self.last_doc += 1;
                let doc = self.last_doc;
                self.doc_to_uri.insert(doc, uri.clone());
                self.uri_to_doc.insert(uri.clone(), doc);

                debug!("doc_intern doc:{} {:?}", doc, uri);

                (true, doc)
            }
        }
    }

    pub(crate) fn get_uri(&self, doc: DocId) -> Option<&CanonicalUri> {
        self.doc_to_uri.get(&doc)
    }

    pub(crate) fn get_doc(&self, uri: &CanonicalUri) -> Option<DocId> {
        self.uri_to_doc.get(uri).cloned()
    }

    /// 使用されなくなったDocIdに関するデータを削除する
    pub(crate) fn remove(&mut self, doc: DocId, uri: &CanonicalUri) {
        // デバッグ時のみ: 正しいペアが渡されたことを確認する
        match (self.doc_to_uri.get(&doc), self.uri_to_doc.get(uri)) {
            (Some(actual_uri), Some(&actual_doc)) => {
                debug_assert_eq!(actual_doc, doc);
                debug_assert_eq!(actual_uri, uri);
            }
            _ => {}
        }

        self.doc_to_uri.remove(&doc);
        self.uri_to_doc.remove(&uri);
    }
}
