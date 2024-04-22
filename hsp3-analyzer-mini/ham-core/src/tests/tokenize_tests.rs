#![cfg(test)]

use crate::{
    source::DocId,
    token,
    utils::{rc_str::RcStr, read_file::read_file},
};
use std::{fs, path::PathBuf, rc::Rc};

#[test]
#[cfg_attr(not(feature = "full_test"), ignore)]
fn tokenize_standard_files() {
    let hsp3_root: &str = option_env!("HSP3_ROOT")
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../vendor/hsp3"));

    let tests_dir = {
        let project_dir: &'static str = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(project_dir).join("../tests")
    };

    let mut last_id = 0;
    let mut text = Rc::new(String::new());

    let paths = vec![
        glob::glob(&format!("{}/common/**/*.hsp", hsp3_root)).unwrap(),
        glob::glob(&format!("{}/common/**/*.as", hsp3_root)).unwrap(),
        glob::glob(&format!("{}/sample/**/*.hsp", hsp3_root)).unwrap(),
        glob::glob(&format!("{}/sample/**/*.as", hsp3_root)).unwrap(),
    ];
    for path in paths.into_iter().flatten() {
        let path = path.unwrap();

        let output_path = {
            let name = path
                .strip_prefix(&hsp3_root)
                .unwrap()
                .to_str()
                .unwrap()
                .replace("/", "...")
                .replace("\\", "...");
            tests_dir.join("tokenize").join(&format!("{}.txt", name))
        };

        let previous_output_opt = fs::read_to_string(&output_path).ok();

        {
            let text = Rc::get_mut(&mut text).unwrap();
            text.clear();
            if !read_file(&path, text) {
                eprintln!("couldn't read {:?}", path);
                continue;
            }
        }

        let output = {
            let doc: DocId = {
                last_id += 1;
                last_id
            };
            let tokens = token::tokenize(doc, RcStr::from(text.to_string()));
            format!("{:#?}\n", tokens)
        };

        if previous_output_opt.map_or(true, |previous| previous != output) {
            fs::write(output_path, output).unwrap();
        }
    }

    if last_id == 0 {
        panic!("no files");
    }
}
