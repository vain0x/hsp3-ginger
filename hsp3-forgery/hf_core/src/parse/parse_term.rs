use super::*;

type Px = ParseContext;

pub(crate) fn parse_name(p: &mut Px) {
    assert_eq!(p.next(), Token::Ident);

    p.start_node();
    p.bump();
    p.end_node(NodeKind::Ident);
}
