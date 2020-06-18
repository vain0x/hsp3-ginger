use super::{
    parse_context::Px,
    parse_expr::{parse_args, parse_atomic_expr, parse_compound, parse_label},
    parse_preproc::parse_preproc_stmt,
    PAssignStmt, PCommandStmt, PInvokeStmt, PJumpModifier, PRoot, PStmt, PToken,
};
use crate::token::TokenKind;

/// 先読みトークン数の上限
const LOOKAHEAD_LIMIT: usize = 30;

enum ExprLikeStmtKind {
    Assign,
    Command,
    Invoke,
}

/// あるトークンを演算子として解釈するとき、どのような構文としてパースできるか？
#[derive(PartialEq, Eq)]
enum OperatorKind {
    /// 演算子ではない
    None,
    /// 中置のみ
    Infix,
    /// 代入のみ
    Assign,
    /// 中置または代入
    InfixOrAssign,
    /// 前置、中置、複合代入
    PrefixOrInfixOrAssign,
}

impl TokenKind {
    fn is_plus_or_minus(self) -> bool {
        match self {
            TokenKind::Minus | TokenKind::Plus => true,
            _ => false,
        }
    }

    fn to_operator_kind(self) -> OperatorKind {
        match self {
            TokenKind::Minus | TokenKind::Star => OperatorKind::PrefixOrInfixOrAssign,
            TokenKind::AndAnd | TokenKind::PipePipe | TokenKind::EqualEqual => OperatorKind::Infix,
            TokenKind::LeftAngle
            | TokenKind::RightAngle
            | TokenKind::And
            | TokenKind::Backslash
            | TokenKind::Bang
            | TokenKind::Equal
            | TokenKind::Hat
            | TokenKind::LeftEqual
            | TokenKind::LeftShift
            | TokenKind::Pipe
            | TokenKind::Plus
            | TokenKind::RightEqual
            | TokenKind::RightShift
            | TokenKind::Slash => OperatorKind::InfixOrAssign,
            TokenKind::AndEqual
            | TokenKind::BackslashEqual
            | TokenKind::BangEqual
            | TokenKind::HatEqual
            | TokenKind::MinusEqual
            | TokenKind::MinusMinus
            | TokenKind::PipeEqual
            | TokenKind::PlusEqual
            | TokenKind::PlusPlus
            | TokenKind::SlashEqual
            | TokenKind::StarEqual => OperatorKind::Assign,
            _ => OperatorKind::None,
        }
    }

    /// 複合代入演算子
    pub(crate) fn is_compound_assignment_operator(self) -> bool {
        match self {
            TokenKind::AndEqual
            | TokenKind::BackslashEqual
            | TokenKind::HatEqual
            | TokenKind::MinusEqual
            | TokenKind::MinusMinus
            | TokenKind::PipeEqual
            | TokenKind::PlusEqual
            | TokenKind::PlusPlus
            | TokenKind::SlashEqual
            | TokenKind::StarEqual => true,
            _ => false,
        }
    }

    pub(crate) fn is_assignment_operator(self) -> bool {
        match self.to_operator_kind() {
            OperatorKind::Assign | OperatorKind::InfixOrAssign => true,
            _ => false,
        }
    }

    pub(crate) fn at_end_of_stmt(self) -> bool {
        match self {
            TokenKind::Eof
            | TokenKind::Eos
            | TokenKind::Colon
            | TokenKind::LeftBrace
            | TokenKind::RightBrace => true,
            _ => false,
        }
    }
}

fn parse_end_of_stmt(px: &mut Px) {
    while !px.next().at_end_of_stmt() {
        px.skip();
    }
}

fn parse_jump_modifier(px: &mut Px) -> Option<(PJumpModifier, PToken)> {
    if px.next() != TokenKind::Ident {
        return None;
    }

    match px.next_token().body_text().parse::<PJumpModifier>() {
        Ok(jump_modifier) => {
            let token = px.bump();
            Some((jump_modifier, token))
        }
        Err(()) => None,
    }
}

