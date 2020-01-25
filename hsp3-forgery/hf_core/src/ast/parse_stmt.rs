use super::*;
use crate::syntax::*;
use parse_context::ParseContext;

type Px = ParseContext;

impl Token {
    fn is_expr_first(self) -> bool {
        self == Token::Digit
            || self == Token::SingleQuote
            || self == Token::DoubleQuote
            || self == Token::Ident
            || self == Token::LeftParen
            || self == Token::Minus
            || self == Token::Star
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

fn parse_expr(p: &mut Px) -> AExpr {
    assert!(p.next().is_expr_first());

    match p.next() {
        Token::Digit => AExpr::Int(parse_int_expr(p)),
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
        _ => unimplemented!("{:?}", p.next_data()),
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

pub(crate) fn parse_tokens(mut tokens: Vec<TokenData>) -> ARoot {
    tokens.retain(|t| t.token() != Token::Space);

    let mut p = ParseContext::new(tokens);
    let mut root = parse_root(&mut p);

    let (_, errors) = p.finish();
    root.errors.extend(errors);

    root
}
