pub(crate) mod parse_context;
pub(crate) mod parse_expr;
pub(crate) mod parse_pp;
pub(crate) mod parse_stmt;
pub(crate) mod parse_term;

pub(crate) use parse_context::ParseContext;
pub(crate) use parse_expr::*;
pub(crate) use parse_pp::*;
pub(crate) use parse_stmt::*;
pub(crate) use parse_term::*;

use crate::source::*;
use crate::syntax::*;
use crate::token::*;
use std::rc::Rc;

/// トークン列をパースする。
pub(crate) fn parse_tokens(tokens: &[TokenData]) -> Rc<SyntaxRoot> {
    let mut leading = vec![];
    let mut fat_tokens: Vec<FatToken> = vec![];

    for token in tokens.iter().cloned() {
        if token.token().is_trailing_trivia() && !fat_tokens.is_empty() && leading.is_empty() {
            fat_tokens.last_mut().unwrap().push_trailing(token);
            continue;
        }

        if token.token().is_leading_trivia() {
            leading.push(token);
            continue;
        }

        let mut fat_token = FatToken::new(token);
        for trivia in leading.drain(..) {
            fat_token.push_leading(trivia);
        }
        fat_tokens.push(fat_token);
    }

    // 最後のトークンは EOF なため。
    assert!(leading.is_empty());

    let mut p = ParseContext::new(fat_tokens);
    parse_root(&mut p);
    p.finish()
}
