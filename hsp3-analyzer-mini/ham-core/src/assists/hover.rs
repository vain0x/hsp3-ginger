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
    let project = wa.require_project_for_doc(doc);

    let (contents, loc) = (|| -> Option<_> {
        let (symbol, symbol_loc) = project.locate_symbol(doc, pos)?;
        let (name, kind, details) = project.get_symbol_details(&symbol)?;

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
        let (_, tokens, _) = wa.get_tokens(doc)?;

        let mut completion_items = vec![];
        if in_preproc(pos, &tokens) {
            wa.require_project_for_doc(doc)
                .collect_preproc_completion_items(&mut completion_items);
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
    })?;

    Some(Hover {
        contents: HoverContents::Array(contents),
        range: Some(loc_to_range(loc)),
    })
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