fn lookahead_after_paren(mut i: usize, px: &mut Px) -> ExprLikeStmtKind {
    let mut balance = 1;

    loop {
        let kind = px.nth(i);
        i += 1;

        match kind {
            TokenKind::LeftParen => balance += 1,
            TokenKind::RightParen => match balance {
                0 | 1 => break,
                _ => balance -= 1,
            },
            TokenKind::Comma if balance == 1 => {
                // カッコの直下にカンマがあるなら添字のカッコなので、代入文で確定。
                return ExprLikeStmtKind::Assign;
            }
            TokenKind::SlimArrow => {
                return ExprLikeStmtKind::Invoke;
            }
            _ if kind.to_operator_kind() == OperatorKind::Assign => {
                return ExprLikeStmtKind::Assign;
            }
            _ if kind.at_end_of_stmt() => break,
            _ if i >= LOOKAHEAD_LIMIT => {
                // 長い文はたぶん命令文。
                return ExprLikeStmtKind::Command;
            }
            _ => {}
        }
    }

    match px.nth(i) {
        TokenKind::Plus | TokenKind::Minus if px.nth(i + 1).at_end_of_stmt() => {
            // `x+`
            ExprLikeStmtKind::Assign
        }
        kind if kind.at_end_of_stmt() => ExprLikeStmtKind::Command,
        kind => match kind.to_operator_kind() {
            OperatorKind::None | OperatorKind::Infix | OperatorKind::InfixOrAssign => {
                ExprLikeStmtKind::Command
            }
            OperatorKind::Assign | OperatorKind::PrefixOrInfixOrAssign => ExprLikeStmtKind::Assign,
        },
    }
}

fn lookahead_stmt(px: &mut Px) -> ExprLikeStmtKind {
    match px.nth(1) {
        TokenKind::LeftParen => lookahead_after_paren(2, px),
        TokenKind::Dot => ExprLikeStmtKind::Assign,
        TokenKind::SlimArrow => ExprLikeStmtKind::Invoke,
        second => match second.to_operator_kind() {
            OperatorKind::None => ExprLikeStmtKind::Command,
            OperatorKind::Infix | OperatorKind::InfixOrAssign | OperatorKind::Assign => {
                ExprLikeStmtKind::Assign
            }
            OperatorKind::PrefixOrInfixOrAssign if px.nth(2).at_end_of_stmt() => {
                // `x-`
                ExprLikeStmtKind::Assign
            }
            OperatorKind::PrefixOrInfixOrAssign => {
                // `x-a...`
                // マイナスが前置演算子なのか複合代入演算子なのか判断できない。
                // ここでは前置演算子と仮定してパースする。(複合代入には `-=` を推奨)
                ExprLikeStmtKind::Command
            }
        },
    }
}

fn parse_expr_like_stmt(px: &mut Px) -> Option<PStmt> {
    match lookahead_stmt(px) {
        ExprLikeStmtKind::Assign => parse_assign_stmt(px).map(PStmt::Assign),
        ExprLikeStmtKind::Command => parse_command_stmt(px).map(PStmt::Command),
        ExprLikeStmtKind::Invoke => parse_invoke_stmt(px).map(PStmt::Invoke),
    }
}

fn parse_assign_stmt(px: &mut Px) -> Option<PAssignStmt> {
    let left = parse_compound(px)?;

    let op_opt = if px.next().is_assignment_operator() {
        Some(px.bump())
    } else {
        None
    };

    let args = parse_args(px);
    Some(PAssignStmt { left, op_opt, args })
}

fn parse_command_stmt(px: &mut Px) -> Option<PCommandStmt> {
    let command = px.bump();
    let jump_modifier_opt = parse_jump_modifier(px);
    let args = parse_args(px);
    Some(PCommandStmt {
        command,
        jump_modifier_opt,
        args,
    })
}

fn parse_invoke_stmt(px: &mut Px) -> Option<PInvokeStmt> {
    let left = parse_compound(px)?;
    let arrow_opt = px.eat(TokenKind::SlimArrow);
    let method_opt = parse_atomic_expr(px);
    let args = parse_args(px);
    Some(PInvokeStmt {
        left,
        arrow_opt,
        method_opt,
        args,
    })
}

pub(crate) fn parse_stmt(px: &mut Px) -> Option<PStmt> {
    let stmt_opt = match px.next() {
        TokenKind::Ident => parse_expr_like_stmt(px),
        TokenKind::Star => parse_label(px).map(PStmt::Label),
        TokenKind::Hash => parse_preproc_stmt(px),
        _ => return None,
    };

    parse_end_of_stmt(px);
    stmt_opt
}

pub(crate) fn parse_root(tokens: Vec<PToken>) -> PRoot {
    let mut px = Px::new(tokens);
    let mut stmts = vec![];

    loop {
        match px.next() {
            TokenKind::Eof => break,
            TokenKind::Eos
            | TokenKind::Eol
            | TokenKind::Colon
            | TokenKind::LeftBrace
            | TokenKind::RightBrace => px.skip(),
            _ => match parse_stmt(&mut px) {
                Some(stmt) => {
                    stmts.push(stmt);
                }
                None => px.skip(),
            },
        }
    }

    let (skipped, eof) = px.finish();

    PRoot {
        stmts,
        skipped,
        eof,
    }
}
