//! HSP Help Source (.hs) ファイルの解析

use super::*;
use crate::utils::read_file::read_file;

const EOL: &str = "\r\n";

/// ヘルプソースファイルから抽出したシンボル情報
#[derive(Default)]
pub(crate) struct HsSymbol {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) documentation: Vec<String>,
    pub(crate) params_opt: Option<Vec<HsParamInfo>>,

    /// 標準命令か関数？
    pub(crate) builtin: bool,
}

fn str_is_whitespace(s: &str) -> bool {
    s.chars().all(|c| c.is_whitespace())
}

/// ディレクトリにあるヘルプソースファイルを列挙する
fn read_dir(hsphelp_dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(&hsphelp_dir)? {
        let entry = entry?;

        if entry.path().extension().map_or(true, |ext| ext != "hs") {
            continue;
        }

        out.push(hsphelp_dir.join(entry.path()));
    }

    Ok(())
}

/// ヘルプソースファイルを解析してシンボル情報を集める。
fn parse_for_symbols(
    file_path: &str,
    content: &str,
    symbols: &mut Vec<HsSymbol>,
    warnings: &mut Vec<String>,
) {
    // セクションに分割する:

    let mut sections = vec![];
    {
        let mut section = vec![];
        let mut in_html = false;

        for line in content.lines() {
            if line.starts_with(";") {
                continue;
            }

            if in_html {
                if line.starts_with("}html") {
                    section.push("<Some HTML contents>");
                    in_html = false;
                }
                continue;
            } else if line.starts_with("html{") {
                in_html = true;
                continue;
            }

            if line.to_lowercase().starts_with("%index") {
                sections.push(section.clone());
                section.clear();
            }

            section.push(line);
        }
        sections.push(section);
    }

    // セクションを解析する:

    let mut maps = vec![];

    for section in sections {
        let mut map = HashMap::new();
        let mut key: Option<String> = None;
        let mut value = vec![];

        for line in section {
            if line.starts_with("%") {
                if let Some(key) = key {
                    map.insert(key.clone(), value.clone());
                    value.clear();
                }

                let name = line[1..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphabetic())
                    .collect::<String>();

                key = Some(name.to_string());
                continue;
            }

            value.push(line);
        }

        if let Some(key) = key {
            map.insert(key.clone(), value.clone());
            value.clear();
        }

        maps.push(map.clone());
        map.clear();
    }

    // 不要な行やセクションを削除する。

    for map in maps.iter_mut() {
        for (_, v) in map.iter_mut() {
            // 制御記号の削除
            v.retain(|s| s.trim() != "^p" && s.trim() != "^");

            let mut retain = vec![true; v.len()];

            // 連続する空行の削除
            let blank = v.iter().map(|s| str_is_whitespace(s)).collect::<Vec<_>>();

            for i in 0..v.len() {
                if (i == 0 || blank[i - 1]) && blank[i] {
                    retain[i] = false;
                }
            }

            // 後方の空行の削除
            for i in (0..v.len()).rev().take_while(|&i| blank[i]) {
                retain[i] = false;
            }

            let mut i = 0;
            v.retain(|_| {
                i += 1;
                retain[i - 1]
            });
        }

        map.retain(|_, v| !v.is_empty());
    }

    // セクションの既定値を合成する。

    let default_map = maps.drain(..1).next().unwrap();

    for map in maps.iter_mut() {
        for (k, v) in &default_map {
            if !map.contains_key(k) {
                map.insert(k.clone(), v.clone());
            }
        }
    }

    // シンボル情報を構築する。

    for mut map in maps {
        let index_lines = match map.get_mut("index") {
            None => {
                // unreachable?
                warnings.push(format!("%index がみつかりません {}", file_path));
                continue;
            }
            Some(index_lines) => index_lines,
        };

        let name = match index_lines.drain(..1).next() {
            None => {
                // unreachable?
                warnings.push(format!("%index が空です {}", file_path));
                continue;
            }
            Some(name) => name,
        };

        let description = Some(index_lines.join(EOL));

        let mut documentation = vec![];
        let mut params_opt = None;
        let mut builtin = false;

        if let Some(prm) = map.get("prm") {
            params_opt = Some(parse_prm_section(prm));
            documentation.push(prm.join(EOL));
        }

        if let Some(inst) = map.get("inst") {
            documentation.push(inst.join(EOL));
        }

        if let Some(note) = map.get("note") {
            if !name.starts_with("#")
                && !name.starts_with("_")
                && note
                    .iter()
                    .any(|s| s.contains("標準命令") || s.contains("標準関数"))
            {
                builtin = true;
            }

            documentation.push(note.join(EOL));
        }

        symbols.push(HsSymbol {
            name: name.trim().to_string(),
            description,
            documentation,
            params_opt,
            builtin,
        });
    }
}

