use crate::{canonical_uri::CanonicalUri, docs::Docs, sem::ProjectSem, syntax};
use lsp_types::{
    Hover, HoverContents, LanguageString, Location, MarkedString, Position, Range, Url,
};

fn plain_text_to_marked_string(text: String) -> MarkedString {
    const PLAIN_LANG_ID: &str = "plaintext";

    MarkedString::LanguageString(LanguageString {
        language: PLAIN_LANG_ID.to_string(),
        value: text,
    })
}

fn loc_to_range(loc: syntax::Loc) -> Range {
    // FIXME: UTF-8 から UTF-16 基準のインデックスへの変換
    Range::new(
        Position::new(loc.start.row as u64, loc.start.col as u64),
        Position::new(loc.end.row as u64, loc.end.col as u64),
    )
}

fn loc_to_location(loc: syntax::Loc, docs: &Docs) -> Option<Location> {
    let uri = docs.get_uri(loc.doc)?.clone().into_url()?;
    let range = loc_to_range(loc);
    Some(Location { uri, range })
}

fn to_loc(uri: &Url, position: Position, docs: &Docs) -> Option<syntax::Loc> {
    let uri = CanonicalUri::from_url(uri)?;
    let doc = docs.find_by_uri(&uri)?;

    // FIXME: position は UTF-16 ベース、pos は UTF-8 ベースなので、マルチバイト文字が含まれている場合は変換が必要
    let pos = syntax::Pos {
        row: position.line as usize,
        col: position.character as usize,
    };

    Some(syntax::Loc {
        doc,
        start: pos,
        end: pos,
    })
}

pub(crate) fn hover(
    uri: Url,
    position: Position,
    sem: &mut ProjectSem,
    docs: &Docs,
) -> Option<Hover> {
    let loc = to_loc(&uri, position, docs)?;
    let (symbol, symbol_loc) = sem.locate_symbol(loc.doc, loc.start)?;
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
