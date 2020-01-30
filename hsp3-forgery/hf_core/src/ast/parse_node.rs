use super::*;
use crate::syntax::*;

struct Context {
    in_module: bool,
    deffunc_stmt_opt: Option<ADeffuncStmt>,
    global_stmt_opt: Option<AGlobalStmt>,
    stmts: Vec<AStmt>,
    errors: Vec<SyntaxError>,
}

impl Context {
    fn new(mut stmts: Vec<AStmt>, errors: Vec<SyntaxError>) -> Context {
        stmts.reverse();

        Context {
            in_module: false,
            deffunc_stmt_opt: None,
            global_stmt_opt: None,
            stmts,
            errors,
        }
    }

    fn in_module(&self) -> bool {
        self.in_module
    }

    fn enter_module(&mut self) {
        assert!(!self.in_module());
        self.in_module = true;
    }

    fn leave_module(&mut self) {
        assert!(self.in_module());
        self.in_module = false;
    }

    fn set_deffunc_stmt(&mut self, deffunc_stmt: ADeffuncStmt) {
        assert!(self.deffunc_stmt_opt.is_none());
        self.deffunc_stmt_opt = Some(deffunc_stmt);
    }

    fn set_global_stmt(&mut self, global_stmt: AGlobalStmt) {
        assert!(self.global_stmt_opt.is_none());
        self.global_stmt_opt = Some(global_stmt);
    }

    fn pop_stmt(&mut self) -> Option<AStmt> {
        if let Some(deffunc_stmt) = self.deffunc_stmt_opt.take() {
            return Some(AStmt::Deffunc(deffunc_stmt));
        }

        if let Some(global_stmt) = self.global_stmt_opt.take() {
            return Some(AStmt::Global(global_stmt));
        }

        self.stmts.pop()
    }

    fn error(&mut self, msg: &str, location: Location) {
        self.errors
            .push(SyntaxError::new(msg.to_string(), location));
    }

    fn finish(self) -> Vec<SyntaxError> {
        assert!(!self.in_module());
        assert!(self.stmts.is_empty());

        self.errors
    }
}

fn nested_module_error(module_stmt: AModuleStmt, context: &mut Context) {
    context.error(
        "#module の中で #module を使用することはできません。",
        module_stmt.main_location(),
    );
}

fn missing_module_error(global_stmt: AGlobalStmt, context: &mut Context) {
    context.error(
        "#global に対応する #module がありません。",
        global_stmt.main_location(),
    );
}

fn gen_fn(deffunc_stmt: ADeffuncStmt, context: &mut Context) -> ANodeData {
    let mut children = vec![];

    while let Some(stmt) = context.pop_stmt() {
        match stmt {
            AStmt::Label(stmt) => children.push(stmt.into()),
            AStmt::Assign(assign_stmt) => children.push(assign_stmt.into()),
            AStmt::Command(command_stmt) => children.push(command_stmt.into()),
            AStmt::Return(return_stmt) => children.push(return_stmt.into()),
            AStmt::Module(module_stmt) => {
                if context.in_module() {
                    nested_module_error(module_stmt, context);
                    continue;
                }

                children.push(gen_module(module_stmt, context));
            }
            AStmt::Global(global_stmt) => {
                context.set_global_stmt(global_stmt);
                break;
            }
            AStmt::Deffunc(deffunc_stmt) => {
                context.set_deffunc_stmt(deffunc_stmt);
                break;
            }
            AStmt::UnknownPp(..) => {}
        }
    }

    ANodeData {
        node: ANode::Fn(AFnNode { deffunc_stmt }),
        children,
    }
}

fn gen_module(module_stmt: AModuleStmt, context: &mut Context) -> ANodeData {
    context.enter_module();

    let mut children = vec![];
    let mut global_stmt_opt = None;

    while let Some(stmt) = context.pop_stmt() {
        match stmt {
            AStmt::Label(stmt) => children.push(stmt.into()),
            AStmt::Assign(assign_stmt) => children.push(assign_stmt.into()),
            AStmt::Command(command_stmt) => children.push(command_stmt.into()),
            AStmt::Return(return_stmt) => children.push(return_stmt.into()),
            AStmt::Module(module_stmt) => {
                nested_module_error(module_stmt, context);
            }
            AStmt::Global(global_stmt) => {
                global_stmt_opt = Some(global_stmt);
                break;
            }
            AStmt::Deffunc(deffunc_stmt) => children.push(gen_fn(deffunc_stmt, context)),
            AStmt::UnknownPp(..) => {}
        }
    }

    context.leave_module();

    ANodeData {
        node: ANode::Module(AModuleNode {
            module_stmt,
            global_stmt_opt,
        }),
        children,
    }
}

fn gen_root(children: &mut Vec<ANodeData>, context: &mut Context) {
    while let Some(stmt) = context.pop_stmt() {
        match stmt {
            AStmt::Label(stmt) => children.push(stmt.into()),
            AStmt::Assign(assign_stmt) => children.push(assign_stmt.into()),
            AStmt::Command(command_stmt) => children.push(command_stmt.into()),
            AStmt::Return(return_stmt) => children.push(return_stmt.into()),
            AStmt::Module(module_stmt) => {
                children.push(gen_module(module_stmt, context));
            }
            AStmt::Global(global_stmt) => {
                missing_module_error(global_stmt, context);
            }
            AStmt::Deffunc(deffunc_stmt) => {
                children.push(gen_fn(deffunc_stmt, context));
            }
            AStmt::UnknownPp(..) => {}
        }
    }
}

pub(crate) fn parse_node(root: ARoot) -> ANodeData {
    let ARoot {
        children: stmts,
        errors,
    } = root;

    let mut context = Context::new(stmts, errors);
    let mut children = vec![];
    gen_root(&mut children, &mut context);
    let errors = context.finish();

    ANodeData {
        node: ANode::Root(ARootNode { errors }),
        children,
    }
}
