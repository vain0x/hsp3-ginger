use super::parse_expr::*;
use super::parse_pp::*;
use super::*;
use crate::syntax::*;
use parse_context::ParseContext;

type Px = ParseContext;

impl Token {
    pub(crate) fn is_stmt_first(self, line_head: bool) -> bool {
        match self {
            Token::Hash if line_head => true,
            Token::Ident | Token::Star => true,
            _ => self.is_control_keyword(),
        }
    }

    pub(crate) fn is_stmt_follow(self) -> bool {
        self.at_end_of_stmt()
    }

    pub(crate) fn is_command_first(self) -> bool {
        self.is_jump_keyword()
    }

    pub(crate) fn at_end_of_stmt(self) -> bool {
        self == Token::Eof
            || self == Token::Eol
            || self == Token::RightBrace
            || self == Token::Colon
    }
}

fn parse_end_of_stmt(p: &mut Px) -> Option<TokenData> {
    if !p.next().at_end_of_stmt() {
        p.error_next("解釈できない字句です");

        while !p.at_eof() && !p.next().at_end_of_stmt() {
            p.bump();
        }
    }

    if p.next().at_end_of_stmt() {
        Some(p.bump())
    } else {
        None
    }
}

fn parse_label_stmt(p: &mut Px) -> ALabelStmt {
    assert_eq!(p.next(), Token::Star);

    let label = parse_label(p);
    let sep_opt = parse_end_of_stmt(p);

    ALabelStmt { label, sep_opt }
}

fn parse_command_contents(head: TokenData, p: &mut Px) -> AStmt {
    // FIXME: goto/gosub

    let mut args = vec![];
    if p.next().is_arg_first() {
        parse_args(&mut args, p);
    }

    let sep_opt = parse_end_of_stmt(p);

    AStmt::Command(ACommandStmt {
        command: head,
        args,
        sep_opt,
    })
}

fn parse_command_stmt(p: &mut Px) -> AStmt {
    assert!(p.next().is_command_first());

    let head = p.bump();
    parse_command_contents(head, p)
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

            let sep_opt = parse_end_of_stmt(p);

            AStmt::Assign(AAssignStmt {
                left: head,
                equal,
                right_opt,
                sep_opt,
            })
        }
        _ if p.next().is_arg_first() || p.next().at_end_of_stmt() => {
            parse_command_contents(head, p)
        }
        _ => {
            unimplemented!("{:?}", p.next_data());
        }
    }
}

fn parse_stmt(p: &mut Px) -> AStmt {
    match p.next() {
        Token::Ident => parse_assign_or_command_stmt(p),
        Token::Hash => {
            let hash = p.bump();
            parse_pp_stmt(hash, p)
        }
        Token::Star => AStmt::Label(parse_label_stmt(p)),
        _ if p.next().is_command_first() => parse_command_stmt(p),
        _ => unimplemented!("{:?}", p.next_data()),
    }
}

fn parse_root(p: &mut Px) -> ARoot {
    let mut children = vec![];

    while !p.at_eof() {
        if p.next() == Token::Eol || p.next() == Token::Colon {
            p.bump();
            continue;
        }

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
