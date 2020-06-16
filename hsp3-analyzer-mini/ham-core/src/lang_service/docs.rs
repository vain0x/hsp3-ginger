use crate::{
    analysis::ADoc,
    lang::Lang,
    utils::{canonical_uri::CanonicalUri, rc_str::RcStr},
};
use encoding::{codec::utf_8::UTF8Encoding, DecoderTrap, Encoding, StringWriter};
use notify::{DebouncedEvent, RecommendedWatcher};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};

/// テキストドキュメントのバージョン番号
/// (エディタ上で編集されるたびに変わる番号。
///  いつの状態のテキストドキュメントを指しているかを明確にするためのもの。)
type TextDocumentVersion = i64;

pub(crate) const NO_VERSION: i64 = 1;

pub(crate) enum DocChange {
    Opened { doc: ADoc, text: RcStr },
    Changed { doc: ADoc, text: RcStr },
    Closed { doc: ADoc },
}

/// テキストドキュメントを管理するもの。
#[derive(Default)]
pub(crate) struct Docs {
    last_doc: usize,
    doc_to_uri: HashMap<ADoc, CanonicalUri>,
    uri_to_doc: HashMap<CanonicalUri, ADoc>,
    open_docs: HashSet<ADoc>,
    doc_langs: HashMap<ADoc, Lang>,
    doc_versions: HashMap<ADoc, TextDocumentVersion>,
    // hsphelp や common の下をウォッチするのに使う
    #[allow(unused)]
    hsp_root: PathBuf,
    file_watcher: Option<RecommendedWatcher>,
    file_event_rx: Option<Receiver<DebouncedEvent>>,
    doc_changes: Vec<DocChange>,
}

impl Docs {
    pub(super) fn new(hsp_root: PathBuf) -> Self {
        Self {
            hsp_root,
            ..Default::default()
        }
    }

    // pub(crate) fn is_open(&self, uri: &Url) -> bool {
    //     self.uri_to_doc
    //         .get(&uri)
    //         .map_or(false, |doc| self.open_docs.contains(&doc))
    // }

    pub(crate) fn fresh_doc(&mut self) -> ADoc {
        self.last_doc += 1;
        ADoc::new(self.last_doc)
    }

    fn resolve_uri(&mut self, uri: CanonicalUri) -> ADoc {
        match self.uri_to_doc.get(&uri) {
            Some(&doc) => doc,
            None => {
                let doc = self.fresh_doc();
                self.doc_to_uri.insert(doc, uri.clone());
                self.uri_to_doc.insert(uri, doc);
                doc
            }
        }
    }

    pub(crate) fn find_by_uri(&self, uri: &CanonicalUri) -> Option<ADoc> {
        self.uri_to_doc.get(uri).cloned()
    }

    pub(crate) fn get_uri(&self, doc: ADoc) -> Option<&CanonicalUri> {
        self.doc_to_uri.get(&doc)
    }

    pub(crate) fn get_version(&self, doc: ADoc) -> Option<TextDocumentVersion> {
        self.doc_versions.get(&doc).copied()
    }

    pub(crate) fn drain_doc_changes(&mut self, changes: &mut Vec<DocChange>) {
        changes.extend(self.doc_changes.drain(..));
    }

    pub(super) fn did_initialize(&mut self) {
        self.scan_files();

        if let Some((file_watcher, file_event_rx)) = self.start_file_watcher() {
            self.file_watcher = Some(file_watcher);
            self.file_event_rx = Some(file_event_rx);
        }
    }

    fn scan_files(&mut self) -> Option<()> {
        let current_dir = std::env::current_dir()
            .map_err(|err| warn!("カレントディレクトリの取得 {:?}", err))
            .ok()?;

        let glob_pattern = format!("{}/**/*.hsp", current_dir.to_str()?);

        debug!("ファイルリストを取得します '{}'", glob_pattern);

        let entries = match glob::glob(&glob_pattern) {
            Err(err) => {
                warn!("ファイルリストの取得 {:?}", err);
                return None;
            }
            Ok(entries) => entries,
        };

        for entry in entries {
            match entry {
                Err(err) => warn!("ファイルエントリの取得 {:?}", err),
                Ok(path) => {
                    self.change_file(&path);
                }
            }
        }

        None
    }

    fn start_file_watcher(&mut self) -> Option<(RecommendedWatcher, Receiver<DebouncedEvent>)> {
        debug!("ファイルウォッチャーを起動します");

        use notify::{RecursiveMode, Watcher};
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let delay_millis = 1000;

        let current_dir = std::env::current_dir()
            .map_err(|err| warn!("カレントディレクトリの取得 {:?}", err))
            .ok()?;

        let (tx, rx) = channel();

        let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_millis(delay_millis))
            .map_err(|err| warn!("ファイルウォッチャーの作成 {:?}", err))
            .ok()?;

        watcher
            .watch(&current_dir, RecursiveMode::Recursive)
            .map_err(|err| warn!("ファイルウォッチャーの起動 {:?}", err))
            .ok()?;

