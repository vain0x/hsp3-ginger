use super::*;
use crate::syntax::*;
use parse_context::ParseContext;

type Px = ParseContext;

impl Token {
    fn at_end_of_str(self) -> bool {
        self.at_end_of_stmt() || self == Token::DoubleQuote
    }

    fn is_expr_first(self) -> bool {
        self == Token::Digit
            || self == Token::SingleQuote
            || self == Token::DoubleQuote
            || self == Token::Ident
            || self == Token::LeftParen
            || self == Token::Minus
            || self == Token::Star
    }

    fn at_end_of_expr(self) -> bool {
        self.at_end_of_stmt() || self == Token::RightParen
    }

    fn is_arg_first(self) -> bool {
        self.is_expr_first() || self == Token::Comma
    }

    fn at_end_of_args(self) -> bool {
        self.at_end_of_expr() || self.at_end_of_stmt()
    }

    fn is_stmt_first(self, line_head: bool) -> bool {
        (line_head && self == Token::Hash)
            || self == Token::Ident
            || self == Token::Star
            || self.is_control_keyword()
    }

    fn is_stmt_follow(self) -> bool {
        self.at_end_of_stmt()
    }

    fn at_end_of_pp(self) -> bool {
        self == Token::Eof || self == Token::Eol
    }

    fn at_end_of_stmt(self) -> bool {
        self == Token::Eof
            || self == Token::Eol
            || self == Token::RightBrace
            || self == Token::Colon
    }
}

fn parse_end_of_pp(p: &mut Px) {
    if !p.next().at_end_of_pp() {
        p.error_next("余分な字句です");

        while !p.at_eof() && !p.next().at_end_of_pp() {
            p.bump();
        }
    }

    p.eat(Token::Eol);
}

fn parse_end_of_stmt(p: &mut Px) {
    if !p.next().at_end_of_stmt() {
        p.error_next("解釈できない字句です");

        while !p.at_eof() && !p.next().at_end_of_stmt() {
            p.bump();
        }
    }

    if !p.at_eof() && p.next().at_end_of_stmt() {
        p.bump();
    }
}

fn parse_int_expr(p: &mut Px) -> AIntExpr {
    assert_eq!(p.next(), Token::Digit);

    let token = p.bump();

    AIntExpr { token }
}

fn parse_str_expr(p: &mut Px) -> AStrExpr {
    assert_eq!(p.next(), Token::DoubleQuote);

    let start_quote = p.bump();

    let mut segments = vec![];
    while !p.at_eof() && !p.next().at_end_of_str() {
        segments.push(p.bump());
    }

    let end_quote_opt = p.eat(Token::DoubleQuote);

    AStrExpr {
        start_quote,
        segments,
        end_quote_opt,
    }
}

