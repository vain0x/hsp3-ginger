use super::*;
use crate::syntax::*;

pub(crate) fn parse(tokens: Vec<TokenData>) -> ANodeData {
    parse_node::parse_node(parse_stmt::parse_tokens(tokens))
}
