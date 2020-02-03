use super::*;

type Px = ParseContext;

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

pub(crate) fn parse_pp_stmt(p: &mut Px) {
    assert_eq!(p.next(), Token::Hash);

    p.start_node();

    p.bump();
    parse_end_of_pp(p);

    p.end_node(NodeKind::UnknownPp);
}
