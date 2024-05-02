use std::path::{Path, PathBuf};

/// スクリプトファイルを探索する
///
/// - ルートディレクトリから再帰的にディレクトリをたどり、`.hsp` 拡張子のファイルを見つけるたび、 `on_script_path` 関数が呼ばれる
pub(crate) fn scan_script_files(root_dir: &Path, mut on_script_path: impl FnMut(PathBuf)) {
    let glob_results = match glob::glob(&format!("{}/**/*.hsp", root_dir.to_string_lossy())) {
        Ok(it) => it,
        Err(err) => {
            warn!("glob: {:?}", err);
            return;
        }
    };

    for path in glob_results.flatten() {
        on_script_path(path);
    }
}
