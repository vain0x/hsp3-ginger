use super::{
    p_token::PToken, parse_context::Px, parse_stmt::parse_stmt, PDefFuncStmt, PGlobalStmt,
    PModuleStmt, PParam, PStmt,
};
use crate::{analysis::AParamTy, token::TokenKind};
use std::str::FromStr;

static DEFFUNC_LIKE_KEYWORDS: &[&str] = &["deffunc", "defcfunc", "modfunc", "modcfunc"];

impl TokenKind {
    fn at_end_of_preproc(self) -> bool {
        match self {
            TokenKind::Eof | TokenKind::Eos => true,
            _ => false,
        }
    }
}

impl PToken {
    /// `deffunc` 系の命令の領域を分割するプリプロセッサ命令の名前
    fn at_deffunc_terminator(&self) -> bool {
        self.kind() == TokenKind::Ident
            && (DEFFUNC_LIKE_KEYWORDS.contains(&self.body_text())
                || self.body_text() == "module"
                || self.body_text() == "global")
    }
}

fn eat_ident(pattern: &str, px: &mut Px) -> Option<PToken> {
    if px.next() == TokenKind::Ident && px.next_token().body_text() == pattern {
        Some(px.bump())
    } else {
        None
    }
}

fn eat_privacy(px: &mut Px) -> Option<PToken> {
    if px.next() != TokenKind::Ident {
        return None;
    }

    match px.next_token().body_text() {
        "global" | "local" => Some(px.bump()),
        _ => None,
    }
}

fn eat_param_ty(px: &mut Px) -> Option<PToken> {
    if px.next() != TokenKind::Ident {
        return None;
    }

    match AParamTy::from_str(px.next_token().body_text()) {
        Ok(_) => Some(px.bump()),
        Err(_) => None,
    }
}

fn parse_end_of_preproc(px: &mut Px) {
    while !px.next().at_end_of_preproc() {
        px.skip();
    }
}

fn parse_deffunc_params(px: &mut Px) -> Vec<PParam> {
    let mut params = vec![];

    loop {
        match px.next() {
            TokenKind::Eof | TokenKind::Eos => break,
            TokenKind::Comma => {
                let comma = px.bump();

                params.push(PParam {
                    param_ty_opt: None,
                    name_opt: None,
                    comma_opt: Some(comma),
                });
            }
            TokenKind::Ident => {
                let param_ty_opt = eat_param_ty(px);
                let name_opt = px.eat(TokenKind::Ident);
                let comma_opt = px.eat(TokenKind::Comma);
                let comma_seen = comma_opt.is_some();

                params.push(PParam {
                    param_ty_opt,
                    name_opt,
                    comma_opt,
                });

                if !comma_seen {
                    break;
                }
            }
            _ => px.skip(),
        }
    }

    params
}

fn parse_deffunc_like_stmt(hash: PToken, px: &mut Px) -> PDefFuncStmt {
    assert!(DEFFUNC_LIKE_KEYWORDS.contains(&px.next_token().body_text()));

    let keyword = px.bump();

    let privacy_opt = eat_privacy(px);
    let name_opt = px.eat(TokenKind::Ident);

    let onexit_opt = eat_ident("onexit", px);
    let params = parse_deffunc_params(px);

    let mut stmts = vec![];

    loop {
        match px.next() {
            TokenKind::Eof => break,
            TokenKind::Eos | TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Colon => {
                px.skip();
            }
            TokenKind::Hash if px.nth_token(1).at_deffunc_terminator() => break,
            _ => match parse_stmt(px) {
                Some(stmt) => stmts.push(stmt),
                None => px.skip(),
            },
        }
    }

    PDefFuncStmt {
        hash,
        keyword,
        privacy_opt,
        name_opt,
        params,
        onexit_opt,
        stmts,
    }
}

fn parse_module_stmt(hash: PToken, px: &mut Px) -> PModuleStmt {
    assert_eq!(px.next_token().body_text(), "module");

    let keyword = px.bump();

    let name_opt = match px.next() {
        TokenKind::Ident | TokenKind::Str => Some(px.bump()),
        _ => None,
    };

    // FIXME: フィールド名をパース
    parse_end_of_preproc(px);

    let mut stmts = vec![];
    let global_opt = loop {
        match px.next() {
            TokenKind::Eof => break None,
            TokenKind::Eos | TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Colon => {
                px.skip();
            }
            _ => match parse_stmt(px) {
                Some(PStmt::Global(global)) => break Some(global),
                Some(stmt) => stmts.push(stmt),
                None => px.skip(),
            },
        }
    };

    PModuleStmt {
        hash,
        keyword,
        name_opt,
        stmts,
        global_opt,
    }
}

fn parse_global_stmt(hash: PToken, px: &mut Px) -> PGlobalStmt {
    assert_eq!(px.next_token().body_text(), "global");

    let keyword = px.bump();

    PGlobalStmt { hash, keyword }
}

pub(crate) fn parse_preproc_stmt(px: &mut Px) -> Option<PStmt> {
    let hash = px.eat(TokenKind::Hash)?;

    let stmt = match px.next_token().body_text() {
        "module" => PStmt::Module(parse_module_stmt(hash, px)),
        "global" => PStmt::Global(parse_global_stmt(hash, px)),
        keyword if DEFFUNC_LIKE_KEYWORDS.contains(&keyword) => {
            PStmt::DefFunc(parse_deffunc_like_stmt(hash, px))
        }
        _ => {
            parse_end_of_preproc(px);
            return None;
        }
    };

    parse_end_of_preproc(px);
    Some(stmt)
}
