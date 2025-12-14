// ディレクトリを探索してファイルを収集する機能

use super::*;

/// `common` ディレクトリを探索してファイルを収集する
pub(super) fn scan_common(
    hsp3_root: &Path,
    doc_interner: &mut DocInterner,
    docs: &mut Docs,
    common_docs: &mut HashMap<String, DocId>,
) {
    debug!("scan_common");

    let common_dir = hsp3_root.join("common");

    let patterns = match common_dir.to_str() {
        Some(dir) => vec![format!("{}/**/*.hsp", dir), format!("{}/**/*.as", dir)],
        None => vec![],
    };

    // 条件コンパイルが実装されていないのでhspdef経由でhsp261cmpをincludeしているとみなされるが、不要なので読み込まない。
    let is_excluded = |path: &Path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map_or(false, |name| name == "hsp261cmp.as")
    };

    for path in patterns
        .into_iter()
        .flat_map(|pattern| glob::glob(&pattern).unwrap())
        .flat_map(|result| result.ok())
        .filter(|path| !is_excluded(&path))
    {
        (|| -> Option<()> {
            // commonに対する相対パス
            let relative = path
                .strip_prefix(&common_dir)
                .ok()?
                .to_string_lossy()
                .replace("\\", "/");

            let (_, doc) = doc_interner.intern(&CanonicalUri::from_abs_path(&path)?);
            docs.ensure_file_opened(doc, &path)?;
            // debug!("common/{} => doc={}", relative, doc);
            common_docs.insert(relative, doc);
            None
        })();
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
