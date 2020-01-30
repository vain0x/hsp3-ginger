use crate::ast::*;
use crate::syntax::*;

pub(crate) fn get_completion_list(ast_root: &ANodeData, position: Position) -> Vec<String> {
    fn on_stmt(a: &AStmtNode, idents: &mut Vec<String>) {
        match a {
            AStmtNode::Assign(stmt) => {
                idents.push(stmt.left.text().to_string());
            }
            AStmtNode::Command(stmt) => {
                // FIXME: 実装
            }
            AStmtNode::Return(..) => {}
        }
    }

    fn go_node(a: &ANodeData, idents: &mut Vec<String>) {
        match &a.node {
            ANode::Stmt(node) => on_stmt(node, idents),
            ANode::Fn { .. } | ANode::Module { .. } | ANode::Root { .. } => {
                go_nodes(&a.children, idents)
            }
        }
    }

    fn go_nodes(nodes: &[ANodeData], idents: &mut Vec<String>) {
        for node in nodes {
            go_node(node, idents);
        }
    }

    let mut symbols = vec![];
    go_node(ast_root, &mut symbols);
    symbols.sort();
    symbols.dedup();

    symbols
}

pub(crate) struct SignatureHelp {
    pub(crate) params: Vec<String>,
    pub(crate) active_param_index: usize,
}

pub(crate) fn signature_help(ast_root: &ANodeData, position: Position) -> Option<SignatureHelp> {
    fn on_expr(
        a: &AExpr,
        p: Position,
        out: &mut Option<SignatureHelp>,
        accept: &impl Fn(&mut Option<SignatureHelp>),
    ) -> bool {
        match a {
            AExpr::Int(expr) => {
                // FIXME: トークンに接触していなくても引数領域の範囲内ならシグネチャヘルプは反応するべき
                if expr.token.location.range.contains_loosely(p) {
                    accept(out);
                    return true;
                }

                false
            }
            AExpr::Str(..) => {
                // FIXME: 実装
                false
            }
            AExpr::Name(expr) => {
                // FIXME: トークンに接触していなくても引数領域の範囲内ならシグネチャヘルプは反応するべき
                if expr.token.location.range.contains_loosely(p) {
                    accept(out);
                    return true;
                }

                false
            }
            AExpr::Group(AGroupExpr { body_opt, .. }) => {
                if let Some(body) = body_opt {
                    if on_expr(&body, p, out, accept) {
                        return true;
                    }
                }

                false
            }
            AExpr::Call(ACallExpr { cal, .. }) => {
                // FIXME: 引数の中を解析する
                on_expr(&AExpr::Name(cal.clone()), p, out, accept)
            }
        }
    }

    fn on_arg(
        a: &AArg,
        p: Position,
        out: &mut Option<SignatureHelp>,
        accept: &impl Fn(&mut Option<SignatureHelp>),
    ) -> bool {
        if let Some(expr) = &a.expr_opt {
            if on_expr(expr, p, out, accept) {
                return true;
            }
        }

        if let Some(comma) = &a.comma_opt {
            if p == comma.location.start() {
                accept(out);
                return true;
            }
        }

        false
    }

    fn on_stmt(a: &AStmtNode, p: Position, out: &mut Option<SignatureHelp>) -> bool {
        // FIXME: assign/return も関数の引数のシグネチャヘルプを表示できる可能性があるので内部に入るべき
        match a {
            AStmtNode::Assign(stmt) => false,
            AStmtNode::Command(stmt) => {
                for (i, arg) in stmt.args.iter().enumerate() {
                    if on_arg(arg, p, out, &|out| {
                        *out = Some(SignatureHelp {
                            params: vec!["x".to_string(), "y".to_string()],
                            active_param_index: i,
                        });
                    }) {
                        return true;
                    }
                }
                false
            }
            AStmtNode::Return(..) => false,
        }
    }

    fn go_node(a: &ANodeData, p: Position, out: &mut Option<SignatureHelp>) -> bool {
        match &a.node {
            ANode::Stmt(node) => on_stmt(node, p, out),
            ANode::Fn { .. } | ANode::Module { .. } | ANode::Root { .. } => {
                if go_nodes(&a.children, p, out) {
                    return true;
                }
                false
            }
        }
    }

    fn go_nodes(nodes: &[ANodeData], p: Position, out: &mut Option<SignatureHelp>) -> bool {
        for node in nodes {
            if go_node(node, p, out) {
                return true;
            }
        }

        false
    }

    let mut signature_help = None;
    go_node(&ast_root, position, &mut signature_help);
    signature_help
}
