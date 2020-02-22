use super::*;
use std::str::FromStr;

fn create_param_infos(deffunc: &Symbol, symbols: &Symbols) -> Vec<String> {
    let mut params = vec![];
    let mut s = String::new();

    for param in symbols.params(deffunc) {
        if let Some(param_ty_token) = symbols.param_node(&param).param_ty() {
            // 引数を受け取らないパラメータは無視する。
            if !ParamTy::from_str(param_ty_token.text())
                .map_or(false, |param_ty| param_ty.takes_arg())
            {
                continue;
            }

            s += param_ty_token.text();
            s += " ";
        }

        match symbols.unqualified_name(&param) {
            Some(name) => s += name,
            None => s += "???",
        }

        params.push(s.clone());
        s.clear();
    }

    params
}

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

        let active_param_index = arg_holder
            .child_nodes()
            .filter_map(|node| {
                if node.kind() == NodeKind::Arg {
                    Some(node)
                } else {
                    None
                }
            })
            .flat_map(|node| node.child_tokens())
            .take_while(|token| token.range().start() < p)
            .filter(|token| token.kind() == Token::Comma)
            .count();

        let params = match name_context
            .symbol(&name)
            .map(|deffunc| create_param_infos(&deffunc, symbols))
        {
            None => continue,
            Some(x) => x,
        };

        *out = Some(SignatureHelp {
            command: name.unqualified_name(),
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
