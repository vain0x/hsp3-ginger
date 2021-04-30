use super::{from_document_position, loc_to_range, plain_text_to_marked_string};
use crate::{
    analysis::integrate::AWorkspaceAnalysis, assists::markdown_marked_string,
    lang_service::docs::Docs,
};
use lsp_types::{
    CompletionItem, Documentation, Hover, HoverContents, MarkedString, MarkupContent, MarkupKind,
    Position, Url,
};

pub(crate) fn hover(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
    hsphelp_symbols: &[CompletionItem],
) -> Option<Hover> {
    let (doc, pos) = from_document_position(&uri, position, docs)?;

    let (contents, loc) = (|| -> Option<_> {
        let (symbol, symbol_loc) = wa.locate_symbol(doc, pos)?;
        let (name, kind, details) = wa.get_symbol_details(symbol)?;

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
        let item = hsphelp_symbols
            .iter()
            .find(|s| s.label == name.as_str())?
            .clone();

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
