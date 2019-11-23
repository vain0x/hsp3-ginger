//! Parse HSP Help Source (.hs) files for completion

use encoding::{
    codec::utf_8::UTF8Encoding, label::encoding_from_windows_code_page, DecoderTrap, Encoding,
    StringWriter,
};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const EOL: &str = "\r\n";

#[derive(Clone, Debug, Default)]
pub(crate) struct HsSymbol {
    pub name: String,
    pub description: Option<String>,
    pub documentation: Vec<String>,
}

fn str_is_whitespace(s: &str) -> bool {
    s.chars().all(|c| c.is_whitespace())
}

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

// Read a file as shift_jis or UTF-8.
fn read_file(file_path: &Path, out: &mut impl StringWriter, shift_jis: &dyn Encoding) -> bool {
    let content = match fs::read(file_path).ok() {
        None => return false,
        Some(x) => x,
    };

    shift_jis
        .decode_to(&content, DecoderTrap::Strict, out)
        .or_else(|_| UTF8Encoding.decode_to(&content, DecoderTrap::Strict, out))
        .is_ok()
}

fn parse_for_symbols(
    file_path: &str,
    content: &str,
    symbols: &mut Vec<HsSymbol>,
    warnings: &mut Vec<String>,
) {
    // Split into sections:

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

    // Parse sections:

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

    // Merge defaults:

    let default_map = maps.drain(..1).next().unwrap();

    for map in maps.iter_mut() {
        for (k, v) in &default_map {
            if !map.contains_key(k) {
                map.insert(k.clone(), v.clone());
            }
        }
    }

    // Remove leading blank lines:

    for map in maps.iter_mut() {
        for (_, v) in map.iter_mut() {
            while v.get(0).filter(|s| str_is_whitespace(s)).is_some() {
                v.remove(0);
            }
        }
    }

    // Emit symbols:

    for mut map in maps {
        let index_lines = match map.get_mut("index") {
            None => {
                // unreachable?
                warnings.push(format!("missing %index in {}", file_path));
                continue;
            }
            Some(index_lines) => index_lines,
        };

        let name = match index_lines.drain(..1).next() {
            None => {
                warnings.push(format!("empty %index found in {}", file_path));
                continue;
            }
            Some(name) => name,
        };

        let description =
            Some(index_lines.join(EOL).trim().to_string()).filter(|s| !str_is_whitespace(s));

        let mut documentation = vec![];

        if let Some(prm) = map.get("prm") {
            documentation.push(prm.join(EOL).trim().to_string());
        }

        if let Some(inst) = map.get("inst") {
            documentation.push(inst.join(EOL).trim().to_string());
        }

        symbols.push(HsSymbol {
            name: name.trim().to_string(),
            description,
            documentation,
            ..Default::default()
        });
    }
}

pub(crate) fn collect_all_symbols(
    hsp_root: &Path,
    file_count: &mut usize,
    symbols: &mut Vec<HsSymbol>,
    warnings: &mut Vec<String>,
) -> io::Result<()> {
    let shift_jis = encoding_from_windows_code_page(932).unwrap();

    let hsphelp_dir = hsp_root.join("hsphelp");

    let mut help_files = vec![];
    read_dir(&hsphelp_dir, &mut help_files)?;

    let mut content = String::new();
    for file in help_files {
        content.clear();

        if !read_file(&file, &mut content, shift_jis) {
            warnings.push(format!(
                "File {} can't load or not utf-8 or shift_jis",
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
