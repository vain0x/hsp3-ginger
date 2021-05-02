#![cfg(test)]

use crate::{
    lang_service::{LangService, LangServiceOptions},
    utils::canonical_uri::CanonicalUri,
};
use lsp_types::{Position, Url};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum DefOrUse {
    Def,
    Use,
}

fn path_to_uri(path: PathBuf) -> Url {
    CanonicalUri::from_file_path(&path).unwrap().into_url()
}

// 仕組み:
// ソースファイルのコメントに `@def IDENT` や `@use IDENT` という目印を書いておく。
// 各シンボルの定義・使用箇所を調べて、`@def` と書かれた行が定義箇所として検出され、`@use` が書かれた行が使用箇所として検出されていたら成功。過不足があったら失敗。
#[test]
fn symbols_tests() {
    let hsp3_home: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../vendor/hsp3");
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests");

    let options = LangServiceOptions {
        lint_enabled: false,
        watcher_enabled: false,
    };

    let mut ls = LangService::new(PathBuf::from(hsp3_home), options);

    let mut texts = HashMap::new();

    // 識別子 -> 行番号
    let mut map: HashMap<String, Vec<(DefOrUse, Url, usize)>> = HashMap::new();

    for filename in &[
        "symbols/scope_deffunc.hsp",
        "symbols/scope_deffunc_other.hsp",
    ] {
        let path = tests_dir.join(filename);
        let text = fs::read_to_string(&path).expect("read");
        let uri = path_to_uri(path);

        let lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
        for (row, line) in lines.iter().enumerate() {
            match line.find("@def") {
                None => {}
                Some(i) => {
                    let word = match line[i + "@def".len()..].split_ascii_whitespace().next() {
                        Some(it) => it,
                        None => panic!("@defの後ろにシンボルがありません。{:?}:{}", uri, row + 1),
                    };
                    map.entry(word.to_string()).or_default().push((
                        DefOrUse::Def,
                        uri.clone(),
                        row,
                    ));
                }
            }

            match line.find("@use") {
                None => {}
                Some(i) => {
                    let word = match line[i + "@use".len()..].split_ascii_whitespace().next() {
                        Some(it) => it,
                        None => panic!("@useの後ろにシンボルがありません。{:?}:{}", uri, row + 1),
                    };
                    map.entry(word.to_string()).or_default().push((
                        DefOrUse::Use,
                        uri.clone(),
                        row,
                    ));
                }
            }
        }

        texts.insert(uri.clone(), lines);
        ls.open_doc(uri, 1, text);
    }

    const EXCLUDE_DEFINITION: bool = false;

    assert!(!map.is_empty());

    for (word, mut expected) in map {
        let (_, uri, row) = expected.first().unwrap().clone();
        let line = &texts[&uri][row];
        let column = line.find(&word).unwrap();

        let pos = Position::new(row as u64, column as u64);
        let def_sites = ls.definitions(uri.clone(), pos);
        let use_sites = ls.references(uri.clone(), pos, EXCLUDE_DEFINITION);

        let mut actual = def_sites
            .into_iter()
            .map(|loc| (DefOrUse::Def, loc.uri, loc.range.start.line as usize))
            .chain(
                use_sites
                    .into_iter()
                    .map(|loc| (DefOrUse::Use, loc.uri, loc.range.start.line as usize)),
            )
            .collect::<Vec<_>>();

        expected.sort();
        actual.sort();

        if expected != actual {
            eprintln!(
                "uri: {:?}\npos: {:?}\nword: {}\nactual: {:#?}\nexpected: {:#?}",
                uri, pos, word, actual, expected
            );

            let redundant = actual
                .iter()
                .filter(|loc| !expected.contains(loc))
                .collect::<Vec<_>>();
            let missing = expected
                .iter()
                .filter(|loc| !actual.contains(loc))
                .collect::<Vec<_>>();
            panic!("{}\n\n{}\n{}\n\n過剰: {:#?}, 不足: {:#?}",
                "定義・使用箇所の結果が期待通りではありませんでした。",
                "- 過剰分は定義・使用箇所として検出されましたが、@def/@useが書かれていません。",
                "- 不足分は逆に@def/@useが書かれていますが、定義・使用箇所として検出されていません。",
                redundant, missing
            )
        }
    }
}
