use std::{
    fs,
    path::{Path, PathBuf},
};

/// マニフェストファイル (ginger.txt) を探索する
///
/// - ルートディレクトリから再帰的にディレクトリをたどり、特定の名前を持つファイルを見つけるたび、`on_script_path` 関数が呼ばれる
pub(crate) fn scan_manifest_files(root_dir: &Path, mut on_script_path: impl FnMut(PathBuf)) {
    let glob_results = match glob::glob(&format!("{}/**/ginger.txt", root_dir.to_string_lossy())) {
        Ok(it) => it,
        Err(err) => {
            warn!("glob: {:?}", err);
            return;
        }
    };

    for manifest_path in glob_results.flatten() {
        // manifest_path = `project_dir/ginger.txt`
        let project_dir = match manifest_path.parent() {
            Some(it) => it,
            None => continue,
        };

        let contents = match fs::read_to_string(&manifest_path).ok() {
            Some(it) => it,
            None => continue,
        };

        for (line_index, line_text) in contents.lines().enumerate() {
            // 前後のスペースを除去する。空行は無視する
            let line_text = line_text.trim();
            if line_text.is_empty() {
                continue;
            }

            // 各行には、マニフェストファイルがあるディレクトリ内のファイル名、またはそのディレクトリを基準とする相対パスが書かれている
            let script_path = project_dir.join(line_text);

            if !script_path.exists() {
                warn!(
                    "{:?}:{} - ファイルがみつかりません",
                    script_path, line_index
                );
                continue;
            }

            on_script_path(script_path);
        }
    }
}

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
