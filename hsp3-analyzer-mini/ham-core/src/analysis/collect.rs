// analysisというよりinputとかproject_modelとよぶべき

use std::{fs, path::PathBuf};

pub(crate) fn collect_project_files(root_dir: PathBuf) -> Vec<PathBuf> {
    // let root_dir_opt = self.root_uri_opt.as_ref().and_then(|x| x.to_file_path());
    let root_dir_opt = Some(root_dir);

    let project_files = root_dir_opt
        .into_iter()
        .filter_map(|dir| glob::glob(&format!("{}/**/ginger.txt", dir.to_str()?)).ok())
        .flatten()
        .filter_map(|path_opt| path_opt.ok())
        .filter_map(|path| Some((path.clone(), fs::read_to_string(&path).ok()?)));
    let mut entrypoints = vec![];
    for (path, contents) in project_files {
        let dir = path.parent();
        let docs = contents
            .lines()
            .enumerate()
            .map(|(i, line)| (i, line.trim_end()))
            .filter(|&(_, line)| line != "")
            .filter_map(|(i, name)| {
                let name = dir?.join(name);
                if !name.exists() {
                    warn!("ファイルがありません {:?}:{}", path, i);
                    return None;
                }

                // let doc = match self.docs.ensure_file_opened(&name) {
                //     Some(it) => it,
                //     None => {
                //         warn!("ファイルをopenできません。{:?}", name);
                //         return None;
                //     }
                // };
                // Some(doc)
                Some(name)
            });

        entrypoints.extend(docs);
    }

    trace!(
        "entrypoints={:?}",
        entrypoints // .iter()
                    // .map(|&doc| self.docs.get_uri(doc).ok_or(doc))
                    // .collect::<Vec<_>>()
    );
    entrypoints
}
