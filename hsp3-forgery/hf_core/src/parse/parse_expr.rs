use super::*;

type Px = ParseContext;

pub(crate) fn parse_expr(p: &mut Px) {
    match p.next() {
        Token::Ident => parse_name(p),
        Token::Star => parse_label_literal(p),
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
