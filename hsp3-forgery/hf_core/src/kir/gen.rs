use super::*;
use crate::ast::*;

struct KirGenContext {
    modules: Vec<KModule>,
}

impl KirGenContext {
    fn new() -> Self {
        Self { modules: vec![] }
    }

    fn finish(self) -> KRoot {
        KRoot {
            modules: self.modules,
        }
    }
}

type Kx = KirGenContext;

fn gen_expr(expr: AExpr, hole: KHole) -> KNode {
    match expr {
        AExpr::Int(int_expr) => hole.apply(KTerm::Int(KInt {
            token: int_expr.token,
        })),
    }
}

fn gen_stmts(stmts: &mut Vec<AStmtNode>) -> KNode {
    let stmt = match stmts.pop() {
        None => return KNode::Entry,
        Some(stmt) => stmt,
    };

    match stmt {
        AStmtNode::Assign(assign_stmt) => {
            let left = KName {
                token: assign_stmt.left,
            };
            let next = gen_stmts(stmts);

            match assign_stmt.right_opt {
                None => KNode::Abort,
                Some(right) => gen_expr(right, KHole::Assign { left, next }),
            }
        }
        AStmtNode::Return(return_stmt) => match return_stmt.result_opt {
            None => KNode::Return(KArgs { terms: vec![] }),
            Some(expr) => gen_expr(expr, KHole::ReturnWithArg),
        },
    }
}

fn gen_fn_node(fn_node: AFnNode, children: Vec<ANodeData>, fns: &mut Vec<KFn>, kx: &mut Kx) {
    let name = fn_node
        .deffunc_stmt
        .name_opt
        .as_ref()
        .map(|token| token.text())
        .unwrap_or("_")
        .to_string();

    let mut stmts = vec![];
    gen_nodes(children, &mut stmts, fns, kx);

    let body = {
        stmts.reverse();
        gen_stmts(&mut stmts)
    };
    fns.push(KFn { name, body });
}

fn gen_module_node(module_node: AModuleNode, children: Vec<ANodeData>, kx: &mut Kx) {
    // FIXME: m1, m2, etc.
    let name = module_node
        .module_stmt
        .name_opt
        .as_ref()
        .map(|token| token.text())
        .unwrap_or("_")
        .to_string();

    let mut stmts = vec![];
    let mut fns = vec![];
    gen_nodes(children, &mut stmts, &mut fns, kx);

    // #module から最初の #deffunc までは実行されない。
    std::mem::drop(stmts);

    kx.modules.push(KModule { name, fns });
}

fn gen_nodes(nodes: Vec<ANodeData>, stmts: &mut Vec<AStmtNode>, fns: &mut Vec<KFn>, kx: &mut Kx) {
    for ANodeData { node, children } in nodes {
        match node {
            ANode::Stmt(stmt_node) => {
                assert!(children.is_empty());
                stmts.push(stmt_node);
            }
            ANode::Fn(fn_node) => gen_fn_node(fn_node, children, fns, kx),
            ANode::Module(module_node) => gen_module_node(module_node, children, kx),
            ANode::Root(..) => gen_nodes(children, stmts, fns, kx),
        }
    }
}
pub(crate) fn gen(root: ANodeData) -> KRoot {
    assert!(root.node.is_root());

    let mut context = Kx::new();

    let mut stmts = vec![];
    let mut fns = vec![];
    gen_nodes(vec![root], &mut stmts, &mut fns, &mut context);

    let body = {
        stmts.reverse();
        gen_stmts(&mut stmts)
    };
    fns.push(KFn {
        name: "forgery_main".to_string(),
        body,
    });

    context.modules.push(KModule {
        name: "global".to_string(),
        fns,
    });

    context.finish()
}
