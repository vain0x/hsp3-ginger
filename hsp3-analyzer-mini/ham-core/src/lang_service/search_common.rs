use super::*;
use crate::source::DocId;

pub(crate) fn search_common(
    hsp3_root: &Path,
    docs: &mut Docs,
    common_docs: &mut HashMap<String, DocId>,
) {
    // trace!("commonディレクトリにあるファイルを開きます。");
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

            let doc = docs.ensure_file_opened(&path)?;
            // trace!("common/{} => doc={}", relative, doc);
            common_docs.insert(relative, doc);
            None
        })();
    }
}
