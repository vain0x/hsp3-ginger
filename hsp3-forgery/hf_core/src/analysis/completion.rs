use crate::ast::*;
use crate::syntax::*;
use crate::token::*;
use std::rc::Rc;

pub(crate) fn get_completion_list(syntax_root: Rc<SyntaxRoot>, position: Position) -> Vec<String> {
    fn go_node(node: &SyntaxNode, idents: &mut Vec<String>) {
        for child in node.child_nodes() {
            if let Some(assign_stmt) = AAssignStmt::cast(&child) {
                if let Some(token) = assign_stmt
                    .syntax()
                    .child_tokens()
                    .filter(|t| t.kind() == Token::Ident)
                    .next()
                {
                    idents.push(token.text().to_string());
                }
            }

            go_node(&child, idents);
        }
    }

    let mut symbols = vec![];
    go_node(&syntax_root.into_node(), &mut symbols);
    symbols.sort();
    symbols.dedup();

    symbols
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

            let command_stmt = match ACommandStmt::cast(&child) {
                None => continue,
                Some(x) => x,
            };

            let mut arg_index = 0;

            for element in command_stmt.syntax().child_elements() {
                match element {
                    SyntaxElement::Token(token) => {
                        if token.kind() == Token::Ident {
                            // コマンド
                        } else {
                            continue;
                        }
                    }
                    SyntaxElement::Node(node) => match AArg::cast(&node) {
                        None => continue,
                        Some(arg) => {
                            arg_index += 1;

                            let syntax = arg.syntax();
                            if !syntax.range().contains_loosely(p) {
                                continue;
                            }

                            // 引数
                            if go_node(syntax, p, out) {
                                return true;
                            }

                            *out = Some(SignatureHelp {
                                params: vec!["x", "y"]
                                    .into_iter()
                                    .map(ToString::to_string)
                                    .collect(),
                                active_param_index: arg_index - 1,
                            });
                            return true;
                        }
                    },
                }
            }
        }

        false
    }

    let mut help = None;
    go_node(&syntax_root.into_node(), position, &mut help);
    help
}