pub(crate) struct HsParamInfo {
    pub(crate) name: String,
    pub(crate) details_opt: Option<String>,
}

/// `%prm` の中身を解析する。
///
/// 先頭行はシグネチャとみなす。
/// 命令の場合はパラメータの名前をカンマ区切りで並べて p1, p2 のように書く。
/// 関数の場合は全体をカッコで囲んで `(p1, p2)` とかく。
/// パラメータの名前の代わりに文字列リテラルを使ってもよい。
///
/// 先頭以外の行は各パラメータの説明書きとみなす。
/// 行の先頭にパラメータ名があったら、その行はそのパラメータの説明とみなす。
/// また、以降の行が空白で始まっている (字下げされている) 限り、その説明が続くとみなす。
/// 例:
///
/// ```hs
/// %prm
/// "message", mode
///
/// "message": 表示する文字列
/// mode (0): 表示するモード
///           省略したら0
/// ```
fn parse_prm_section(prm: &[&str]) -> Vec<HsParamInfo> {
    let mut params: Vec<HsParamInfo>;

    // 先頭行:
    {
        let mut s = match prm.first() {
            Some(it) => *it,
            None => "",
        };

        if s.starts_with('(') {
            s = s[1..].trim_end_matches(')').trim();
        }

        params = s
            .split(",")
            .map(|name| HsParamInfo {
                name: name.trim().to_string(),
                details_opt: None,
            })
            .collect();
    }

    // 先頭以外:
    let mut row = 1;
    loop {
        // 空行を飛ばす。
        row += prm[row..]
            .iter()
            .take_while(|s| s.trim_start().is_empty())
            .count();
        if row >= prm.len() {
            break;
        }

        // このブロックの行数を調べる。
        let count = {
            1 + prm[row + 1..]
                .iter()
                .take_while(|s| match s.chars().next() {
                    Some(c) => c.is_whitespace(),
                    None => false,
                })
                .count()
        };
        let details = prm[row..row + count].join("\n");
        row += count;

        if let Some(p) = params
            .iter_mut()
            .find(|p| details.starts_with(&p.name) && p.details_opt.is_none())
        {
            p.details_opt = Some(details);
        }
    }

    params
}

/// ディレクトリに含まれるすべてのヘルプソースファイルからすべてのシンボル情報を抽出する。
pub(crate) fn collect_all_symbols(
    hsp3_home: &Path,
    file_count: &mut usize,
    symbols: &mut Vec<HsSymbol>,
    warnings: &mut Vec<String>,
) -> io::Result<()> {
    let hsphelp_dir = hsp3_home.join("hsphelp");

    let mut help_files = vec![];
    read_dir(&hsphelp_dir, &mut help_files)?;

    let mut content = String::new();
    for file in help_files {
        content.clear();

        if !read_file(&file, &mut content) {
            warnings.push(format!(
                "ファイル {} は開けないか、shift_jis でも UTF-8 でもありません。",
                file.to_str().unwrap_or("???.hs")
            ));
            continue;
        }

        parse_for_symbols(
            file.to_str().unwrap_or("???.hs"),
            &content,
            symbols,
            warnings,
        );
        *file_count += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn test_parse_prm_section() {
        let prm = r#""message", model, mode
"message": 表示する文字列
mode (0): モード
          これは2行目。
model: モデル"#;

        let lines = prm.lines().collect::<Vec<_>>();
        let params = parse_prm_section(&lines);
        expect![[r#"
            [
                (
                    "\"message\"",
                    Some(
                        "\"message\": 表示する文字列",
                    ),
                ),
                (
                    "model",
                    Some(
                        "model: モデル",
                    ),
                ),
                (
                    "mode",
                    Some(
                        "mode (0): モード\n          これは2行目。",
                    ),
                ),
            ]"#]]
        .assert_eq(&format!(
            "{:#?}",
            params
                .into_iter()
                .map(|p| (p.name, p.details_opt))
                .collect::<Vec<_>>()
        ));
    }

    #[test]
    fn test_parse_prm_section_func() {
        let prm = r#"(n)
n 数値"#;

        let lines = prm.lines().collect::<Vec<_>>();
        let params = parse_prm_section(&lines);

        expect![[r#"
            [
                (
                    "n",
                    Some(
                        "n 数値",
                    ),
                ),
            ]"#]]
        .assert_eq(&format!(
            "{:#?}",
            params
                .into_iter()
                .map(|p| (p.name, p.details_opt))
                .collect::<Vec<_>>()
        ));
    }
}
