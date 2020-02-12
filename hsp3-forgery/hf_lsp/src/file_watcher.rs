//! ファイルの変更を検出する機能

use crate::file_changes::FileChanges;
use notify::{DebouncedEvent, RecommendedWatcher};
use notify::{RecursiveMode, Watcher};
use std::env;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

#[derive(Default)]
pub(crate) struct FileWatcher {
    changes: FileChanges,
    file_watcher: Option<RecommendedWatcher>,
    file_event_rx: Option<Receiver<DebouncedEvent>>,
}

impl FileWatcher {
    pub(crate) fn start() -> Option<Self> {
        let mut it = FileWatcher::default();

        it.scan()?;

        let (file_watcher, file_event_rx) = it.do_start()?;
        it.file_watcher = Some(file_watcher);
        it.file_event_rx = Some(file_event_rx);

        Some(it)
    }

    fn do_start(&mut self) -> Option<(RecommendedWatcher, Receiver<DebouncedEvent>)> {
        debug!("ファイルウォッチャーを起動します。");

        let delay_millis = 1000;

        let current_dir = env::current_dir()
            .map_err(|err| warn!("カレントディレクトリの取得 {:?}", err))
            .ok()?;

        let (tx, rx) = mpsc::channel();

        let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_millis(delay_millis))
            .map_err(|err| warn!("ファイルウォッチャーの作成 {:?}", err))
            .ok()?;

        watcher
            .watch(&current_dir, RecursiveMode::Recursive)
            .map_err(|err| warn!("ファイルウォッチャーの起動 {:?}", err))
            .ok()?;

        debug!(
            "ファイルウォッチャーを起動しました。(current_dir = {:?})",
            current_dir
        );
        Some((watcher, rx))
    }

    fn shutdown(&mut self) {
        debug!("ファイルウォッチャーがシャットダウンしました。");
        self.file_watcher.take();
        self.file_event_rx.take();
    }

    pub(crate) fn scan(&mut self) -> Option<()> {
        debug!("FileWatch::scan");

        let current_dir = env::current_dir()
            .map_err(|err| warn!("カレントディレクトリの取得 {:?}", err))
            .ok()?;

        let glob_pattern = format!("{}/**/*.hsp", current_dir.to_str()?);

        debug!("ファイルリストの取得 {:?}", glob_pattern);

        let entries = match glob::glob(&glob_pattern) {
            Err(err) => {
                warn!("ファイルリストの取得に失敗 {:?}", err);
                return None;
            }
            Ok(entries) => entries,
        };

        for entry in entries {
            match entry {
                Err(err) => warn!("ファイルエントリの取得 {:?}", err),
                Ok(path) => {
                    self.changes.add_updated_path(path);
                }
            }
        }

        None
    }

    pub(crate) fn poll(&mut self) {
        let rx = match self.file_event_rx.as_mut() {
            None => return,
            Some(rx) => rx,
        };

        debug!("ファイルウォッチャーのイベントを読み取ります。");

        loop {
            match rx.try_recv() {
                Ok(DebouncedEvent::Create(ref path)) if file_ext_is_watched(path) => {
                    debug!("ファイルが作成されました: {:?}", path);
                    self.changes.add_updated_path(path.clone());
                }
                Ok(DebouncedEvent::Write(ref path)) if file_ext_is_watched(path) => {
                    debug!("ファイルが変更されました: {:?}", path);
                    self.changes.add_updated_path(path.clone());
                }
                Ok(DebouncedEvent::Remove(ref path)) if file_ext_is_watched(path) => {
                    debug!("ファイルが削除されました: {:?}", path);
                    self.changes.add_removed_path(path.clone());
                }
                Ok(DebouncedEvent::Rename(ref src_path, ref dest_path)) => {
                    debug!("ファイルが移動しました: {:?} → {:?}", src_path, dest_path);
                    if file_ext_is_watched(src_path) {
                        self.changes.add_removed_path(src_path.clone());
                    }
                    if file_ext_is_watched(dest_path) {
                        self.changes.add_updated_path(dest_path.clone());
                    }
                }
                Ok(DebouncedEvent::Rescan) => {
                    debug!("ファイルウォッチャーから再スキャンが要求されました");
                    self.changes.rescan_required = true;
                }
                Ok(ev) => {
                    debug!("ファイルウォッチャーのイベントをスキップします: {:?}", ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.changes.disconnected = true;
                    break;
                }
            }
        }

        if self.changes.disconnected {
            self.shutdown();
        }

        if self.changes.rescan_required {
            self.scan();
            self.changes.rescan_required = false;
        }
    }

    pub(crate) fn swap_changes(&mut self, changes: &mut FileChanges) {
        std::mem::swap(&mut self.changes, changes);
    }
}

fn file_ext_is_watched(path: &Path) -> bool {
    path.extension()
        .map_or(false, |ext| ext == "hsp" || ext == "as")
}
