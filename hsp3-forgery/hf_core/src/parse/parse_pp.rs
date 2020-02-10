use super::*;

type Px = ParseContext;

impl Token {
    pub(crate) fn at_end_of_pp(self) -> bool {
        self == Token::Eof || self == Token::Semi
    }
}

fn parse_end_of_pp(p: &mut Px) {
    if !p.at_eof() && !p.next().at_end_of_pp() {
        p.start_node();
        while !p.at_eof() && !p.next().at_end_of_pp() {
            p.bump();
        }
        p.end_node(NodeKind::Other);
    }

    if !p.at_eof() {
        p.bump();
    }
}

fn at_deffunc_like_keyword(p: &Px) -> bool {
    p.next() == Token::Ident && {
        match p.next_data().text() {
            "deffunc" => true,
            _ => false,
        }
    }
}

fn parse_deffunc_like_stmt_contents(p: &mut Px) {
    assert!(at_deffunc_like_keyword(p));

    p.bump();

    if !p.eat_ident("global") {
        p.eat_ident("local");
    }

    // modinit/modterm でなければ
    if p.next() == Token::Ident {
        parse_name(p);
    }

    if !p.eat_ident("onexit") {
        // params
    }
}

fn parse_module_stmt_contents(p: &mut Px) {
    assert!(p.next_data().text() == "module");

    p.bump();

    if !p.eat(Token::Ident) {
        if p.next() == Token::StrStart {
            parse_str_literal(p);
        }
    }
}

fn parse_global_stmt_contents(p: &mut Px) {
    assert!(p.next_data().text() == "global");

    p.bump();
}

pub(crate) fn parse_pp_stmt(p: &mut Px) {
    assert_eq!(p.next(), Token::Hash);

    p.start_node();

    p.bump();

    let kind = match p.next_data().text() {
        "deffunc" => {
            parse_deffunc_like_stmt_contents(p);
            NodeKind::DeffuncPp
        }
        "module" => {
            parse_module_stmt_contents(p);
            NodeKind::ModulePp
        }
        "global" => {
            parse_global_stmt_contents(p);
            NodeKind::GlobalPp
        }
        _ => NodeKind::UnknownPp,
    };

    parse_end_of_pp(p);

    p.end_node(kind);
}
