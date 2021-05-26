use super::*;
use crate::source::DocId;

pub(crate) fn search_common(
    hsp3_home: &Path,
    docs: &mut Docs,
    common_docs: &mut HashMap<String, DocId>,
) {
    // trace!("commonディレクトリにあるファイルを開きます。");
    let common_dir = hsp3_home.join("common");

    let patterns = match common_dir.to_str() {
        Some(dir) => vec![format!("{}/**/*.hsp", dir), format!("{}/**/*.as", dir)],
        None => vec![],
    };

    for path in patterns
        .into_iter()
        .flat_map(|pattern| glob::glob(&pattern).unwrap())
        .flat_map(|result| result.ok())
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
