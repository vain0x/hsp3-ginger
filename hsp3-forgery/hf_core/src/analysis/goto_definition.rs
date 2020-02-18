use super::*;

pub(crate) fn goto_definition(
    syntax_root: &SyntaxRoot,
    position: Position,
    name_context: &NameContext,
    symbols: &Symbols,
) -> Option<Location> {
    syntax_root
        .node()
        .descendant_nodes()
        .filter(|node| node.range().contains_loosely(position))
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
