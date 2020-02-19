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
