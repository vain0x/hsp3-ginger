use crate::parse::bp::Bp;

use super::*;

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

fn parse_paren_expr(px: &mut Px) -> Option<PParenExpr> {
    let left_paren = px.eat(TokenKind::LeftParen)?;
    let body_opt = parse_expr(px).map(Box::new);
    let right_paren_opt = px.eat(TokenKind::RightParen);
    Some(PParenExpr {
        left_paren,
        body_opt,
        right_paren_opt,
    })
}

pub(crate) fn parse_atomic_expr(px: &mut Px) -> Option<PExpr> {
    match px.next() {
        TokenKind::Ident => parse_compound(px).map(PExpr::Compound),
        TokenKind::LeftParen => parse_paren_expr(px).map(PExpr::Paren),
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

fn parse_infix_expr_with_bp(px: &mut Px, bp: Bp) -> Option<PExpr> {
    if bp > Bp::MULDIV {
        return parse_prefix_expr(px);
    }

    let mut left = parse_infix_expr_with_bp(px, bp.next())?;

    loop {
        if px.next().is_infix_op() && Bp::from(px.next()) == bp {
            let infix = px.bump();
            let right_opt = parse_infix_expr_with_bp(px, bp.next()).map(Box::new);
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

fn parse_infix_expr(px: &mut Px) -> Option<PExpr> {
    parse_infix_expr_with_bp(px, Bp::BOOL)
}

pub(crate) fn parse_expr(px: &mut Px) -> Option<PExpr> {
    parse_infix_expr(px)
}

#[cfg(test)]
mod tests {
    use crate::{
        parse::{parse_context::Px, PExpr, PToken, PVisitor},
        source::DocId,
        token::{tokenize, TokenKind},
        utils::{rc_slice::RcSlice, rc_str::RcStr},
    };

    struct V {
        output: String,
    }

    impl PVisitor for V {
        fn on_expr(&mut self, expr: &crate::parse::PExpr) {
            if let PExpr::Infix(infix) = expr {
                self.output += "(";
                self.on_expr(&infix.left);
                self.output += " ";
                self.on_token(&infix.infix);
                self.output += " ";
                self.on_expr(&infix.right_opt.as_ref().unwrap());
                self.output += ")";
                return;
            } else {
                self.on_expr_default(expr);
            }
        }
        fn on_token(&mut self, token: &PToken) {
            self.output += token.body_text();
        }
    }

    fn f(expected: &str, input: &str) {
        let doc: DocId = 1;
        let text = RcStr::from(input);

        // tokenize
        let tokens: RcSlice<_> = tokenize(doc, RcStr::clone(&text)).into();

        // parse
        let p_tokens = PToken::from_tokens(tokens);
        let mut px = Px::new(p_tokens.to_owned());
        let expr = super::parse_expr(&mut px).unwrap();
        px.eat(TokenKind::Eos);
        let _ = px.finish();

        // print
        let mut v = V {
            output: String::new(),
        };
        PVisitor::on_expr(&mut v, &expr);
        let actual = v.output;

        assert_eq!(expected, &actual, "expr = {expr:?}");
    }

    #[test]
    fn test_infix_all_operators() {
        f("(((((_ & _) && _) ^ _) | _) || ((((((((_ ! _) != _) = _) == _) < _) <= _) > _) >= ((_ << _) >> ((_ - _) + (((_ * _) \\ _) / _)))))", "_&_&&_^_|_||_!_!=_=_==_<_<=_>_>=_<<_>>_-_+_*_\\_/_");
    }

    #[test]
    fn test_infix_bool() {
        // equal is stronger
        f("(((a & b) | (c == 0)) ^ d)", "a & b | c == 0 ^ d");
    }

    #[test]
    fn test_infix_compare() {
        // shift is stronger
        f("((a == b) & ((c << 1) != d))", "a == b & c << 1 != d");
    }

    #[test]
    fn test_infix_addsub() {
        // mul is stronger
        f("((a + (b * 2)) - c)", "a + b * 2 - c");
    }

    #[test]
    fn test_infix_muldiv() {
        // prefix is stronger
        f("((a * -b) / c)", "a * -b / c");
    }
}
