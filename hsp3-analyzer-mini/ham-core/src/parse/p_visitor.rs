use super::*;

pub(crate) trait PVisitor {
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

    fn on_compound_default(&mut self, compound: &PCompound) {
        match compound {
            PCompound::Name(name) => self.on_token(name),
            PCompound::Paren(np) => {
                self.on_token(&np.name);
                self.on_token(&np.left_paren);
                self.on_args(&np.args);
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

    fn on_compound(&mut self, compound: &PCompound) {
        self.on_compound_default(compound);
    }

    fn on_arg(&mut self, arg: &PArg) {
        self.on_expr_opt(arg.expr_opt.as_ref());
        self.on_token_opt(arg.comma_opt.as_ref());
    }

    fn on_args_default(&mut self, args: &[PArg]) {
        for arg in args {
            self.on_arg(arg);
        }
    }

    fn on_args(&mut self, args: &[PArg]) {
        self.on_args_default(args);
    }

    fn on_expr_default(&mut self, expr: &PExpr) {
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

    fn on_expr(&mut self, expr: &PExpr) {
        self.on_expr_default(expr);
    }

    fn on_expr_opt(&mut self, expr_opt: Option<&PExpr>) {
        if let Some(expr) = expr_opt {
            self.on_expr(expr);
        }
    }

    fn on_param_default(&mut self, param: &PParam) {
        if let Some((_, token)) = &param.param_ty_opt {
            self.on_token(token);
        }
        self.on_token_opt(param.name_opt.as_ref());
        self.on_token_opt(param.comma_opt.as_ref());
    }

    fn on_param(&mut self, param: &PParam) {
        self.on_param_default(param);
    }

    fn on_params_default(&mut self, params: &[PParam]) {
        for param in params {
            self.on_param(param);
        }
    }

    fn on_params(&mut self, params: &[PParam]) {
        self.on_params_default(params);
    }

    fn on_block(&mut self, block: &PBlock) {
        self.on_stmts(&block.outer_stmts);

        if let Some(left) = &block.left_opt {
            self.on_token(left);
            self.on_stmts(&block.inner_stmts);
            self.on_token_opt(block.right_opt.as_ref());
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

    fn on_stmt_default(&mut self, stmt: &PStmt) {
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
            PStmt::If(stmt) => {
                self.on_token(&stmt.command);
                self.on_expr_opt(stmt.cond_opt.as_ref());
                self.on_block(&stmt.body);

                if let Some(e) = &stmt.else_opt {
                    self.on_token(e);
                    self.on_block(&stmt.alt);
                }
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

    fn on_stmt(&mut self, stmt: &PStmt) {
        self.on_stmt_default(stmt);
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

    fn on_root(&mut self, root: &PRoot) {
        self.on_stmts(&root.stmts);
        self.on_token(&root.eof);
    }
}

// -----------------------------------------------
// Range
// -----------------------------------------------

impl PCompound {
    pub(crate) fn compute_range(&self) -> Range {
        let mut visitor = VisitorForRange::default();
        visitor.on_compound(self);
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

        // 文は明示的に終端するトークンがないので、後続する空白も範囲に含める。(これはシグネチャヘルプの都合。)
        visitor.trailing = true;

        visitor.on_stmt(self);
        visitor.finish()
    }
}

#[derive(Default)]
struct VisitorForRange {
    first: Option<Range>,
    last: Option<Pos>,
    trailing: bool,
}

impl VisitorForRange {
    fn update_last(&mut self, end: Pos) {
        if self.last.map_or(true, |last| last < end) {
            self.last = Some(end);
        }
    }

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
            self.first = Some(if self.trailing {
                token.body.loc.range.join(token.behind().range)
            } else {
                token.body.loc.range
            });
        }

        let end = if self.trailing {
            token.behind().range.end()
        } else {
            token.body.loc.end()
        };
        self.update_last(end);
    }

    // 中間のトークンの探索を可能な限りスキップする:

    fn on_args(&mut self, args: &[PArg]) {
        if let Some((head, tail)) = args.split_first() {
            self.on_arg(head);

            let last = take(&mut self.last);
            for arg in tail.iter().rev() {
                self.on_arg(arg);
                if self.last.is_some() {
                    break;
                }
            }
            if let Some(last) = last {
                self.update_last(last);
            }
        }
    }

    fn on_stmts(&mut self, stmts: &[PStmt]) {
        if let Some((head, tail)) = stmts.split_first() {
            self.on_stmt(head);

            let last = take(&mut self.last);
            for stmt in tail.iter().rev() {
                self.on_stmt(stmt);
                if self.last.is_some() {
                    break;
                }
            }
            if let Some(last) = last {
                self.update_last(last);
            }
        }
    }
}
