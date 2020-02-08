use super::*;

type Px = ParseContext;

impl Token {
    pub(crate) fn is_atom_expr_first(self) -> bool {
        match self {
            Token::CharStart
            | Token::StrStart
            | Token::FloatInt
            | Token::Ident
            | Token::LeftParen => true,
            _ => self.is_int_literal_first(),
        }
    }

    /// このトークンが式の先頭になることがあるか？
    /// (= expr の FIRST 集合に含まれるか？)
    pub(crate) fn is_expr_first(self) -> bool {
        match self {
            Token::Minus | Token::Star => true,
            _ => self.is_atom_expr_first(),
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

fn parse_group_expr(p: &mut Px) {
    assert_eq!(p.next(), Token::LeftParen);

    p.bump();

    if p.next().is_expr_first() {
        parse_expr(p);
    }

    p.eat(Token::RightParen);
}

pub(crate) fn parse_call_expr(p: &mut Px) {
    assert_eq!(p.next(), Token::Ident);

    parse_name(p);

    // FIXME: . 記法

    if !p.eat(Token::LeftParen) {
        return;
    }

    p.restart_node();

    parse_args(p);

    p.eat(Token::RightParen);

    p.end_node(NodeKind::CallExpr);
}

fn parse_unary_expr(p: &mut Px) {
    assert_eq!(p.next(), Token::Minus);

    p.start_node();
    p.bump();

    if p.next().is_expr_first() {
        parse_expr(p);
    }
    p.end_node(NodeKind::UnaryExpr);
}

pub(crate) fn parse_expr(p: &mut Px) {
    match p.next() {
        Token::Ident => parse_call_expr(p),
        Token::LeftParen => parse_group_expr(p),
        Token::CharStart => parse_char_literal(p),
        Token::FloatInt => parse_double_literal(p),
        Token::Minus => parse_unary_expr(p),
        Token::Star => parse_label_literal(p),
        Token::StrStart => parse_str_literal(p),
        _ if p.next().is_int_literal_first() => parse_int_literal(p),
        _ => {
            // unimplemented
            p.bump();
        }
    }
}

/// 引数リスト (カンマ区切りの式の並び) を解析する。
pub(crate) fn parse_args(p: &mut Px) {
    let mut ends_with_comma = false;

    loop {
        // エラー回復
        if !p.at_eof() && !p.next().is_arg_first() && !p.next().at_end_of_args() {
            p.start_node();
            while !p.at_eof() && !p.next().is_arg_first() && !p.next().at_end_of_args() {
                p.bump();
            }
            p.end_node(NodeKind::Other);
        }

        if !p.next().is_arg_first() {
            break;
        }

        p.start_node();

        if p.next().is_expr_first() {
            parse_expr(p);
        }

        ends_with_comma = p.eat(Token::Comma);

        p.end_node(NodeKind::Arg);
    }

    if ends_with_comma {
        p.start_node();
        p.end_node(NodeKind::Arg);
    }
}
