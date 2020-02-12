use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct FileChanges {
    /// 作成・変更・移動したファイルのパスの集合
    pub(crate) updated_paths: HashSet<PathBuf>,

    /// 削除されたファイルのパスの集合
    pub(crate) removed_paths: HashSet<PathBuf>,

    /// ファイルウォッチャーがファイルシステムの再スキャンを要求しているか？
    pub(crate) rescan_required: bool,

    /// ファイルウォッチャーが切断したか？
    pub(crate) disconnected: bool,
}

impl FileChanges {
    pub(crate) fn clear(&mut self) {
        self.updated_paths.clear();
        self.removed_paths.clear();
        self.rescan_required = false;
        self.disconnected = false;
    }

    pub(crate) fn add_updated_path(&mut self, path: PathBuf) {
        self.updated_paths.insert(path);
    }

    pub(crate) fn add_removed_path(&mut self, path: PathBuf) {
        self.removed_paths.insert(path);
    }
}
