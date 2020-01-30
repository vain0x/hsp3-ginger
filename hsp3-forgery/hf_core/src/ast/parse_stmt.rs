use super::parse_expr::*;
use super::parse_pp::*;
use super::*;
use crate::syntax::*;
use parse_context::ParseContext;
type Px = ParseContext;

impl Token {
    pub(crate) fn is_stmt_first(self, line_head: bool) -> bool {
        (line_head && self == Token::Hash)
            || self == Token::Ident
            || self == Token::Star
            || self.is_control_keyword()
    }

    pub(crate) fn is_stmt_follow(self) -> bool {
        self.at_end_of_stmt()
    }

    pub(crate) fn at_end_of_stmt(self) -> bool {
        self == Token::Eof
            || self == Token::Eol
            || self == Token::RightBrace
            || self == Token::Colon
    }
}

fn parse_end_of_stmt(p: &mut Px) {
    if !p.next().at_end_of_stmt() {
        p.error_next("解釈できない字句です");

        while !p.at_eof() && !p.next().at_end_of_stmt() {
            p.bump();
        }
    }

    if !p.at_eof() && p.next().at_end_of_stmt() {
        p.bump();
    }
}

fn parse_return_stmt(p: &mut Px) -> AReturnStmt {
    assert_eq!(p.next(), Token::Return);

    let keyword = p.bump();

    let result_opt = if p.next().is_expr_first() {
        Some(parse_expr(p))
    } else {
        None
    };

    parse_end_of_stmt(p);

    AReturnStmt {
        keyword,
        result_opt,
    }
}

fn parse_assign_or_command_stmt(p: &mut Px) -> AStmt {
    assert_eq!(p.next(), Token::Ident);
    let head = p.bump();

    match p.next() {
        Token::Equal => {
            let equal = p.bump();

            // FIXME: 右辺はカンマ区切り
            let right_opt = if p.next().is_expr_first() {
                Some(parse_expr(p))
            } else {
                None
            };

            parse_end_of_stmt(p);

            AStmt::Assign(AAssignStmt {
                left: head,
                equal,
                right_opt,
            })
        }
        _ if p.next().is_expr_first() => {
            let mut args = vec![];

            parse_args(&mut args, p);
            parse_end_of_stmt(p);

            AStmt::Command(ACommandStmt {
                command: head,
                args,
            })
        }
        _ => {
            unimplemented!("{:?}", p.next_data());
        }
    }
}

fn parse_stmt(p: &mut Px) -> AStmt {
    match p.next() {
        Token::Hash => {
            let hash = p.bump();
            parse_pp_stmt(hash, p)
        }
        Token::Ident => parse_assign_or_command_stmt(p),
        Token::Return => AStmt::Return(parse_return_stmt(p)),
        _ => unimplemented!("{:?}", p.next_data()),
    }
}

fn parse_root(p: &mut Px) -> ARoot {
    let mut children = vec![];

    while !p.at_eof() {
        if !p.next().is_stmt_first(p.at_head()) {
            p.error_next("文が必要です");

            // エラー回復
            while !p.at_eof() && !p.next().is_stmt_first(p.at_head()) {
                p.bump();
            }
            continue;
        }

        let stmt = parse_stmt(p);
        children.push(stmt);
    }

    ARoot {
        children,
        errors: vec![],
    }
}

pub(crate) fn parse_tokens(tokens: &[TokenData]) -> ARoot {
    let tokens = tokens
        .into_iter()
        .filter(|t| t.token() != Token::Space)
        .cloned()
        .collect::<Vec<_>>();

    let mut p = ParseContext::new(tokens);
    let mut root = parse_root(&mut p);

    let (_, errors) = p.finish();
    root.errors.extend(errors);

    root
}