fn parse_name_expr(p: &mut Px) -> ANameExpr {
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

fn parse_expr(p: &mut Px) -> AExpr {
    assert!(p.next().is_expr_first());

    match p.next() {
        Token::Digit => AExpr::Int(parse_int_expr(p)),
        Token::Ident => parse_call_expr(p),
        Token::LeftParen => AExpr::Group(parse_group_expr(p)),
        Token::DoubleQuote => AExpr::Str(parse_str_expr(p)),
        _ => unimplemented!("{:?}", p.next_data()),
    }
}

fn parse_return_stmt(p: &mut Px) -> AReturnStmt {
    assert_eq!(p.next(), Token::Return);

    let keyword = p.bump();

    let result_opt = if p.next().is_expr_first() {
        Some(parse_expr(p))
    } else {
        None
    };

    parse_end_of_stmt(p);

    AReturnStmt {
        keyword,
        result_opt,
    }
}

fn at_deffunc_like_keyword(p: &Px) -> bool {
    if p.next() != Token::Ident {
        return false;
    }

    match p.next_data().text() {
        "deffunc" => true,
        _ => false,
    }
}

fn parse_deffunc_like_stmt(hash: TokenData, p: &mut Px) -> ADeffuncStmt {
    assert!(at_deffunc_like_keyword(p));

    let keyword = p.bump();

    if let Some(_) = p.eat_ident("global") {
        // global
    }

    if let Some(_) = p.eat_ident("local") {
        // local
    }

    // modinit/modterm でなければ
    let name_opt = p.eat(Token::Ident);

    if let Some(_) = p.eat_ident("onexit") {
        // onexit
    }

    // params

    parse_end_of_pp(p);

    ADeffuncStmt {
        hash,
        keyword,
        name_opt,
    }
}

fn parse_module_stmt(hash: TokenData, p: &mut Px) -> AModuleStmt {
    assert!(p.next_data().text() == "module");

    let keyword = p.bump();

    // FIXME: or string
    let name_opt = p.eat(Token::Ident);

    parse_end_of_pp(p);

    AModuleStmt {
        hash,
        keyword,
        name_opt,
    }
}

fn parse_global_stmt(hash: TokenData, p: &mut Px) -> AGlobalStmt {
    assert!(p.next_data().text() == "global");

    let keyword = p.bump();
    parse_end_of_pp(p);

    AGlobalStmt { hash, keyword }
}

fn parse_unknown_pp_stmt(hash: TokenData, p: &mut Px) -> AStmt {
    while !p.at_eof() && !p.next().at_end_of_pp() {
        p.bump();
    }

    AStmt::UnknownPreprocessor { hash }
}

fn parse_pp_stmt(hash: TokenData, p: &mut Px) -> AStmt {
    if p.next() != Token::Ident {
        return parse_unknown_pp_stmt(hash, p);
    }

    match p.next_data().text() {
        "module" => AStmt::Module(parse_module_stmt(hash, p)),
        "global" => AStmt::Global(parse_global_stmt(hash, p)),
        "deffunc" => AStmt::Deffunc(parse_deffunc_like_stmt(hash, p)),
        _ => unimplemented!("{:?}", p.next_data()),
    }
}

fn parse_args(args: &mut Vec<AArg>, p: &mut Px) {
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

fn parse_assign_or_command_stmt(p: &mut Px) -> AStmt {
    assert_eq!(p.next(), Token::Ident);
    let head = p.bump();

    match p.next() {
        Token::Equal => {
            let equal = p.bump();

            // FIXME: 右辺はカンマ区切り
            let right_opt = if p.next().is_expr_first() {
                Some(parse_expr(p))
            } else {
                None
            };

            parse_end_of_stmt(p);

            AStmt::Assign(AAssignStmt {
                left: head,
                equal,
                right_opt,
            })
        }
        _ if p.next().is_expr_first() => {
            let mut args = vec![];

            parse_args(&mut args, p);
            parse_end_of_stmt(p);

            AStmt::Command(ACommandStmt {
                command: head,
                args,
            })
        }
        _ => {
            unimplemented!("{:?}", p.next_data());
        }
    }
}

fn parse_stmt(p: &mut Px) -> AStmt {
    match p.next() {
        Token::Hash => {
            let hash = p.bump();
            parse_pp_stmt(hash, p)
        }
        Token::Return => AStmt::Return(parse_return_stmt(p)),
        Token::Ident => parse_assign_or_command_stmt(p),
        _ => unimplemented!("{:?}", p.next_data()),
    }
}

fn parse_root(p: &mut Px) -> ARoot {
    let mut children = vec![];

    while !p.at_eof() {
        if !p.next().is_stmt_first(p.at_head()) {
            p.error_next("文が必要です");

            // エラー回復
            while !p.at_eof() && !p.next().is_stmt_first(p.at_head()) {
                p.bump();
            }
            continue;
        }

        let stmt = parse_stmt(p);
        children.push(stmt);
    }

    ARoot {
        children,
        errors: vec![],
    }
}

pub(crate) fn parse_tokens(tokens: &[TokenData]) -> ARoot {
    let tokens = tokens
        .into_iter()
        .filter(|t| t.token() != Token::Space)
        .cloned()
        .collect::<Vec<_>>();

    let mut p = ParseContext::new(tokens);
    let mut root = parse_root(&mut p);

    let (_, errors) = p.finish();
    root.errors.extend(errors);

    root
}
