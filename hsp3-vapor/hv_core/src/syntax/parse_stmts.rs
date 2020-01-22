use super::parse_context::ParseContext;
use super::parse_exprs::*;
use super::*;

impl Token {
    pub(crate) fn is_stmt_first(self) -> bool {
        self.is_expr_first()
    }
}

pub(crate) fn parse_stmt(p: &mut ParseContext) -> NodeData {
    parse_element()

    match p.next() {
        _ => {
            if let Some(expr) = parse_expr(p) {
                let node = NodeData::new_before(expr);
                return Some(node.set_node(Node::ExprStmt));
            }

            debug_assert!(!p.next().is_stmt_first());
            None
        }
    }
}

pub(crate) fn parse_root(p: &mut ParseContext) -> NodeData {
    let mut root = NodeData::new();

    while !p.at_eof() {
        if let Some(stmt) = parse_stmt(p) {
            root.push_node(stmt);
        } else {
            p.bump(&mut root);
            root.push_error(ParseError::ExpectedExpr);

            // エラー回復
            while !p.at_eof() && !p.next().is_stmt_first() {
                p.bump(&mut root);
            }
        }
    }

    root.set_node(Node::Root)
}
