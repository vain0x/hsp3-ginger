use super::*;

type Px = ParseContext;

pub(crate) fn parse_root(p: &mut Px) {
    while !p.at_eof() {
        if p.next() == Token::Eol {
            p.bump();
            continue;
        }

        // エラー回復
        if !p.next().is_stmt_first() {
            while !p.at_eof() && !p.next().is_stmt_first() {
                p.bump();
            }
            continue;
        }

        // let stmt = parse_stmt(p);
        p.bump();
    }
}
