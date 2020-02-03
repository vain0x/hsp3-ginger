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

fn parse_assign_or_command_stmt(p: &mut Px) {
    assert_eq!(p.next(), Token::Ident);

    let kind;
    p.start_node();
    p.bump();

    match p.next() {
        Token::Equal => {
            p.bump();

            if p.next().is_expr_first() {
                parse_expr(p);
            }

            kind = NodeKind::AssignStmt;
        }
        _ => {
            kind = NodeKind::Other;
        }
    }

    parse_end_of_stmt(p);
    p.end_node(kind);
}

fn parse_label_stmt(p: &mut Px) {
    assert_eq!(p.next(), Token::Star);

    p.start_node();

    parse_label_literal(p);
    parse_end_of_stmt(p);

    p.end_node(NodeKind::LabelStmt);
}

fn parse_stmt(p: &mut Px) {
    match p.next() {
        Token::Ident => parse_assign_or_command_stmt(p),
        Token::Star => parse_label_stmt(p),
        _ => {
            // assert!(p.next().at_end_of_stmt(), "is_stmt_first/at_end_of_stmt bug");
            parse_end_of_stmt(p);
        }
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

        parse_stmt(p);
    }
}
