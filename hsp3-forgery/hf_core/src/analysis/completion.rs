use crate::ast::*;
use crate::syntax::*;
use crate::token::*;
use std::rc::Rc;

pub(crate) fn get_completion_list(syntax_root: Rc<SyntaxRoot>, position: Position) -> Vec<String> {
    fn go_node(node: Rc<SyntaxNode>, idents: &mut Vec<String>) {
        for child in node.child_nodes() {
            if let Some(assign_stmt) = AAssignStmt::cast(child.clone()) {
                if let Some(token) = assign_stmt
                    .into_syntax()
                    .child_tokens()
                    .filter(|t| t.kind() == Token::Ident)
                    .next()
                {
                    idents.push(token.text().to_string());
                }
            }

            go_node(child, idents);
        }
    }

    let mut symbols = vec![];
    go_node(syntax_root.into_node(), &mut symbols);
    symbols.sort();
    symbols.dedup();

    symbols
}

pub(crate) struct SignatureHelp {
    pub(crate) params: Vec<String>,
    pub(crate) active_param_index: usize,
}

pub(crate) fn signature_help(
    syntax_root: Rc<SyntaxRoot>,
    position: Position,
) -> Option<SignatureHelp> {
    None
}
