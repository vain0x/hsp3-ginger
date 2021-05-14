use crate::{
    parse::{PCommandStmt, PRoot, PStmt},
    source::Loc,
};
use std::mem::take;

#[derive(Clone)]
pub(crate) enum SyntaxLint {
    ReturnInLoop,
}

impl SyntaxLint {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SyntaxLint::ReturnInLoop => "repeatループの中ではreturnできません。",
        }
    }
}

#[derive(Default)]
pub(crate) struct SyntaxLinter<'p> {
    loop_stack: Vec<&'p PCommandStmt>,
    lints: Vec<(SyntaxLint, Loc)>,
}

impl<'p> SyntaxLinter<'p> {
    fn report(&mut self, lint: SyntaxLint, loc: Loc) {
        self.lints.push((lint, loc));
    }

    fn on_command_stmt(&mut self, stmt: &'p PCommandStmt) {
        match stmt.command.body_text() {
            "repeat" | "foreach" => {
                self.loop_stack.push(stmt);
            }
            "loop" => {
                self.loop_stack.pop();
            }
            "return" => {
                if !self.loop_stack.is_empty() {
                    self.report(SyntaxLint::ReturnInLoop, stmt.command.body.loc);
                }
            }
            _ => {}
        }
    }

    fn on_stmt(&mut self, stmt: &'p PStmt) {
        match stmt {
            PStmt::Label(_) | PStmt::Assign(_) => {}
            PStmt::Command(stmt) => self.on_command_stmt(stmt),
            PStmt::Invoke(_) => {}

            PStmt::DefFunc(stmt) => {
                for stmt in &stmt.stmts {
                    self.on_stmt(stmt);
                }
            }
            PStmt::Module(stmt) => {
                for stmt in &stmt.stmts {
                    self.on_stmt(stmt);
                }
            }
            _ => {}
        }
    }

    fn run(&mut self, root: &'p PRoot) {
        for stmt in &root.stmts {
            self.on_stmt(stmt);
        }
    }
}

pub(crate) fn syntax_lint(root: &PRoot, lints: &mut Vec<(SyntaxLint, Loc)>) {
    let mut linter = SyntaxLinter::default();
    linter.lints = take(lints);
    linter.run(root);
    *lints = take(&mut linter.lints);
}
