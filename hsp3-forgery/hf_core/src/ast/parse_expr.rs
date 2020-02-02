use super::*;
use parse_context::ParseContext;

type Px = ParseContext;

impl Token {
    pub(crate) fn is_str_content(self) -> bool {
        self == Token::StrVerbatim
    }

    pub(crate) fn at_end_of_str(self) -> bool {
        self.at_end_of_stmt() || self == Token::DoubleQuote
    }

    pub(crate) fn at_end_of_multiline_str(self) -> bool {
        match self {
            Token::Eof | Token::RightQuote => true,
            _ => false,
        }
    }

    pub(crate) fn is_expr_first(self) -> bool {
        match self {
            Token::Digit
            | Token::SingleQuote
            | Token::DoubleQuote
            | Token::LeftQuote
            | Token::Ident
            | Token::LeftParen
            | Token::Minus
            | Token::Star => true,
            _ => false,
        }
    }

    pub(crate) fn at_end_of_expr(self) -> bool {
        self.at_end_of_stmt() || self == Token::RightParen
    }

    pub(crate) fn is_arg_first(self) -> bool {
        self.is_expr_first() || self == Token::Comma
    }

    pub(crate) fn at_end_of_args(self) -> bool {
        self.at_end_of_expr() || self.at_end_of_stmt()
    }
}

pub(crate) fn parse_label(p: &mut Px) -> ALabel {
    assert_eq!(p.next(), Token::Star);

    let star = p.bump();

    // FIXME: 前後の空白を検査する。
    match p.next() {
        Token::Ident => {
            let ident = p.bump();
            ALabel::Name { star, ident }
        }
        Token::AtSign => {
            let at_sign = p.bump();
            let ident_opt = p.eat(Token::Ident);
            ALabel::Anonymous {
                star,
                at_sign,
                ident_opt,
            }
        }
        _ => {
            p.error("ラベルの名前がありません", &star);
            ALabel::StarOnly { star }
        }
    }
}

pub(crate) fn parse_int_expr(p: &mut Px) -> AIntExpr {
    assert_eq!(p.next(), Token::Digit);

    let token = p.bump();

    AIntExpr { token }
}

pub(crate) fn parse_str_expr(p: &mut Px) -> AStrExpr {
    assert_eq!(p.next(), Token::DoubleQuote);

    let start_quote = p.bump();

    let mut segments = vec![];
    while !p.at_eof() && !p.next().at_end_of_str() {
        assert!(p.next().is_str_content());
        segments.push(p.bump());
    }

    let end_quote_opt = p.eat(Token::DoubleQuote);

    AStrExpr {
        start_quote,
        segments,
        end_quote_opt,
    }
}

pub(crate) fn parse_multiline_str_expr(p: &mut Px) -> AStrExpr {
    assert_eq!(p.next(), Token::LeftQuote);

    let start_quote = p.bump();

    let mut segments = vec![];
    while !p.at_eof() && !p.next().at_end_of_multiline_str() {
        assert!(p.next().is_str_content());
        segments.push(p.bump());
    }

    let end_quote_opt = p.eat(Token::RightQuote);

    AStrExpr {
        start_quote,
        segments,
        end_quote_opt,
    }
}

pub(crate) fn parse_name_expr(p: &mut Px) -> ANameExpr {
    assert_eq!(p.next(), Token::Ident);

    let token = p.bump();

    ANameExpr { token }
}

fn parse_group_expr(p: &mut Px) -> AGroupExpr {
    assert_eq!(p.next(), Token::LeftParen);

    let left_paren = p.bump();

    let body_opt = if p.next().is_expr_first() {
        Some(Box::new(parse_expr(p)))
    } else {
        p.error("カッコの中は空にできません", &left_paren);
        None
    };

    let right_paren_opt = p.eat(Token::RightParen);
    if right_paren_opt.is_none() {
        p.error("対応する右カッコがありません", &left_paren);
    }

    AGroupExpr {
        left_paren,
        body_opt,
        right_paren_opt,
    }
}

fn parse_call_expr(p: &mut Px) -> AExpr {
    assert_eq!(p.next(), Token::Ident);

    let name_expr = parse_name_expr(p);

    // FIXME: . 記法

    let left_paren = match p.eat(Token::LeftParen) {
        None => return AExpr::Name(name_expr),
        Some(left_paren) => left_paren,
    };

    let mut args = vec![];
    if p.next().is_arg_first() {
        parse_args(&mut args, p);
    }

    let right_paren_opt = p.eat(Token::RightParen);
    if right_paren_opt.is_none() {
        p.error("対応する右カッコがありません", &left_paren);
    }

    AExpr::Call(ACallExpr {
        cal: name_expr,
        left_paren_opt: Some(left_paren),
        args,
        right_paren_opt,
    })
}

pub(crate) fn parse_expr(p: &mut Px) -> AExpr {
    assert!(p.next().is_expr_first());

    match p.next() {
        Token::Digit => AExpr::Int(parse_int_expr(p)),
        Token::Ident => parse_call_expr(p),
        Token::LeftParen => AExpr::Group(parse_group_expr(p)),
        Token::DoubleQuote => AExpr::Str(parse_str_expr(p)),
        Token::LeftQuote => AExpr::Str(parse_multiline_str_expr(p)),
        Token::Star => AExpr::Label(parse_label(p)),
        _ => unimplemented!("{:?}", p.next_data()),
    }
}

pub(crate) fn parse_args(args: &mut Vec<AArg>, p: &mut Px) {
    assert!(p.next().is_arg_first());

    loop {
        let arg = if let Some(comma) = p.eat(Token::Comma) {
            AArg {
                expr_opt: None,
                comma_opt: Some(comma),
            }
        } else if p.next().is_expr_first() {
            let expr = parse_expr(p);
            let comma_opt = p.eat(Token::Comma);
            AArg {
                expr_opt: Some(expr),
                comma_opt: comma_opt,
            }
        } else {
            unreachable!("ERROR: is_arg_first bug")
        };

        let ends_with_comma = arg.comma_opt.is_some();
        args.push(arg);

        if ends_with_comma || p.next().is_arg_first() {
            continue;
        }

        if !p.next().at_end_of_args() {
            let bad_token = p.bump();
            p.error("引数リストの終端が期待されました", &bad_token);
            continue;
        }

        break;
    }
}
