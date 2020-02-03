use super::*;

type Px = ParseContext;

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

pub(crate) fn parse_name(p: &mut Px) {
    assert_eq!(p.next(), Token::Ident);

    p.start_node();
    p.bump();
    p.end_node(NodeKind::Ident);
}
