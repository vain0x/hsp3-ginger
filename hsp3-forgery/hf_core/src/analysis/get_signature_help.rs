use super::*;

fn go_node(
    node: &SyntaxNode,
    p: Position,
    name_context: &NameContext,
    symbols: &Symbols,
    out: &mut Option<SignatureHelp>,
) -> bool {
    for child in node.child_nodes() {
        if !child.range().contains_loosely(p) {
            continue;
        }

        if go_node(&child, p, name_context, symbols, out) {
            return true;
        }

        let arg_holder = match ACommandStmt::cast(&child)
            .map(|command| command.syntax().clone())
            .or_else(|| ACallExpr::cast(&child).map(|s| s.syntax().clone()))
        {
            None => continue,
            Some(x) => x,
        };

        let name = match arg_holder
            .child_nodes()
            .filter_map(|node| AName::cast(&node))
            .next()
        {
            None => continue,
            Some(x) => x,
        };

        if name.syntax().nontrivia_range().contains_loosely(p) {
            continue;
        }

        let params = match name_context.symbol(&name).map(|symbol| {
            symbols
                .params(&symbol)
                .map(|param| symbols.unqualified_name(&param).unwrap_or("?").to_string())
                .collect()
        }) {
            None => continue,
            Some(x) => x,
        };

        let mut active_param_index = 0;

        let args = arg_holder
            .child_nodes()
            .filter_map(|node| AArg::cast(&node))
            .enumerate()
            .filter(|(_, arg)| arg.syntax().range().contains_loosely(p));

        for (arg_index, arg) in args {
            if go_node(arg.syntax(), p, name_context, symbols, out) {
                return true;
            }

            active_param_index = arg_index;
            break;
        }

        *out = Some(SignatureHelp {
            params,
            active_param_index,
        });
        return true;
    }

    false
}

pub(crate) fn get(
    syntax_root: &SyntaxRoot,
    position: Position,
    name_context: &NameContext,
    symbols: &Symbols,
) -> Option<SignatureHelp> {
    let mut signature_help = None;
    go_node(
        &syntax_root.node(),
        position,
        name_context,
        symbols,
        &mut signature_help,
    );
    signature_help
}
