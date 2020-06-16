use super::{loc_to_location, loc_to_range, plain_text_to_marked_string, to_loc};
use crate::{lang_service::docs::Docs, sem::ProjectSem};
use lsp_types::{Hover, HoverContents, MarkedString, Position, Url};

pub(crate) fn hover(
    uri: Url,
    position: Position,
    docs: &Docs,
    sem: &mut ProjectSem,
) -> Option<Hover> {
    let loc = to_loc(&uri, position, docs)?;
    let (symbol, symbol_loc) = sem.locate_symbol(loc.doc, loc.start())?;
    let symbol_id = symbol.symbol_id;

    let mut contents = vec![];
    contents.push(plain_text_to_marked_string(symbol.name.to_string()));

    if let Some(description) = symbol.details.description.as_ref() {
        contents.push(plain_text_to_marked_string(description.to_string()));
    }

    contents.extend(
        symbol
            .details
            .documentation
            .iter()
            .map(|text| plain_text_to_marked_string(text.to_string())),
    );

    {
        let mut locs = vec![];
        sem.get_symbol_defs(symbol_id, &mut locs);
        let def_links = locs
            .iter()
            .filter_map(|&loc| {
                let location = loc_to_location(loc, docs)?;
                let uri = location
                    .uri
                    .to_string()
                    .replace("%3A", ":")
                    .replace("\\", "/");
                let Position { line, character } = location.range.start;
                Some(format!("- [{}:{}:{}]({})", uri, line, character, uri))
            })
            .collect::<Vec<_>>();
        if !def_links.is_empty() {
            contents.push(MarkedString::from_markdown(def_links.join("\r\n")));
        }
    }

    Some(Hover {
        contents: HoverContents::Array(contents),
        range: Some(loc_to_range(symbol_loc)),
    })
}
