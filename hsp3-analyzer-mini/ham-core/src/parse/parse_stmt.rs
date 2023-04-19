use super::{
    p_op_kind::POpKind,
    parse_context::Px,
    parse_expr::{parse_args, parse_atomic_expr, parse_compound, parse_expr, parse_label},
    parse_preproc::parse_preproc_stmt,
    PAssignStmt, PBlock, PCommandStmt, PIfStmt, PInvokeStmt, PJumpModifier, PRoot, PStmt, PToken,
};
use crate::token::TokenKind;

/// 先読みトークン数の上限
const LOOKAHEAD_LIMIT: usize = 30;

enum ExprLikeStmtKind {
    Assign,
    Command,
    Invoke,
}

impl TokenKind {
    fn is_end_of_stmt(self) -> bool {
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
    while !px.next().is_end_of_stmt() {
        px.skip();
    }
}

fn parse_jump_modifier(px: &mut Px) -> Option<(PJumpModifier, PToken)> {
    if px.next() != TokenKind::Ident {
        return None;
    }

    let jump_modifier = PJumpModifier::parse(px.next_token().body_text())?;
    let token = px.bump();
    Some((jump_modifier, token))
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
            _ if kind.to_op_kind() == Some(POpKind::Assign) => {
                return ExprLikeStmtKind::Assign;
            }
            _ if kind.is_end_of_stmt() => break,
            _ if i >= LOOKAHEAD_LIMIT => {
                // 長い文はたぶん命令文。
                return ExprLikeStmtKind::Command;
            }
            _ => {}
        }
    }

    match px.nth(i) {
        TokenKind::Plus | TokenKind::Minus if px.nth(i + 1).is_end_of_stmt() => {
            // `x+`
            ExprLikeStmtKind::Assign
        }
        TokenKind::SlimArrow => ExprLikeStmtKind::Invoke,

        // HACK: `=` は二項演算子としても使えるが、ここでは代入演算子とみなす。
        //       これがないと `a(i) = x` が命令文になってしまう。
        TokenKind::Equal => ExprLikeStmtKind::Assign,

        kind if kind.is_end_of_stmt() => ExprLikeStmtKind::Command,
        kind => match kind.to_op_kind() {
            None | Some(POpKind::Infix) | Some(POpKind::InfixOrAssign) => ExprLikeStmtKind::Command,
            Some(POpKind::Assign) | Some(POpKind::PrefixOrInfixOrAssign) => {
                ExprLikeStmtKind::Assign
            }
        },
    }
}

fn lookahead_stmt(px: &mut Px) -> ExprLikeStmtKind {
    match px.nth(1) {
        TokenKind::LeftParen => lookahead_after_paren(2, px),
        TokenKind::Dot => ExprLikeStmtKind::Assign,
        TokenKind::SlimArrow => ExprLikeStmtKind::Invoke,
        TokenKind::Plus | TokenKind::Minus if px.nth(2).is_end_of_stmt() => {
            ExprLikeStmtKind::Assign
        }
        second => match second.to_op_kind() {
            None | Some(POpKind::Infix) | Some(POpKind::PrefixOrInfixOrAssign) => {
                ExprLikeStmtKind::Command
            }
            Some(POpKind::InfixOrAssign) | Some(POpKind::Assign) => ExprLikeStmtKind::Assign,
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

    let op_opt = if px.next().is_assign_op() {
        Some(px.bump())
    } else {
        None
    };

    let args = parse_args(px);
    parse_end_of_stmt(px);

    Some(PAssignStmt { left, op_opt, args })
}

fn parse_command_stmt(px: &mut Px) -> Option<PCommandStmt> {
    let command = px.bump();
    let jump_modifier_opt = parse_jump_modifier(px);
    let args = parse_args(px);
    parse_end_of_stmt(px);

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
    parse_end_of_stmt(px);

    Some(PInvokeStmt {
        left,
        arrow_opt,
        method_opt,
        args,
    })
}

fn parse_block(px: &mut Px) -> PBlock {
    let mut block = PBlock::default();

    // outer_stmtsをパースしながら `{` または `else` を探す。
    let left = loop {
        match px.next() {
            TokenKind::Eof | TokenKind::Eos | TokenKind::RightBrace | TokenKind::Else => {
                return block
            }
            TokenKind::LeftBrace => break px.bump(),
            TokenKind::Colon => {
                px.skip();
                block.outer_stmts.extend(parse_stmt(px));
            }
            _ => px.skip(),
        }
    };
    block.left_opt = Some(left);

    // inner_stmtsをパースしながら `}` を探す。
    let right = loop {
        match px.next() {
            TokenKind::Eof => return block,
            TokenKind::RightBrace => break px.bump(),
            TokenKind::Eos | TokenKind::Colon => px.skip(),
            _ => match parse_stmt(px) {
                Some(stmt) => block.inner_stmts.push(stmt),
                None => px.skip(),
            },
        }
    };
    block.right_opt = Some(right);

    block
}

fn parse_if_stmt(px: &mut Px) -> Option<PIfStmt> {
    let command = px.bump();
    let cond_opt = parse_expr(px);
    let body = parse_block(px);

    let else_opt = match (px.next(), px.nth(1)) {
        // elseの直前は1個だけ改行が認められる。
        (TokenKind::Eos, TokenKind::Else) => {
            px.skip();
            Some(px.bump()) // else
        }
        (TokenKind::Else, _) => Some(px.bump()),
        _ => None,
    };
    let alt = if else_opt.is_some() {
        parse_block(px)
    } else {
        PBlock::default()
    };
    parse_end_of_stmt(px);

    Some(PIfStmt {
        command,
        cond_opt,
        body,
        else_opt,
        alt,
    })
}

pub(crate) fn parse_stmt(px: &mut Px) -> Option<PStmt> {
    let stmt_opt = match px.next() {
        TokenKind::Ident => parse_expr_like_stmt(px),
        TokenKind::If => parse_if_stmt(px).map(PStmt::If),
        TokenKind::Star => parse_label(px).map(PStmt::Label),
        TokenKind::Hash => parse_preproc_stmt(px),
        _ => return None,
    };
    stmt_opt
}

pub(crate) fn parse_root(tokens: Vec<PToken>) -> PRoot {
    let mut px = Px::new(tokens);
    let mut stmts = vec![];

    loop {
        match px.next() {
            TokenKind::Eof => break,
            TokenKind::Eos | TokenKind::Colon | TokenKind::LeftBrace | TokenKind::RightBrace => {
                px.skip()
            }
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
