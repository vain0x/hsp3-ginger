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

pub(crate) fn parse_tokens(tokens: &[TokenData]) -> SyntaxRoot {
    let tokens = tokens
        .into_iter()
        .filter(|t| t.token() != Token::Space && t.token() != Token::Comment)
        .cloned()
        .collect::<Vec<_>>();

    let mut p = ParseContext::new(tokens);
    parse_root(&mut p);
    p.finish()
}
