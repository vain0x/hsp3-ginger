use super::*;
use crate::syntax::*;
use std::collections::HashMap;

pub(crate) type SyntaxRootComponent = HashMap<SyntaxSource, ANodeData>;

pub(crate) fn parse(tokens: &[TokenData]) -> ANodeData {
    parse_node::parse_node(parse_stmt::parse_tokens(tokens))
}

pub(crate) fn parse_sources(
    sources: &[(SyntaxSource, &[TokenData])],
    syntax_roots: &mut SyntaxRootComponent,
) {
    for (source, tokens) in sources {
        let root = parse(tokens);
        syntax_roots.insert(source.clone(), root);
    }
}
