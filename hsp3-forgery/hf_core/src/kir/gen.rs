use super::*;
use crate::ast::*;

#[derive(Debug)]
enum KNodeHead {
    Assign { left: KName },
    Command { command: KName },
    Return,
}

#[derive(Debug)]
enum KCode {
    Term(KTerm),
    Node { head: KNodeHead, arity: usize },
}

#[derive(Debug)]
enum KElement {
    Term(KTerm),
    Node(KNode),
}

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

fn take_term(slot: &mut KTerm) -> KTerm {
    std::mem::replace(slot, KTerm::Omit)
}

fn take_node(slot: &mut KNode) -> KNode {
    std::mem::replace(slot, KNode::Entry)
}

fn gen_expr(expr: AExpr, codes: &mut Vec<KCode>) {
    match expr {
        AExpr::Int(int_expr) => codes.push(KCode::Term(KTerm::Int(KInt {
            token: int_expr.token,
        }))),
        AExpr::Name(name_expr) => codes.push(KCode::Term(KTerm::Name(KName {
            token: name_expr.token,
        }))),
    }
}

fn gen_expr_opt(expr_opt: Option<AExpr>, codes: &mut Vec<KCode>) {
    match expr_opt {
        Some(expr) => gen_expr(expr, codes),
        None => codes.push(KCode::Term(KTerm::Omit)),
    }
}

fn gen_code(codes: &mut Vec<KCode>) -> KElement {
    // stop?
    let default_node = KNode::Abort;

    let (head, arity) = match codes.pop() {
        None => return KElement::Node(default_node),
        Some(KCode::Term(term)) => {
            return KElement::Term(term);
        }
        Some(KCode::Node { head, arity }) => (head, arity),
    };

    let mut children = vec![];
    for _ in 0..arity {
        children.push(gen_code(codes));
    }

    match head {
        KNodeHead::Assign { left } => match children.as_mut_slice() {
            [KElement::Term(right), KElement::Node(next)] => KElement::Node(KNode::Prim {
                prim: KPrim::Assign,
                args: vec![KTerm::Name(left), take_term(right)],
                results: vec![],
                nexts: vec![take_node(next)],
            }),
            _ => unimplemented!("{:?}", children),
        },
        KNodeHead::Command { command } => {
            let next = match children.pop() {
                Some(KElement::Node(node)) => node,
                Some(_) | None => default_node,
            };

            let mut args = vec![KTerm::Name(command)];
            for element in children {
                let term = match element {
                    KElement::Term(term) => term,
                    KElement::Node(node) => unreachable!("Expected term {:?}", node),
                };
                args.push(term);
            }

            KElement::Node(KNode::Prim {
                prim: KPrim::Command,
                args,
                results: vec![],
                nexts: vec![next],
            })
        }
        KNodeHead::Return => {
            let mut args = vec![];
            for element in children {
                let term = match element {
                    KElement::Term(term) => term,
                    KElement::Node(node) => unreachable!("Expected term {:?}", node),
                };
                args.push(term);
            }

            KElement::Node(KNode::Return(KArgs { terms: args }))
        }
    }
}

fn gen_stmts(stmts: &mut Vec<AStmtNode>) -> KNode {
    if stmts.is_empty() {
        return KNode::Entry;
    }

    // FIXME: モジュール内なら abort, トップレベルなら stop
    let mut codes = vec![];

    stmts.reverse();

    while let Some(stmt) = stmts.pop() {
        match stmt {
            AStmtNode::Assign(assign_stmt) => {
                let left = KName {
                    token: assign_stmt.left,
                };

                gen_expr_opt(assign_stmt.right_opt, &mut codes);

                codes.push(KCode::Node {
                    head: KNodeHead::Assign { left },
                    arity: 2,
                });
            }
            AStmtNode::Command(command_stmt) => {
                let command = KName {
                    token: command_stmt.command,
                };

                let arity = command_stmt.args.len();

                for arg in command_stmt.args {
                    gen_expr_opt(arg.expr_opt, &mut codes);
                }

                codes.push(KCode::Node {
                    head: KNodeHead::Command { command },
                    arity: arity + 1,
                });
            }
            AStmtNode::Return(return_stmt) => {
                let arity = if let Some(expr) = return_stmt.result_opt {
                    gen_expr(expr, &mut codes);
                    1
                } else {
                    0
                };

                codes.push(KCode::Node {
                    head: KNodeHead::Return,
                    arity,
                });
            }
        }
    }

    match gen_code(&mut codes) {
        KElement::Term(term) => unimplemented!("bad code {:?}", term),
        KElement::Node(node) => node,
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
    let name = module_node
        .module_stmt
        .name_opt
        .as_ref()
        .map(|token| token.text().to_string())
        .unwrap_or_else(|| format!("{}", kx.modules.len() + 1));

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
