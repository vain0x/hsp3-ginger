use super::*;

type Px = ParseContext;

impl Token {
    pub(crate) fn is_str_literal_first(self) -> bool {
        match self {
            Token::DoubleQuote | Token::LeftQuote => true,
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

pub(crate) fn parse_str_literal(p: &mut Px) {
    assert!(p.next().is_str_literal_first());

    p.start_node();

    match p.next() {
        Token::DoubleQuote => {
            p.bump();

            while !p.at_eof() && !p.next().at_end_of_str() {
                assert!(p.next().is_str_content());
                p.bump();
            }

            p.eat(Token::DoubleQuote);
        }
        Token::LeftQuote => {
            p.bump();

            while !p.at_eof() && !p.next().at_end_of_multiline_str() {
                assert!(p.next().is_str_content());
                p.bump();
            }

            p.eat(Token::RightQuote);
        }
        _ => unreachable!("is_str_literal_first bug {:?}", p.next()),
    }

    p.end_node(NodeKind::StrLiteral);
}

pub(crate) fn parse_name(p: &mut Px) {
    assert_eq!(p.next(), Token::Ident);

    p.start_node();
    p.bump();
    p.end_node(NodeKind::Ident);
}
