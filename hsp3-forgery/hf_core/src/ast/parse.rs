use super::*;
use std::collections::HashMap;

pub(crate) fn parse(tokens: &[TokenData]) -> ANodeData {
    parse_node::parse_node(parse_stmt::parse_tokens(tokens))
}
