use super::*;
use std::cmp::Reverse;

/// 指定位置に接触しているノードを深い順で列挙する。
fn ancestral_nodes_at(
    syntax_root: &SyntaxRoot,
    position: Position,
) -> impl Iterator<Item = SyntaxNode> {
    let mut nodes = syntax_root
        .node()
        .descendant_tokens()
        .filter(|token| !token.kind().is_trivia() && token.range().contains_loosely(position))
        .flat_map(|token| token.ancestral_nodes())
        .collect::<Vec<_>>();

    // order by depth desc, start asc
    nodes.sort_by_key(|node| Reverse((node.depth(), Reverse(node.range().start()))));
    nodes.dedup();

    nodes.into_iter()
}

pub(crate) fn goto_definition(
    syntax_root: &SyntaxRoot,
    position: Position,
    name_context: &NameContext,
    symbols: &Symbols,
) -> Option<Location> {
    ancestral_nodes_at(syntax_root, position)
        .filter_map(|node| {
            let name = AName::cast(&node)?;
            let symbol = name_context.symbol(&name)?;
            let def_site = symbols.def_sites(&symbol).next()?;

            let source = syntax_root.source().clone();
            let range = def_site.range();
            Some(Location::new(source, range))
        })
        .next()
}
