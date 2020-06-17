#![cfg(test)]

use super::{parse_root, PToken};
use crate::{analysis::ADoc, utils::rc_str::RcStr};
use encoding::{codec::utf_8::UTF8Encoding, DecoderTrap, Encoding, StringWriter};
use std::{
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

// FIXME: tokenize_tests と重複
#[test]
fn parse_standard_files() {
    // FIXME: 環境変数から読む？
    let hsp3_root: &str = if cfg!(windows) {
        "C:/bin/hsp36b2"
    } else {
        concat!(env!("HOME"), "/bin/hsp36beta")
    };

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
            tests_dir.join("parse").join(&format!("{}.txt", name))
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
            let doc = {
                last_id += 1;
                ADoc::new(last_id)
            };
            let tokens = crate::token::tokenize(doc, RcStr::new(text.clone(), 0, text.len()));
            let tokens = PToken::from_tokens(tokens);
            let root = parse_root(tokens);
            format!("{:#?}\n", root)
        };

        if previous_output_opt.map_or(true, |previous| previous != output) {
            fs::write(output_path, output).unwrap();
        }
    }

    if last_id == 0 {
        panic!("no files");
    }
}

/// ファイルを shift_jis または UTF-8 として読む。
fn read_file(file_path: &Path, out: &mut impl StringWriter) -> bool {
    // utf-8?
    let content = match fs::read(file_path).ok() {
        None => return false,
        Some(x) => x,
    };

    // shift-jis?
    encoding::all::WINDOWS_31J
        .decode_to(&content, DecoderTrap::Strict, out)
        .or_else(|_| UTF8Encoding.decode_to(&content, DecoderTrap::Strict, out))
        .is_ok()
}
