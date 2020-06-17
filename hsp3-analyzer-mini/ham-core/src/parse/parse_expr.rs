use super::{
    p_token::PToken, parse_context::Px, PArg, PCompound, PDotArg, PExpr, PGroupExpr, PInfixExpr,
    PLabel, PNameDot, PNameParen, PPrefixExpr,
};
use crate::token::TokenKind;

impl TokenKind {
    fn is_binary_op(self) -> bool {
        match self {
            TokenKind::LeftAngle
            | TokenKind::RightAngle
            | TokenKind::And
            | TokenKind::AndAnd
            | TokenKind::Backslash
            | TokenKind::Bang
            | TokenKind::Equal
            | TokenKind::EqualEqual
            | TokenKind::Hat
            | TokenKind::LeftEqual
            | TokenKind::LeftShift
            | TokenKind::Minus
            | TokenKind::Pipe
            | TokenKind::PipePipe
            | TokenKind::Plus
            | TokenKind::RightEqual
            | TokenKind::RightShift
            | TokenKind::Slash
            | TokenKind::Star => true,
            _ => false,
        }
    }
}

pub(crate) fn parse_label(px: &mut Px) -> Option<PLabel> {
    let star = px.eat(TokenKind::Star)?;
    let name_opt = px.eat(TokenKind::Ident);
    Some(PLabel { star, name_opt })
}

pub(crate) fn parse_args(px: &mut Px) -> Vec<PArg> {
    let mut args = vec![];

    loop {
        match px.next() {
            TokenKind::Eof
            | TokenKind::Eos
            | TokenKind::Eol
            | TokenKind::LeftBrace
            | TokenKind::RightBrace
            | TokenKind::Colon
            | TokenKind::RightParen => break,
            TokenKind::Comma => {
                let comma = px.bump();

                args.push(PArg {
                    expr_opt: None,
                    comma_opt: Some(comma),
                });
            }
            _ => match parse_expr(px) {
                None => break,
                Some(expr) => {
                    let comma_opt = px.eat(TokenKind::Comma);
                    args.push(PArg {
                        expr_opt: Some(expr),
                        comma_opt,
                    });
                }
            },
        }
    }

    args
}

fn parse_args_in_paren(px: &mut Px) -> Option<(PToken, Vec<PArg>, Option<PToken>)> {
    let left_paren = px.eat(TokenKind::LeftParen)?;
    let args = parse_args(px);
    let right_paren_opt = px.eat(TokenKind::RightParen);
    Some((left_paren, args, right_paren_opt))
}

pub(crate) fn parse_compound(px: &mut Px) -> Option<PCompound> {
    let name = px.eat(TokenKind::Ident)?;

    match px.next() {
        TokenKind::Dot => {
            let mut args = vec![];
            while let Some(dot) = px.eat(TokenKind::Dot) {
                let expr_opt = parse_expr(px);
                args.push(PDotArg { dot, expr_opt });
            }
            Some(PCompound::Dots(PNameDot { name, args }))
        }
        TokenKind::LeftParen => {
            let (left_paren, args, right_paren_opt) = parse_args_in_paren(px).unwrap();
            Some(PCompound::Paren(PNameParen {
                name,
                left_paren,
                args,
                right_paren_opt,
            }))
        }
        _ => Some(PCompound::Name(name)),
    }
}

fn parse_group_expr(px: &mut Px) -> Option<PGroupExpr> {
    let left_paren = px.eat(TokenKind::LeftParen)?;
    let body_opt = parse_expr(px).map(Box::new);
    let right_paren_opt = px.eat(TokenKind::RightParen);
    Some(PGroupExpr {
        left_paren,
        body_opt,
        right_paren_opt,
    })
}

pub(crate) fn parse_atomic_expr(px: &mut Px) -> Option<PExpr> {
    match px.next() {
        TokenKind::Ident => parse_compound(px).map(PExpr::Compound),
        TokenKind::LeftParen => parse_group_expr(px).map(PExpr::Group),
        TokenKind::Star => parse_label(px).map(PExpr::Label),
        TokenKind::Number | TokenKind::Char | TokenKind::Str => Some(PExpr::Literal(px.bump())),
        _ => None,
    }
}

fn parse_prefix_expr(px: &mut Px) -> Option<PExpr> {
    match px.next() {
        TokenKind::Minus => {
            let prefix = px.bump();
            let arg_opt = parse_prefix_expr(px).map(Box::new);
            Some(PExpr::Prefix(PPrefixExpr { prefix, arg_opt }))
        }
        _ => parse_atomic_expr(px),
    }
}

fn parse_infix_expr(px: &mut Px) -> Option<PExpr> {
    let mut left = parse_prefix_expr(px)?;

    loop {
        // 二項演算の優先順位はいまのところ無視する。
        if px.next().is_binary_op() {
            let infix = px.bump();
            let right_opt = parse_prefix_expr(px).map(Box::new);
            left = PExpr::Infix(PInfixExpr {
                left: Box::new(left),
                infix,
                right_opt,
            });
        } else {
            break;
        }
    }

    Some(left)
}

pub(crate) fn parse_expr(px: &mut Px) -> Option<PExpr> {
    parse_infix_expr(px)
}
