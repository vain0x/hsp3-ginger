//! (LSPサーバー以外の) CLIコマンドの処理

use crate::{
    parse::{parse_root, PToken},
    source::DocId,
    utils::{rc_str::RcStr, read_file::read_file},
};
use std::{
    hash::{Hash, Hasher},
    io::{stdin, stdout, Read, Write},
    path::PathBuf,
    time::Duration,
};

/// ファイルを構文解析して構文木を出力する (出力内容は仮)
pub fn parse(files: Vec<String>) {
    assert!(!files.is_empty());
    let multiple = files.len() != 0;

    for (file_index, filename) in files.into_iter().enumerate() {
        let text = if filename == "-" {
            let mut buf = String::new();
            stdin().read_to_string(&mut buf).unwrap();
            buf
        } else {
            let mut buf = String::new();
            if !read_file(&PathBuf::from(&filename), &mut buf) {
                panic!("ERROR: Cannot read {filename:?}");
            }
            buf
        };

        let doc: DocId = (file_index as usize) + 1;
        let text = RcStr::from(text);

        let tokens = crate::token::tokenize(doc, text);
        let tokens = PToken::from_tokens(tokens.into());
        let root = parse_root(tokens);

        let mut out = stdout().lock();
        if multiple {
            write!(out, "file: {filename}\n{root:?}\n").unwrap();
        } else {
            write!(out, "{root:?}\n").unwrap();
        }
    }
}

/// 字句解析・構文解析にかかる時間を測る
///
/// (HSP3のインストールディレクトリのcommon, sampleにあるファイルをそれぞれパースしてトータルの時間を図る)
pub fn profile_parse(hsp3_root: PathBuf) {
    let paths = {
        let root = hsp3_root.to_str().unwrap();
        vec![
            glob::glob(&format!("{}/common/**/*.hsp", root)).unwrap(),
            glob::glob(&format!("{}/common/**/*.as", root)).unwrap(),
            glob::glob(&format!("{}/sample/**/*.hsp", root)).unwrap(),
            glob::glob(&format!("{}/sample/**/*.as", root)).unwrap(),
        ]
    };

    let mut results = vec![];
    let mut total = Duration::ZERO;
    let mut count = 0_usize;

    for (i, path) in paths.into_iter().flatten().enumerate() {
        let path = path.unwrap();

        let doc: DocId = i as DocId;
        let mut text = String::new();
        if !read_file(&path, &mut text) {
            panic!("Cannot open {path:?}");
        }
        let text = RcStr::from(text);
        // let text_len = text.len();

        let s = std::time::SystemTime::now();
        let tokens = crate::token::tokenize(doc, text);
        let tokens = PToken::from_tokens(tokens.into());
        let root = parse_root(tokens);
        let t = std::time::SystemTime::now();

        let dt = t.duration_since(s).unwrap();
        total += dt;
        count += 1;

        // consume output
        results.push(format!("{:#?}\n", root));
    }

    // consume output
    let h = {
        let mut h = std::hash::DefaultHasher::new();
        results.hash(&mut h);
        h.finish()
    };
    println!("hash={}", h % 256);
    println!("total={}ms, count={count}", total.as_millis());

    // [ms]
    let average = ((total.as_micros() as f64) / (count as f64)).round() / 1000.0;
    println!("result: {average}ms");
}
