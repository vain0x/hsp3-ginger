//! HSP Help Source (.hs) ファイルの解析

use crate::utils::read_file::read_file;
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

const EOL: &str = "\r\n";

/// ヘルプソースファイルから抽出したシンボル情報
#[derive(Clone, Debug, Default)]
pub(crate) struct HsSymbol {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) documentation: Vec<String>,
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

        if let Some(prm) = map.get("prm") {
            documentation.push(prm.join(EOL));
        }

        if let Some(inst) = map.get("inst") {
            documentation.push(inst.join(EOL));
        }

        if let Some(note) = map.get("note") {
            documentation.push(note.join(EOL));
        }

        symbols.push(HsSymbol {
            name: name.trim().to_string(),
            description,
            documentation,
            ..Default::default()
        });
    }
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
