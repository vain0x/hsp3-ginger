#![cfg(test)]

use super::*;
use crate::{
    lang_service::{docs::NO_VERSION, LangService},
    source::{DocId, Pos, Pos16},
    token::tokenize,
};
use expect_test::expect_file;
use lsp_types::{Position, Url};

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
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests");

    let mut ls = LangService::new_standalone();

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

    let ls = ls.compute_ref();

    const EXCLUDE_DEFINITION: bool = false;

    assert!(!map.is_empty());

    for (word, mut expected) in map {
        let (_, uri, row) = expected.first().unwrap().clone();
        let line = &texts[&uri][row];
        let column = line.find(&word).unwrap();

        let pos = Position::new(row as u32, column as u32);
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

// 仕組み:
// ソースファイルのコメントに `^def WORD` や `^use WORD` という目印を書いておく。
// `^` が指している位置 (`^` の1つ上の行の同じ列) に対してreferencesリクエストを送る。
//

// 各シンボルの定義・使用箇所を調べて、`@def` と書かれた行が定義箇所として検出され、`@use` が書かれた行が使用箇所として検出されていたら成功。過不足があったら失敗。
#[test]
fn namespace_tests() {
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests");

    fn rows(rows: usize) -> Pos16 {
        Pos16::new(rows as u32, 0)
    }

    fn to_pos16(p: Position) -> Pos16 {
        Pos16::new(p.line as u32, p.character as u32)
    }

    for filenames in &[
        &["symbols/namespace_deffunc_global.hsp"],
        &["symbols/namespace_deffunc_local.hsp"],
        &["symbols/namespace_deffunc_qualified.hsp"],
    ] {
        let mut ls = LangService::new_standalone();

        // 各ファイルの内容を行ごとに分割したもの。
        let mut lines_map: HashMap<Url, Vec<String>> = HashMap::new();

        // 各識別子に関して期待される定義・使用箇所のマップ
        let mut word_map: HashMap<String, Vec<(DefOrUse, Url, Pos16)>> = HashMap::new();

        for filename in filenames.iter() {
            let path = tests_dir.join(filename);
            let text = fs::read_to_string(&path)
                .expect("read")
                .replace("\t", "    ");
            let uri = path_to_uri(path);

            let lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
            for (row, line) in lines.iter().enumerate().skip(1) {
                if let Some(i) = line.find("^def") {
                    let word = match line[i + "^def".len()..].split_ascii_whitespace().next() {
                        Some(it) => it,
                        None => panic!("^defの後ろに単語が必要です。{:?}:{}", uri, row + 1),
                    };
                    word_map.entry(word.to_string()).or_default().push((
                        DefOrUse::Def,
                        uri.clone(),
                        rows(row - 1) + Pos16::from(&line[..i]),
                    ));
                }

                if let Some(i) = line.find("^use") {
                    let word = match line[i + "^use".len()..].split_ascii_whitespace().next() {
                        Some(it) => it,
                        None => panic!("^useの後ろに単語が必要です。{:?}:{}", uri, row + 1),
                    };
                    word_map.entry(word.to_string()).or_default().push((
                        DefOrUse::Use,
                        uri.clone(),
                        rows(row - 1) + Pos16::from(&line[..i]),
                    ));
                }
            }

            lines_map.insert(uri.clone(), lines);
            ls.open_doc(uri, NO_VERSION, text);
        }

        let ls = ls.compute_ref();

        assert!(!word_map.is_empty(), "^def/^useがみつかるはず");

        for (word, mut expected) in word_map {
            let (_, uri, pos) = expected.first().unwrap().clone();

            const EXCLUDE_DEFINITION: bool = false;
            let pos = Position::new(pos.row as u32, pos.column as u32);
            let def_sites = ls.definitions(uri.clone(), pos);
            let use_sites = ls.references(uri.clone(), pos, EXCLUDE_DEFINITION);

            let mut actual = def_sites
                .into_iter()
                .map(|loc| (DefOrUse::Def, loc.uri, to_pos16(loc.range.start)))
                .chain(
                    use_sites
                        .into_iter()
                        .map(|loc| (DefOrUse::Use, loc.uri, to_pos16(loc.range.start))),
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
                    .filter(|x| !expected.contains(x))
                    .map(|(kind, uri, pos)| format!("  - {:?} {}:{}", kind, uri, pos))
                    .collect::<Vec<_>>()
                    .join("\n");
                let missing = expected
                    .iter()
                    .filter(|x| !actual.contains(x))
                    .map(|(kind, uri, pos)| format!("  - {:?} {}:{}", kind, uri, pos))
                    .collect::<Vec<_>>()
                    .join("\n");
                panic!("{}\n\n{}\n{}\n\n過剰:\n{}\n不足:\n{}",
                    "定義・使用箇所の結果が期待通りではありませんでした。",
                    "- 過剰分は定義・使用箇所として検出されましたが、^def/^useが書かれていません。",
                    "- 不足分は逆に^def/^useが書かれていますが、定義・使用箇所として検出されていません。",
                    redundant, missing
                    )
            }
        }
    }
}

const NO_DOC: DocId = 1;

fn to_pos16(p: Position) -> Pos16 {
    Pos16::new(p.line as u32, p.character as u32)
}

fn apply_edits(text: &str, mut edits: Vec<lsp_types::TextEdit>) -> String {
    // Pos16からインデックスへのマップを作る。
    let mut rev = HashMap::new();
    let mut p = Pos16::new(0, 0);
    for (i, c) in text.char_indices() {
        rev.insert(p, i);
        p += Pos16::from(c);
    }
    rev.insert(p, text.len());

    // 編集をマージする。
    let mut edits = {
        edits.sort_by_key(|e| e.range.start);
        edits.into_iter()
    };
    let mut output = String::new();
    let mut i = 0;
    loop {
        let edit_opt = edits.next();

        let start = edit_opt
            .as_ref()
            .map(|e| rev[&to_pos16(e.range.start)])
            .unwrap_or(text.len());
        debug_assert!(i <= start);

        output += &text[i..start];

        let edit = match edit_opt {
            Some(it) => it,
            None => break,
        };
        output += &edit.new_text;
        i = rev[&to_pos16(edit.range.end)];
    }

    output
}

/// フォーマッティングのテスト。
#[test]
fn formatting_tests() {
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests");

    fn collect_indent_markers(text: &str) -> Vec<(Pos, usize)> {
        let mut v = vec![];
        for (i, _) in text.match_indices("^indent=") {
            let s = &text[i..];
            let n = {
                let s = &s["^indent=".len()..];
                let c = s.chars().next().unwrap();
                assert!(c.is_ascii_digit());
                (c as u8 - b'0') as usize
            };
            let pos = Pos::from(&text[..i]);
            v.push((pos, n));
        }
        v
    }

    fn check_indents(text: &str) -> Vec<(Pos, usize)> {
        let lines = text.lines().collect::<Vec<_>>();
        let mut v = vec![];

        for (mark_pos, _) in collect_indent_markers(text) {
            debug_assert_ne!(mark_pos.row, 0);

            let row = mark_pos.row as usize - 1;
            let line = &lines[row];

            let mut n = 0;

            if let Some(t) = tokenize(NO_DOC, RcStr::from(line.to_string()))
                .into_iter()
                .take_while(|t| t.kind.is_space())
                .next()
            {
                for c in t.text.chars().skip_while(|&c| c == '\r' || c == '\n') {
                    if c == '\t' {
                        n += 1;
                    } else {
                        n = 0;
                        break;
                    }
                }
            }

            v.push((mark_pos, n));
        }
        v
    }

    for filename in &["formatting/indent.hsp"] {
        let path = tests_dir.join(filename);
        let text = fs::read_to_string(&path).expect("read");
        let uri = path_to_uri(path);

        let expected = collect_indent_markers(&text);
        assert_ne!(expected.len(), 0);

        let formatted = {
            let mut ls = LangService::new_standalone();
            ls.open_doc(uri.clone(), NO_VERSION, text.to_string());
            let edits = ls
                .compute_ref()
                .formatting(uri.clone())
                .expect("formatting");
            apply_edits(&text, edits)
        };

        let actual = check_indents(&formatted);

        let f = |xs: Vec<(Pos, usize)>| {
            xs.into_iter()
                .map(|(pos, n)| (Pos16::from(pos), n))
                .collect::<Vec<_>>()
        };

        let expected = f(expected);
        let actual = f(actual);

        if actual != expected {
            eprintln!(
                "uri: {:?}\nactual: {:#?}\nexpected: {:#?}\nformatted: {}",
                uri, actual, expected, formatted,
            );

            let redundant = actual
                .iter()
                .filter(|x| !expected.contains(x))
                .map(|(pos, n)| format!("  - {}:{} n={}", uri, pos, n))
                .collect::<Vec<_>>()
                .join("\n");
            let missing = expected
                .iter()
                .filter(|x| !actual.contains(x))
                .map(|(pos, n)| format!("  - {}:{} n={}", uri, pos, n))
                .collect::<Vec<_>>()
                .join("\n");
            panic!(
                "{}\n\n過剰:\n{}\n不足:\n{}",
                "フォーマッティングの結果が期待通りではありませんでした。", redundant, missing
            )
        }
    }
}

#[test]
fn formatting_blank_test() {
    let text = include_str!["../../tests/formatting/blank.hsp"];
    let expected = expect_file!["../../tests/formatting/blank.expected.hsp"];

    let uri = CanonicalUri::from_file_path(&PathBuf::from("blank.hsp"))
        .unwrap()
        .into_url();

    let actual = {
        let mut ls = LangService::new_standalone();
        ls.open_doc(uri.clone(), NO_VERSION, text.to_string());
        let edits = ls.compute_ref().formatting(uri).expect("formatting");
        apply_edits(&text, edits)
    };

    expected.assert_eq(&actual);
}
