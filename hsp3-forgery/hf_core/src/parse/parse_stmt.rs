use super::*;

type Px = ParseContext;

fn parse_end_of_stmt(p: &mut Px) {
    if !p.at_eof() && !p.next().at_end_of_stmt() {
        p.start_node();

        while !p.at_eof() && !p.next().at_end_of_stmt() {
            p.bump();
        }

        p.end_node(NodeKind::Other);
    }

    if !p.at_eof() {
        assert!(p.next().at_end_of_stmt());
        p.bump();
    }
}

pub(crate) fn parse_root(p: &mut Px) {
    while !p.at_eof() {
        // エラー回復
        if !p.next().is_stmt_first() && !p.next().at_end_of_stmt() {
            p.start_node();

            while !p.at_eof() && !p.next().is_stmt_first() {
                p.bump();
            }

            p.end_node(NodeKind::Other);
            continue;
        }

        // parse_stmt(p);
        parse_end_of_stmt(p)
    }
}
