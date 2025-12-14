//! hsphelp ディレクトリ内のファイル探索

use super::*;
use crate::{help_source::parse_for_symbols, source::DocId, utils::read_file::read_sjis_file};

#[derive(Default)]
pub(crate) struct HspHelpInfo {
    /// 標準命令や関数などのヘルプファイル
    pub(crate) builtin_docs: Vec<DocId>,

    /// commonのモジュールに対するヘルプファイル
    pub(crate) linked_docs: HashMap<DocId, DocId>,

    /// ヘルプファイルに含まれる情報
    pub(crate) doc_symbols: HashMap<DocId, Vec<CompletionItem>>,
}

fn is_builtin(stem: &str) -> bool {
    match stem {
        "ex_macro" | "sysval" => true,
        "i_hsp3util" => false,
        _ if stem.starts_with("i_") => true,
        _ => false,
    }
}

fn convert_symbol(hs_symbol: HsSymbol) -> (SymbolRc, CompletionItem) {
    let kind = CompletionItemKind::FUNCTION;
    let HsSymbol {
        name,
        description,
        documentation,
        params_opt,
        builtin,
    } = hs_symbol;

    let name_rc = RcStr::from(name.clone());

    let signature_opt = params_opt.map(|params| {
        let params = params
            .into_iter()
            .map(|p| (None, Some(p.name.into()), p.details_opt))
            .collect();

        Rc::new(SignatureData {
            name: name_rc.clone(),
            params,
        })
    });

    let symbol = DefInfo::HspHelp {
        name: name_rc.clone(),
        details: SymbolDetails {
            desc: description.clone().map(RcStr::from),
            docs: documentation.clone(),
        },
        builtin,
        signature_opt,
    }
    .into_symbol();

    // 補完候補の順番を制御するための文字。(標準命令を上に出す。)
    let sort_prefix = if builtin {
        'x'
    } else if !name.starts_with("_") {
        'y'
    } else {
        'z'
    };

    // '#' なし
    let word = if name.starts_with("#") {
        Some(name.as_str().chars().skip(1).collect::<String>())
    } else {
        None
    };

    let completion_item = CompletionItem {
        kind: Some(kind),
        label: name,
        detail: description,
        documentation: if documentation.is_empty() {
            None
        } else {
            Some(Documentation::String(documentation.join("\r\n\r\n")))
        },
        sort_text: Some(format!("{}{}", sort_prefix, name_rc)),
        filter_text: word.clone(),
        insert_text: word,
        ..Default::default()
    };

    (symbol, completion_item)
}

pub(crate) fn search_hsphelp(
    hsp3_root: &Path,
    common_docs: &HashMap<String, DocId>,
    doc_interner: &mut DocInterner,
    docs: &mut Docs,
    builtin_env: &mut SymbolEnv,
) -> Option<HspHelpInfo> {
    debug!("search_hsphelp");

    let hsphelp_dir = hsp3_root.join("hsphelp");

    let entries = match fs::read_dir(&hsphelp_dir) {
        Ok(it) => it,
        Err(err) => {
            warn!(
                "hsphelpを開けません。hsphelp={:?}, err={:?}",
                hsphelp_dir, err
            );
            return None;
        }
    };

    let mut info = HspHelpInfo::default();
    let mut contents = String::new();
    let mut hs_symbols = vec![];
    let mut symbols = vec![];
    let mut warnings = vec![];

    for entry_result in entries {
        debug_assert_eq!(hs_symbols.len(), 0);
        debug_assert_eq!(warnings.len(), 0);
        debug_assert_eq!(contents.len(), 0);

        (|| -> Option<()> {
            let relative_path = entry_result.ok()?.path();
            let stem = relative_path.file_stem()?.to_string_lossy();
            let ext = relative_path.extension()?;
            if ext != "hs" {
                return None;
            }

            let full_path = hsphelp_dir.join(&relative_path);
            if !read_sjis_file(&full_path, &mut contents) {
                warn!(
                    "ファイルをshift_jisとして解釈できません。path={:?}",
                    full_path
                );
                return None;
            }

            let (_, hs_doc) = doc_interner.intern(&CanonicalUri::from_abs_path(&full_path)?);
            docs.ensure_file_opened(hs_doc, &full_path)?;

            let builtin = is_builtin(&stem);
            debug!("{}.hs builtin={:?}", stem, builtin);
            if builtin {
                info.builtin_docs.push(hs_doc);
            }

            parse_for_symbols(&contents, &mut hs_symbols, &mut warnings);
            contents.clear();
            for w in warnings.drain(..) {
                warn!("hsphelp({:?}): {}", full_path, w);
            }

            for hs_symbol in hs_symbols.drain(..) {
                let (symbol, completion_item) = convert_symbol(hs_symbol);
                symbols.push(completion_item);

                if builtin {
                    builtin_env.insert(symbol.name.clone(), symbol);
                }
            }

            info.doc_symbols.insert(hs_doc, symbols.split_off(0));

            // 同名のcommonのファイルとリンクする。
            for name in [format!("{}.as", stem), format!("{}.hsp", stem)].iter() {
                if let Some(&common_doc) = common_docs.get(name.as_str()) {
                    debug!("link {}.hs => {}", stem, name);
                    info.linked_docs.insert(common_doc, hs_doc);
                }
            }

            None
        })();
    }

    Some(info)
}