        debug!("ファイルウォッチャーを起動しました ({:?})", current_dir);
        Some((watcher, rx))
    }

    pub(crate) fn poll(&mut self) {
        let rx = match self.file_event_rx.as_mut() {
            None => return,
            Some(rx) => rx,
        };

        debug!("ファイルウォッチャーのイベントをポールします。");

        let mut rescan = false;
        let mut updated_paths = HashSet::new();
        let mut removed_paths = HashSet::new();
        let mut disconnected = false;

        loop {
            match rx.try_recv() {
                Ok(DebouncedEvent::Create(ref path)) if file_ext_is_watched(path) => {
                    debug!("ファイルが作成されました: {:?}", path);
                    updated_paths.insert(path.clone());
                }
                Ok(DebouncedEvent::Write(ref path)) if file_ext_is_watched(path) => {
                    debug!("ファイルが変更されました: {:?}", path);
                    updated_paths.insert(path.clone());
                }
                Ok(DebouncedEvent::Remove(ref path)) if file_ext_is_watched(path) => {
                    debug!("ファイルが削除されました: {:?}", path);
                    removed_paths.insert(path.clone());
                }
                Ok(DebouncedEvent::Rename(ref src_path, ref dest_path)) => {
                    debug!("ファイルが移動しました: {:?} → {:?}", src_path, dest_path);
                    if file_ext_is_watched(src_path) {
                        removed_paths.insert(src_path.clone());
                    }
                    if file_ext_is_watched(dest_path) {
                        updated_paths.insert(dest_path.clone());
                    }
                }
                Ok(DebouncedEvent::Rescan) => {
                    debug!("ファイルウォッチャーから再スキャンが要求されました");
                    rescan = true;
                }
                Ok(ev) => {
                    debug!("ファイルウォッチャーのイベントをスキップします: {:?}", ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        let mut change_count = 0;
        let mut remove_count = 0;

        if rescan {
            self.scan_files();
        } else {
            for path in updated_paths {
                if removed_paths.contains(&path) {
                    continue;
                }
                self.change_file(&path);
                change_count += 1;
            }

            for path in removed_paths {
                self.close_file(&path);
                remove_count += 1;
            }
        }

        if disconnected {
            self.shutdown_file_watcher();
        }

        debug!(
            "ファイルウォッチャーのイベントをポールしました (change={} remove={}{}{})",
            change_count,
            remove_count,
            if rescan { " rescan=true" } else { "" },
            if disconnected {
                " disconnected=true"
            } else {
                ""
            }
        );
    }

    fn shutdown_file_watcher(&mut self) {
        debug!("ファイルウォッチャーがシャットダウンしました。");
        self.file_watcher.take();
        self.file_event_rx.take();
    }

    pub(super) fn shutdown(&mut self) {
        self.shutdown_file_watcher();
    }

    fn do_open_doc(&mut self, uri: CanonicalUri, version: i64, text: RcStr) -> ADoc {
        let doc = self.resolve_uri(uri);

        // この LSP サーバーが対応する拡張子として .hsp しか指定していないので、
        // クライアントから送られてくる情報は .hsp ファイルのものだと仮定してよいはず。
        self.doc_langs.insert(doc, Lang::Hsp3);

        self.doc_versions.insert(doc, version);
        self.doc_changes.push(DocChange::Opened { doc, text });

        doc
    }

    fn do_change_doc(&mut self, uri: CanonicalUri, version: i64, text: RcStr) {
        let doc = self.resolve_uri(uri);
        self.doc_versions.insert(doc, version);
        self.doc_changes.push(DocChange::Changed { doc, text });
    }

    fn do_close_doc(&mut self, uri: CanonicalUri) {
        if let Some(&doc) = self.uri_to_doc.get(&uri) {
            self.doc_to_uri.remove(&doc);
            self.doc_langs.remove(&doc);
            self.doc_versions.remove(&doc);
            self.doc_changes.push(DocChange::Closed { doc })
        }

        self.uri_to_doc.remove(&uri);
    }

    pub(super) fn open_doc(&mut self, uri: CanonicalUri, version: i64, text: String) {
        trace!(
            "クライアントでファイルが開かれました ({:?} version={}, len={})",
            uri,
            version,
            text.len()
        );

        self.do_open_doc(uri.clone(), version, text.into());

        if let Some(&doc) = self.uri_to_doc.get(&uri) {
            self.open_docs.insert(doc);
        }

        self.poll();
    }

    pub(super) fn change_doc(&mut self, uri: CanonicalUri, version: i64, text: String) {
        trace!(
            "クライアントでファイルが変更されました ({:?} version={}, len={})",
            uri,
            version,
            text.len()
        );

        self.do_change_doc(uri, version, text.into());

        self.poll();
    }

    pub(super) fn close_doc(&mut self, uri: CanonicalUri) {
        trace!("クライアントでファイルが閉じられました ({:?})", uri);

        if let Some(&doc) = self.uri_to_doc.get(&uri) {
            self.open_docs.remove(&doc);
        }

        self.poll();
    }

    pub(super) fn change_file(&mut self, path: &Path) -> Option<()> {
        let uri = CanonicalUri::from_file_path(path)?;

        let is_open = self
            .uri_to_doc
            .get(&uri)
            .map_or(false, |doc| self.open_docs.contains(&doc));
        if is_open {
            debug!("ファイルは開かれているのでロードされません。");
            return None;
        }

        let mut text = String::new();
        if !read_file(path, &mut text) {
            warn!("ファイルを開けません {:?}", path);
        }

        self.do_change_doc(uri, NO_VERSION, text.into());

        None
    }

    pub(super) fn close_file(&mut self, path: &Path) -> Option<()> {
        let uri = CanonicalUri::from_file_path(path)?;

        self.do_close_doc(uri);

        None
    }
}

fn file_ext_is_watched(path: &Path) -> bool {
    path.extension()
        .map_or(false, |ext| ext == "hsp" || ext == "as")
}

/// ファイルを shift_jis または UTF-8 として読む。
fn read_file(file_path: &Path, out: &mut impl StringWriter) -> bool {
    // utf-8?
    let content = match fs::read(file_path).ok() {
        None => return false,
        Some(x) => x,
    };

    // shift-jis?
    encoding::all::WINDOWS_31J
        .decode_to(&content, DecoderTrap::Strict, out)
        .or_else(|_| UTF8Encoding.decode_to(&content, DecoderTrap::Strict, out))
        .is_ok()
}
