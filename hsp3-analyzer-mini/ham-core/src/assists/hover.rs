use super::*;
use lsp_types::{
    Documentation, Hover, HoverContents, MarkedString, MarkupContent, MarkupKind, Position, Url,
};

pub(crate) fn hover(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<Hover> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;
    wa.ensure_computed();
    let project = wa.require_project_for_doc(doc);

    let (contents, loc) = (|| -> Option<_> {
        let (symbol, symbol_loc) = project.locate_symbol(doc, pos)?;
        let (name, kind, details) = get_symbol_details(&symbol)?;

        let mut contents = vec![];
        contents.push(plain_text_to_marked_string(format!("{} ({})", name, kind)));

        if let Some(desc) = details.desc {
            contents.push(plain_text_to_marked_string(desc.to_string()));
        }

        contents.extend(details.docs.into_iter().map(plain_text_to_marked_string));

        Some((contents, symbol_loc))
    })()
    .or_else(|| {
        let (name, loc) = wa.get_ident_at(doc, pos)?;
        let tokens = wa.get_syntax(doc)?.tokens;

        let mut completion_items = vec![];
        if in_preproc(pos, &tokens) {
            collect_preproc_completion_items(wa, &mut completion_items);
        }

        let item = completion_items
            .into_iter()
            .find(|s| s.label.trim_start_matches('#') == name.as_str())?;

        let mut contents = vec![];
        contents.push(plain_text_to_marked_string(name.to_string())); // FIXME: %prmの1行目を使ったほうがいい

        if let Some(d) = item.detail {
            contents.push(plain_text_to_marked_string(d));
        }

        if let Some(d) = item.documentation {
            contents.push(documentation_to_marked_string(d));
        }

        Some((contents, loc))
    })
    .or_else(|| {
        if let Some(loc) = wa.on_include_guard(doc, pos) {
            Some((
                vec![plain_text_to_marked_string(
                    "インクルードガード".to_string(),
                )],
                loc,
            ))
        } else {
            None
        }
    })?;

    Some(Hover {
        contents: HoverContents::Array(contents),
        range: Some(loc_to_range(loc)),
    })
}

fn get_symbol_details(symbol: &SymbolRc) -> Option<(RcStr, &'static str, SymbolDetails)> {
    Some((
        symbol.name(),
        symbol.kind.as_str(),
        symbol.compute_details(),
    ))
}

fn documentation_to_marked_string(d: Documentation) -> MarkedString {
    match d {
        Documentation::String(value)
        | Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::PlainText,
            value,
        }) => plain_text_to_marked_string(value),
        Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }) => markdown_marked_string(value),
    }
}

#[cfg(test)]
mod tests {
    use self::assists::lsp::from_proto;
    use super::*;
    use crate::{
        assists::lsp::to_proto,
        lang_service::{docs::NO_VERSION, LangService},
    };
    use std::fmt::Write as _;

    fn dummy_url(s: &str) -> Url {
        let dummy_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".no_exist");
        Url::from_file_path(&dummy_root.join(s)).unwrap()
    }

    // 指定した文字列の指定位置への `Pos` を生成する
    fn pos_at(s: &str, row: u32, column: u32) -> Pos {
        let mut index = 0;
        for _ in 0..row {
            let len = match s[index..].find('\n') {
                Some(it) => it,
                None => panic!("invalid row"),
            };
            index += len + 1; // 1 for LF
        }
        let row_start = index;
        index += column as usize;
        assert!(s.is_char_boundary(index));
        let column16 = s[row_start..index]
            .chars()
            .map(|c| c.len_utf16())
            .sum::<usize>();
        Pos::new(index as u32, row, column, column16 as u32)
    }

    fn format_content(w: &mut String, content: &MarkedString) {
        match content {
            MarkedString::LanguageString(s) => {
                if s.language == "plaintext" {
                    write!(w, "{:?}", s.value).unwrap();
                } else {
                    write!(w, "#{} {:?}", s.language, s.value).unwrap();
                }
            }
            MarkedString::String(s) => {
                write!(w, "{:?}", s).unwrap();
            }
        }
    }

    fn format_response(w: &mut String, hover_opt: Option<&Hover>) {
        let hover = match hover_opt {
            Some(it) => it,
            None => {
                *w += "None";
                return;
            }
        };

        match hover.range {
            Some(r) => {
                let pos = from_proto::pos16(r.start);
                write!(w, "@{} ", pos).unwrap();
            }
            None => {}
        }

        match &hover.contents {
            HoverContents::Scalar(content) => format_content(w, content),
            HoverContents::Array(contents) => {
                for (i, content) in contents.iter().enumerate() {
                    if i >= 1 {
                        *w += "; ";
                    }
                    write!(w, "[{}] ", i + 1).unwrap();
                    format_content(w, content);
                }
            }
            HoverContents::Markup(_) => panic!("no use"),
        }
    }

    #[test]
    fn symbol_test() {
        let mut ls = LangService::new_standalone();

        let main_uri = dummy_url("main.hsp");
        let src = r#"
#module
#defcfunc f int a
    return a + 1
#global

    mes f(42) + 1
"#;
        ls.open_doc(main_uri.clone(), NO_VERSION, src.to_string());

        let mut w = String::new();

        w += "On `f` def:\n";
        format_response(
            &mut w,
            ls.hover(main_uri.clone(), to_proto::pos(pos_at(src, 2, 10)))
                .as_ref(),
        );

        w += "\n\nOn `a` def (param of f):\n";
        format_response(
            &mut w,
            ls.hover(main_uri.clone(), to_proto::pos(pos_at(src, 2, 16)))
                .as_ref(),
        );

        w += "\n\nOn `f` use:\n";
        format_response(
            &mut w,
            ls.hover(main_uri.clone(), to_proto::pos(pos_at(src, 6, 8)))
                .as_ref(),
        );

        assert_eq!(w, "On `f` def:\n@3:11 [1] \"f (関数)\"\n\nOn `a` def (param of f):\n@3:17 [1] \"a (int)\"\n\nOn `f` use:\n@7:9 [1] \"f (関数)\"");
    }

    #[test]
    fn preproc_test() {
        let mut ls = LangService::new_standalone();

        let main_uri = dummy_url("main.hsp");
        let src = r#"
#define ctype hiword(%1) (((%1) >> 16) & 0xFFFF)
"#;
        ls.open_doc(main_uri.clone(), NO_VERSION, src.to_string());

        let mut w = String::new();

        w += "On `<|>ctype`:\n";
        format_response(
            &mut w,
            ls.hover(main_uri.clone(), to_proto::pos(pos_at(src, 1, 8)))
                .as_ref(),
        );

        assert_eq!(
            w,
            "On `<|>ctype`:\n@2:9 [1] \"ctype\"; [2] \"関数形式のマクロを表す\""
        );
    }
}
