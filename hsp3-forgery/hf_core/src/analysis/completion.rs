use crate::ast::*;
use crate::syntax::*;
use crate::token::*;
use std::rc::Rc;

pub(crate) fn get_completion_list(syntax_root: Rc<SyntaxRoot>, position: Position) -> Vec<String> {
    syntax_root
        .node()
        .descendant_elements()
        .filter_map(|e| AAssignStmt::cast(&SyntaxElement::cast_node(e)?))
        .flat_map(|assign_stmt| {
            assign_stmt
                .syntax()
                .child_nodes()
                .filter_map(|node| AExpr::cast(&node))
                .flat_map(|expr| expr.syntax().child_tokens())
                .filter(|token| token.kind() == Token::Ident)
        })
        .map(|token| token.text().to_string())
        .collect()
}

pub(crate) struct SignatureHelp {
    pub(crate) params: Vec<String>,
    pub(crate) active_param_index: usize,
}

pub(crate) fn get_signature_help(
    syntax_root: Rc<SyntaxRoot>,
    position: Position,
) -> Option<SignatureHelp> {
    fn go_node(node: &SyntaxNode, p: Position, out: &mut Option<SignatureHelp>) -> bool {
        for child in node.child_nodes() {
            if !child.range().contains_loosely(p) {
                continue;
            }

            if go_node(&child, p, out) {
                return true;
            }

            if let Some(stmt) = ACommandStmt::cast(&child)
                .map(|s| s.syntax().clone())
                .or_else(|| ACallExpr::cast(&child).map(|s| s.syntax().clone()))
            {
                for (arg_index, arg) in stmt
                    .child_nodes()
                    .filter_map(|node| AArg::cast(&node))
                    .enumerate()
                    .filter(|(_, arg)| arg.syntax().range().contains_loosely(p))
                {
                    if go_node(arg.syntax(), p, out) {
                        return true;
                    }

                    let params = loop {
                        if let Some(command_token) = stmt
                            .child_nodes()
                            .filter(|node| node.kind() == NodeKind::Ident)
                            .flat_map(|node| node.child_tokens())
                            .filter(|t| t.kind() == Token::Ident || t.kind().is_control_keyword())
                            .next()
                        {
                            if command_token.text() == "width" {
                                break vec!["x".to_string(), "y".to_string()];
                            }
                        }

                        if let Some(func_token) = stmt
                            .child_nodes()
                            .filter_map(|node| AIdent::cast(&node))
                            .flat_map(|ident| {
                                ident
                                    .syntax()
                                    .child_tokens()
                                    .filter(|t| t.kind() == Token::Ident)
                            })
                            .next()
                        {
                            if func_token.text() == "instr" {
                                break vec![
                                    "text".to_string(),
                                    "offset".to_string(),
                                    "search_word".to_string(),
                                ];
                            }
                        }

                        break vec![];
                    };

                    *out = Some(SignatureHelp {
                        params,
                        active_param_index: arg_index,
                    });
                    return true;
                }
            }
        }
        false
    }

    let mut help = None;
    go_node(&syntax_root.node(), position, &mut help);
    help
}
