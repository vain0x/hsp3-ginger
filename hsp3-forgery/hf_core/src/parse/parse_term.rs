use super::*;

type Px = ParseContext;

impl Token {
    pub(crate) fn is_int_literal_first(self) -> bool {
        match self {
            Token::Digit | Token::ZeroB | Token::ZeroX | Token::Dollar | Token::Percent => true,
            _ => false,
        }
    }
}

pub(crate) fn parse_label_literal(p: &mut Px) {
    assert_eq!(p.next(), Token::Star);

    p.start_node();
    p.bump();

    match p.next() {
        Token::Ident => {
            p.bump();
        }
        Token::AtSign => {
            p.bump();
            p.eat(Token::Ident);
        }
        _ => {}
    }

    p.end_node(NodeKind::LabelLiteral);
}

pub(crate) fn parse_char_literal(p: &mut Px) {
    assert_eq!(p.next(), Token::CharStart);

    p.start_node();
    p.bump();

    p.end_node(NodeKind::CharLiteral);
}

pub(crate) fn parse_str_literal(p: &mut Px) {
    assert_eq!(p.next(), Token::StrStart);

    p.start_node();
    p.bump();

    p.end_node(NodeKind::StrLiteral);
}

pub(crate) fn parse_double_literal(p: &mut Px) {
    assert_eq!(p.next(), Token::FloatInt);

    p.start_node();
    p.bump();

    p.end_node(NodeKind::DoubleLiteral);
}

pub(crate) fn parse_int_literal(p: &mut Px) {
    assert!(p.next().is_int_literal_first());

    p.start_node();
    p.bump();

    p.end_node(NodeKind::IntLiteral);
}

pub(crate) fn parse_name(p: &mut Px) {
    assert_eq!(p.next(), Token::Ident);

    p.start_node();
    p.bump();
    p.end_node(NodeKind::Ident);
}
