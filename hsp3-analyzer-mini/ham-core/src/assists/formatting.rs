use crate::{
    analysis::integrate::AWorkspaceAnalysis,
    assists::to_lsp_range,
    lang_service::docs::Docs,
    source::{Pos, Range},
    token::TokenKind,
    utils::canonical_uri::CanonicalUri,
};
use lsp_types::{TextEdit, Url};

pub(crate) fn formatting(
    uri: Url,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
) -> Option<Vec<TextEdit>> {
    let doc = docs.find_by_uri(&CanonicalUri::from_url(&uri))?;
    let (tokens, _root) = wa.get_tokens(doc)?;

    let mut edits = vec![];

    // '#' の字下げを消す。
    for t in tokens.as_slice() {
        if t.kind() == TokenKind::Hash {
            for t in t.leading.iter().rev() {
                match t.kind {
                    TokenKind::Blank => edits.push(TextEdit {
                        range: to_lsp_range(t.loc.range),
                        new_text: "".into(),
                    }),
                    TokenKind::Newlines => {
                        let last = t.text.rfind('\n').unwrap() + 1;
                        let start = t.loc.range.start() + Pos::from(&t.text[..last]);
                        let end = t.loc.range.end();
                        let range = Range::from(start..end);
                        edits.push(TextEdit {
                            range: to_lsp_range(range),
                            new_text: "".into(),
                        });
                        break;
                    }
                    _ => break,
                }
            }
        }
    }

    if edits.is_empty() {
        return None;
    }
    Some(edits)
}
