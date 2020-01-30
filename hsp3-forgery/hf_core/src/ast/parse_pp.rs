use super::*;
use crate::syntax::*;

type Px = ParseContext;

impl Token {
    pub(crate) fn at_end_of_pp(self) -> bool {
        self == Token::Eof || self == Token::Eol
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

pub(crate) fn parse_pp_stmt(hash: TokenData, p: &mut Px) -> AStmt {
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
