use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// `#include` に指定されたファイル名に絶対パスを関連づけるもの。
/// NOTE: 同名のファイルが複数のディレクトリーにある場合、誤った絶対パスに対応させてしまう。
pub(crate) struct SourceMap {
    /// 検索パスの集合。
    /// これらのパスは `#include` に指定した相対パスの基準になりうる。
    search_paths: Vec<PathBuf>,
    /// ファイル名から絶対パスへのマップ。
    resolutions: BTreeMap<String, PathBuf>,
}

impl SourceMap {
    pub fn new(hsp_dir: &Path) -> Self {
        SourceMap {
            search_paths: vec![hsp_dir.join("common")],
            resolutions: BTreeMap::new(),
        }
    }

    /// 現在の検索パスを基準として、指定されたファイル名 (相対パス) を探す。
    // NOTE: 見つかったのがディレクトリーだったらおかしなことになる。
    // (*.hsp という名前のディレクトリがないかぎり問題ない。)
    fn try_find(&self, file_name: &str) -> Option<PathBuf> {
        for search_path in self.search_paths.iter() {
            let path = search_path.join(PathBuf::from(file_name));
            if let Ok(path) = fs::canonicalize(path) {
                return Some(strip_unc_prefix(path));
            }
        }
        None
    }

    /// ファイル名と絶対パスのペアを記録する。
    fn add_file_name(&mut self, file_name: &str, full_path: PathBuf) {
        // 見つかったファイルがあるディレクトリーを検索パスに追加する。
        if let Some(parent) = full_path.parent() {
            let parent = parent.to_path_buf();
            if !self.search_paths.contains(&parent) {
                self.search_paths.push(parent);
            }
        }

        self.resolutions.insert(file_name.to_owned(), full_path);
    }

    /// すべてのファイル名の絶対パスを探索する。
    pub fn add_file_names(&mut self, file_names: &[&str]) {
        let mut file_names = file_names.to_owned();

        loop {
            // 今回のループで見つからなかったファイル名のリスト。
            let mut next = Vec::new();

            // 今回のループでどのファイル名の絶対パスも見つからなかったら true 、ループを終了する。
            let mut stuck = true;

            for file_name in file_names.drain(..) {
                if let Some(full_path) = self.try_find(file_name) {
                    self.add_file_name(file_name, full_path);
                    stuck = false;
                } else {
                    next.push(file_name);
                }
            }

            file_names = next;

            if stuck {
                break;
            }
        }
    }

    /// ファイル名に対応する絶対パスを取得する。
    pub fn resolve_file_name(&self, file_name: &str) -> Option<&Path> {
        Some(self.resolutions.get(file_name)?.as_path())
    }
}

/// UNC 形式のパスのプレフィックスを除去する。
/// Windows で `fs::canonicalize` を使うと `\\?\C:\hsp` のような形式で返ってくる。
/// `\\?\` の部分があるとパスとして認識されないことがあるので、除去する。
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    PathBuf::from(path.to_str().unwrap().replace(r#"\\?\"#, ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_map_resolution() {
        let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_files_dir = project_dir.join("tests/hsp");
        let entry_file = test_files_dir.join("deep.hsp");

        let file_names = vec![
            entry_file.to_str().unwrap(),
            "deep.hsp",
            "hspdef.as",
            "inner/inner.hsp",
            "inner_sub.hsp",
            "userdef.as",
        ];

        let mut source_map = SourceMap::new(&project_dir);
        source_map.add_file_names(&file_names);

        let inner_sub = source_map
            .resolve_file_name("inner_sub.hsp")
            .unwrap()
            .strip_prefix(&test_files_dir)
            .unwrap()
            .to_str()
            .unwrap()
            .replace("\\", "/");
        assert_eq!(&inner_sub, "inner/inner_sub.hsp");
    }
}
