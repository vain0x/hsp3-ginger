use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, TryRecvError},
};

use notify::{DebouncedEvent, RecommendedWatcher};

pub(crate) struct FileWatcher {
    watched_dir: PathBuf,
    watcher_opt: Option<RecommendedWatcher>,
    rx_opt: Option<Receiver<DebouncedEvent>>,
    changed_files: HashSet<PathBuf>,
    closed_files: HashSet<PathBuf>,
}

impl FileWatcher {
    pub(crate) fn new(watched_dir: PathBuf) -> Self {
        Self {
            watched_dir,
            watcher_opt: None,
            rx_opt: None,
            changed_files: HashSet::new(),
            closed_files: HashSet::new(),
        }
    }

    pub(crate) fn scan_files(&mut self) {
        do_scan_all(&self.watched_dir, &mut self.changed_files);
    }

    pub(crate) fn start_watch(&mut self) {
        assert!(self.watcher_opt.is_none());
        assert!(self.rx_opt.is_none());

        self.scan_files();

        if let Some((watcher, rx)) = do_start_watch(&self.watched_dir) {
            self.watcher_opt = Some(watcher);
            self.rx_opt = Some(rx);
        }
    }

    /// 監視を停止する。(呼ばなくても問題ない。)
    pub(crate) fn stop_watch(&mut self) {
        #[cfg(trace_docs)]
        trace!("ファイルウォッチャーがシャットダウンしました。");
        self.watcher_opt = None;
        self.rx_opt = None;
    }

    pub(crate) fn poll(&mut self, rescan: &mut bool) {
        let rx = match self.rx_opt.as_mut() {
            None => return,
            Some(rx) => rx,
        };

        #[cfg(trace_docs)]
        trace!("ファイルウォッチャーのイベントをポールします。");

        let mut disconnected = false;

        loop {
            match rx.try_recv() {
                Ok(DebouncedEvent::Create(ref path)) if file_ext_is_watched(path) => {
                    #[cfg(trace_docs)]
                    trace!("ファイルが作成されました: {:?}", path);
                    self.closed_files.remove(path);
                    self.changed_files.insert(path.clone());
                }
                Ok(DebouncedEvent::Write(ref path)) if file_ext_is_watched(path) => {
                    #[cfg(trace_docs)]
                    trace!("ファイルが変更されました: {:?}", path);
                    self.closed_files.remove(path);
                    self.changed_files.insert(path.clone());
                }
                Ok(DebouncedEvent::Remove(ref path)) if file_ext_is_watched(path) => {
                    #[cfg(trace_docs)]
                    trace!("ファイルが削除されました: {:?}", path);
                    self.changed_files.remove(path);
                    self.closed_files.insert(path.clone());
                }
                Ok(DebouncedEvent::Rename(ref src_path, ref dest_path)) => {
                    #[cfg(trace_docs)]
                    trace!("ファイルが移動しました: {:?} → {:?}", src_path, dest_path);
                    if file_ext_is_watched(src_path) {
                        self.changed_files.remove(src_path);
                        self.closed_files.insert(src_path.clone());
                    }
                    if file_ext_is_watched(dest_path) {
                        self.closed_files.remove(dest_path);
                        self.changed_files.insert(dest_path.clone());
                    }
                }
                Ok(DebouncedEvent::Rescan) => {
                    #[cfg(trace_docs)]
                    trace!("ファイルウォッチャーから再スキャンが要求されました");
                    *rescan = true;
                }
                #[allow(unused)]
                Ok(ev) => {
                    #[cfg(trace_docs)]
                    trace!("ファイルウォッチャーのイベントをスキップします: {:?}", ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    disconnected = true;
                    *rescan = false;
                    break;
                }
            }
        }

        if *rescan {
            self.changed_files.clear();
            self.closed_files.clear();
            self.scan_files();
        }

        if disconnected {
            self.stop_watch();
        }

        #[cfg(trace_docs)]
        trace!(
            "ファイルウォッチャーのイベントをポールしました (change={} remove={}{}{})",
            self.changed_files.len(),
            self.closed_files.len(),
            if *rescan { " rescan=true" } else { "" },
            if disconnected {
                " disconnected=true"
            } else {
                ""
            }
        );
    }

    pub(crate) fn drain_changes(
        &mut self,
        changed_files: &mut Vec<PathBuf>,
        closed_files: &mut Vec<PathBuf>,
    ) {
        changed_files.extend(self.changed_files.drain());
        closed_files.extend(self.closed_files.drain());
    }
}

fn file_ext_is_watched(path: &Path) -> bool {
    path.extension()
        .map_or(false, |ext| ext == "hsp" || ext == "as")
}

fn do_scan_all(watched_dir: &Path, changed_files: &mut HashSet<PathBuf>) -> Option<()> {
    let glob_pattern = format!("{}/**/*.hsp", watched_dir.to_str()?);

    #[cfg(trace_docs)]
    trace!("ファイルリストの取得 '{}'", glob_pattern);

    let entries = match glob::glob(&glob_pattern) {
        Err(err) => {
            warn!("ファイルリストの取得 {:?}", err);
            return None;
        }
        Ok(entries) => entries,
    };

    for entry in entries {
        match entry {
            Ok(path) => {
                changed_files.insert(path);
            }
            Err(err) => warn!("ファイルエントリの取得 {:?}", err),
        }
    }

    None
}

fn do_start_watch(watched_dir: &Path) -> Option<(RecommendedWatcher, Receiver<DebouncedEvent>)> {
    #[cfg(trace_docs)]
    trace!("ファイルウォッチャーを起動します");

    use notify::{RecursiveMode, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    let delay_millis = 1000;

    // let current_dir = std::env::current_dir()
    //     .map_err(|err| warn!("カレントディレクトリの取得 {:?}", err))
    //     .ok()?;

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_millis(delay_millis))
        .map_err(|err| warn!("ファイルウォッチャーの作成 {:?}", err))
        .ok()?;

    watcher
        .watch(watched_dir, RecursiveMode::Recursive)
        .map_err(|err| warn!("ファイルウォッチャーの起動 {:?}", err))
        .ok()?;

    #[cfg(trace_docs)]
    trace!("ファイルウォッチャーを起動しました ({:?})", watched_dir);
    Some((watcher, rx))
}
