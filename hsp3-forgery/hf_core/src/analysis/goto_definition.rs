use super::*;

pub(crate) fn goto_definition(
    syntax_root: &SyntaxRoot,
    position: Position,
    global_symbols: &GlobalSymbols,
) -> Option<Location> {
    syntax_root
        .node()
        .descendant_nodes()
        .filter(|node| node.range().contains_loosely(position))
        .filter_map(|node| match node.kind() {
            NodeKind::Ident => {
                let parent_is_command_use_site = match node
                    .parent_node()
                    .map_or(NodeKind::Other, |node| node.kind())
                {
                    NodeKind::CommandStmt | NodeKind::CallExpr => true,
                    _ => false,
                };
                if !parent_is_command_use_site {
                    return None;
                }

                let is_unqualified = node
                    .child_tokens()
                    .all(|token| token.kind() != Token::IdentAtSign);
                if !is_unqualified {
                    // FIXME: モジュール名が一致していればOK
                    return None;
                }

                let use_site_name = AIdent::cast(&node)?.to_string();

                global_symbols
                    .iter()
                    .filter_map(|symbol| {
                        let deffunc_stmt = match symbol {
                            GlobalSymbol::Deffunc { deffunc_stmt, .. } => deffunc_stmt,
                            _ => return None,
                        };

                        let is_local = deffunc_stmt
                            .child_tokens()
                            .any(|token| token.kind() == Token::Ident && token.text() == "local");
                        if is_local {
                            // FIXME: 同じモジュール内ならOK
                            return None;
                        }

                        let name = deffunc_stmt
                            .child_nodes()
                            .filter_map(|node| AIdent::cast(&node))
                            .next()?;

                        if name.to_string() != use_site_name {
                            return None;
                        }

                        let ident = name
                            .syntax()
                            .child_tokens()
                            .filter(|token| token.kind() == Token::Ident)
                            .next()?;
                        Some(ident.location().clone())
                    })
                    .next()
            }
            _ => None,
        })
        .next()
}
