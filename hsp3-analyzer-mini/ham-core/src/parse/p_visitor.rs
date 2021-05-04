use super::*;
use crate::source::{Pos, Range};

trait PVisitor {
    fn on_token(&mut self, _token: &PToken) {}

    fn on_token_opt(&mut self, token_opt: Option<&PToken>) {
        if let Some(token) = token_opt {
            self.on_token(token);
        }
    }

    fn on_label(&mut self, label: &PLabel) {
        self.on_token(&label.star);
        self.on_token_opt(label.name_opt.as_ref());
    }

    fn on_compound(&mut self, compound: &PCompound) {
        match compound {
            PCompound::Name(name) => self.on_token(name),
            PCompound::Paren(np) => {
                self.on_token(&np.name);
                for arg in &np.args {
                    self.on_arg(arg);
                }
                self.on_token_opt(np.right_paren_opt.as_ref());
            }
            PCompound::Dots(nd) => {
                self.on_token(&nd.name);
                for arg in &nd.args {
                    self.on_token(&arg.dot);
                    self.on_expr_opt(arg.expr_opt.as_ref());
                }
            }
        }
    }

    fn on_arg(&mut self, arg: &PArg) {
        self.on_expr_opt(arg.expr_opt.as_ref());
        self.on_token_opt(arg.comma_opt.as_ref());
    }

    fn on_args(&mut self, args: &[PArg]) {
        for arg in args {
            self.on_arg(arg);
        }
    }

    fn on_expr(&mut self, expr: &PExpr) {
        match expr {
            PExpr::Literal(token) => {
                self.on_token(token);
            }
            PExpr::Label(label) => self.on_label(label),
            PExpr::Compound(compound) => self.on_compound(compound),
            PExpr::Paren(expr) => {
                self.on_token(&expr.left_paren);
                self.on_expr_opt(expr.body_opt.as_deref());
                self.on_token_opt(expr.right_paren_opt.as_ref());
            }
            PExpr::Prefix(expr) => {
                self.on_token(&expr.prefix);
                self.on_expr_opt(expr.arg_opt.as_deref());
            }
            PExpr::Infix(expr) => {
                self.on_expr(expr.left.as_ref());
                self.on_token(&expr.infix);
                self.on_expr_opt(expr.right_opt.as_deref());
            }
        }
    }

    fn on_expr_opt(&mut self, expr_opt: Option<&PExpr>) {
        if let Some(expr) = expr_opt {
            self.on_expr(expr);
        }
    }

    fn on_param(&mut self, param: &PParam) {
        if let Some((_, token)) = &param.param_ty_opt {
            self.on_token(token);
        }
        self.on_token_opt(param.name_opt.as_ref());
        self.on_token_opt(param.comma_opt.as_ref());
    }

    fn on_params(&mut self, params: &[PParam]) {
        for param in params {
            self.on_param(param);
        }
    }

    fn on_deffunc_stmt(&mut self, stmt: &PDefFuncStmt) {
        self.on_token(&stmt.hash);
        self.on_token(&stmt.keyword);
        self.on_token_opt(stmt.privacy_opt.as_ref().map(|(_, t)| t));
        self.on_token_opt(stmt.name_opt.as_ref());
        self.on_params(&stmt.params);
        self.on_token_opt(stmt.onexit_opt.as_ref());
        self.on_stmts(&stmt.stmts);
    }

    fn on_module_stmt(&mut self, stmt: &PModuleStmt) {
        self.on_token(&stmt.hash);
        self.on_token(&stmt.keyword);
        self.on_token_opt(stmt.name_opt.as_ref());
        self.on_params(&stmt.fields);
        self.on_stmts(&stmt.stmts);
        // self.on_global_opt()
        // stmt.global_opt
    }

    fn on_stmt(&mut self, stmt: &PStmt) {
        match stmt {
            PStmt::Label(label) => self.on_label(label),
            PStmt::Assign(stmt) => {
                self.on_compound(&stmt.left);
                self.on_token_opt(stmt.op_opt.as_ref());
                self.on_args(&stmt.args);
            }
            PStmt::Command(stmt) => {
                self.on_token(&stmt.command);
                self.on_token_opt(stmt.jump_modifier_opt.as_ref().map(|(_, t)| t));
                self.on_args(&stmt.args);
            }
            PStmt::Invoke(stmt) => {
                self.on_compound(&stmt.left);
                self.on_token_opt(stmt.arrow_opt.as_ref());
                self.on_expr_opt(stmt.method_opt.as_ref());
                self.on_args(&stmt.args);
            }
            PStmt::Const(_) | PStmt::Define(_) | PStmt::Enum(_) => {
                // FIXME: implement
            }
            PStmt::DefFunc(stmt) => self.on_deffunc_stmt(stmt),
            PStmt::UseLib(_)
            | PStmt::LibFunc(_)
            | PStmt::UseCom(_)
            | PStmt::ComFunc(_)
            | PStmt::RegCmd(_)
            | PStmt::Cmd(_) => {
                // FIXME: implement
            }
            PStmt::Module(stmt) => self.on_module_stmt(stmt),
            PStmt::Global(_) | PStmt::Include(_) | PStmt::UnknownPreProc(_) => {
                // FIXME: implement
            }
        }
    }

    fn on_stmt_opt(&mut self, stmt_opt: Option<&PStmt>) {
        if let Some(stmt) = stmt_opt {
            self.on_stmt(stmt);
        }
    }

    fn on_stmts(&mut self, stmts: &[PStmt]) {
        for stmt in stmts {
            self.on_stmt(stmt);
        }
    }
}

// -----------------------------------------------
// Range
// -----------------------------------------------

impl PArg {
    pub(crate) fn compute_range(&self) -> Range {
        let mut visitor = VisitorForRange::default();
        visitor.on_arg(self);
        visitor.finish()
    }
}

impl PExpr {
    pub(crate) fn compute_range(&self) -> Range {
        let mut visitor = VisitorForRange::default();
        visitor.on_expr(self);
        visitor.finish()
    }
}

impl PStmt {
    pub(crate) fn compute_range(&self) -> Range {
        let mut visitor = VisitorForRange::default();
        visitor.on_stmt(self);
        visitor.finish()
    }
}

#[derive(Default)]
struct VisitorForRange {
    first: Option<Range>,
    last: Option<Pos>,
}

impl VisitorForRange {
    fn finish(self) -> Range {
        match (self.first, self.last) {
            (Some(first), Some(last)) => first.join(Range::empty(last)),
            (Some(range), _) => range,
            _ => Range::empty(Pos::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX)),
        }
    }
}

impl PVisitor for VisitorForRange {
    fn on_token(&mut self, token: &PToken) {
        if self.first.is_none() {
            self.first = Some(token.body.loc.range);
        }

        let end = token.body.loc.end();
        if self.last.map_or(true, |last| last < end) {
            self.last = Some(end);
        }
    }

    // 中間のトークンの探索を可能な限りスキップする:

    fn on_args(&mut self, args: &[PArg]) {
        if let Some((head, tail)) = args.split_first() {
            self.on_arg(head);

            for arg in tail.iter().rev() {
                if self.last.is_some() {
                    return;
                }

                self.on_arg(arg);
            }
        }
    }

    fn on_stmts(&mut self, stmts: &[PStmt]) {
        if let Some((head, tail)) = stmts.split_first() {
            self.on_stmt(head);

            for stmt in tail.iter().rev() {
                if self.last.is_some() {
                    return;
                }

                self.on_stmt(stmt);
            }
        }
    }
}