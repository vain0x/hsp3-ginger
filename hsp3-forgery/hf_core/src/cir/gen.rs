use super::*;
use crate::kir::*;

struct Context {
    module_name: String,
    stmts: Vec<CStmt>,
}

type Cx = Context;

impl Context {
    fn new() -> Self {
        Context {
            module_name: "".to_string(),
            stmts: vec![],
        }
    }

    fn push_stmt(&mut self, stmt: CStmt) {
        self.stmts.push(stmt);
    }

    fn finish(self) -> Vec<CStmt> {
        self.stmts
    }
}

fn take_term(slot: &mut KTerm) -> KTerm {
    slot.clone()
}

fn take_node(slot: &mut KNode) -> KNode {
    std::mem::replace(slot, KNode::Entry)
}

fn gen_term(term: KTerm) -> CExpr {
    match term {
        KTerm::Int(int_term) => CExpr::Int(int_term.token.text().to_string()),
        KTerm::Name(name_term) => unimplemented!(),
    }
}

fn gen_prim_node(
    prim: KPrim,
    mut args: Vec<KTerm>,
    mut results: Vec<String>,
    mut nexts: Vec<KNode>,
    cx: &mut Cx,
) {
    match prim {
        KPrim::Assign => {
            let (left, right, next) = match (
                args.as_mut_slice(),
                results.as_slice(),
                nexts.as_mut_slice(),
            ) {
                ([left, right], [], [next]) => (take_term(left), take_term(right), take_node(next)),
                _ => unreachable!(),
            };

            // FIXME: left = right
        }
    }
}

fn gen_node(node: KNode, stmts: &mut Vec<CStmt>, cx: &mut Cx) {
    match node {
        KNode::Entry => {}
        KNode::Abort => {
            // FIXME: abort();
        }
        KNode::Prim {
            prim,
            args,
            results,
            nexts,
        } => gen_prim_node(prim, args, results, nexts, cx),
        KNode::Return(args) => {
            assert!(args.terms.len() <= 1);
            let arg_opt = args.terms.into_iter().next().map(gen_term);
            stmts.push(CStmt::Return { arg_opt })
        }
    }
}

fn gen_fn(fx: KFn, cx: &mut Cx) {
    let KFn { name, body } = fx;

    let mut stmts = vec![];
    gen_node(body, &mut stmts, cx);

    cx.push_stmt(CStmt::Func(CFuncStmt {
        name,
        result_ty: CTy::Int,
        body: stmts,
    }));
}

fn gen_module(module: KModule, cx: &mut Cx) {
    for fx in module.fns {
        gen_fn(fx, cx);
    }
}

pub(crate) fn gen(root: KRoot) -> CModule {
    let mut context = Context::new();

    for module in root.modules {
        gen_module(module, &mut context);
    }

    let stmts = context.finish();
    CModule { stmts }
}
