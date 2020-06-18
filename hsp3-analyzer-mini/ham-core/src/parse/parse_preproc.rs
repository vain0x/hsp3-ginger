use super::{
    p_token::PToken,
    parse_context::Px,
    parse_expr::{parse_args, parse_expr},
    parse_stmt::parse_stmt,
    PCmdStmt, PConstStmt, PDefFuncStmt, PDefineStmt, PEnumStmt, PGlobalStmt, PIncludeStmt,
    PLibFuncStmt, PMacroParam, PModuleStmt, PParam, PParamTy, PPrivacy, PRegCmdStmt, PStmt,
    PUnknownPreProcStmt, PUseLibStmt,
};
use crate::token::TokenKind;

static DEFFUNC_LIKE_KEYWORDS: &[&str] = &[
    "deffunc", "defcfunc", "modfunc", "modcfunc", "modinit", "modterm",
];

impl TokenKind {
    fn is_end_of_preproc(self) -> bool {
        match self {
            TokenKind::Eof | TokenKind::Eos => true,
            _ => false,
        }
    }
}

impl PToken {
    /// `deffunc` 系の命令の領域を分割するプリプロセッサ命令の名前
    fn is_deffunc_terminator(&self) -> bool {
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

fn parse_privacy(px: &mut Px) -> Option<(PPrivacy, PToken)> {
    if px.next() != TokenKind::Ident {
        return None;
    }

    match px.next_token().body_text().parse::<PPrivacy>() {
        Ok(privacy) => {
            let token = px.bump();
            Some((privacy, token))
        }
        Err(()) => None,
    }
}

fn parse_param_ty(px: &mut Px) -> Option<(PParamTy, PToken)> {
    if px.next() != TokenKind::Ident {
        return None;
    }

    match px.next_token().body_text().parse::<PParamTy>() {
        Ok(param_ty) => {
            let token = px.bump();
            Some((param_ty, token))
        }
        Err(_) => None,
    }
}

fn parse_end_of_preproc(px: &mut Px) {
    while !px.next().is_end_of_preproc() {
        px.skip()
    }
}

fn parse_const_ty(px: &mut Px) -> Option<PToken> {
    if px.next() == TokenKind::Ident && ["double", "int"].contains(&px.next_token().body_text()) {
        Some(px.bump())
    } else {
        None
    }
}

fn parse_const_stmt(hash: PToken, px: &mut Px) -> PConstStmt {
    assert_eq!(px.next_token().body_text(), "const");
    let keyword = px.bump();

    let privacy_opt = parse_privacy(px);
    let ty_opt = parse_const_ty(px);
    let name_opt = px.eat(TokenKind::Ident);
    let init_opt = parse_expr(px);
    parse_end_of_preproc(px);

    PConstStmt {
        hash,
        keyword,
        privacy_opt,
        ty_opt,
        name_opt,
        init_opt,
    }
}

fn parse_enum_stmt(hash: PToken, px: &mut Px) -> PEnumStmt {
    assert_eq!(px.next_token().body_text(), "enum");
    let keyword = px.bump();

    let privacy_opt = parse_privacy(px);
    let name_opt = px.eat(TokenKind::Ident);

    let equal_opt = px.eat(TokenKind::Equal);
    let init_opt = parse_expr(px);
    parse_end_of_preproc(px);

    PEnumStmt {
        hash,
        keyword,
        privacy_opt,
        name_opt,
        equal_opt,
        init_opt,
    }
}

fn parse_macro_params(px: &mut Px) -> Vec<PMacroParam> {
    let mut params = vec![];
    let mut init = vec![];

    loop {
        if let TokenKind::Eof | TokenKind::Eos | TokenKind::RightParen = px.next() {
            break;
        }

        let percent_opt = px.eat(TokenKind::Percent);
        let number_opt = px.eat(TokenKind::Number);

        // 既定値
        let equal_opt = px.eat(TokenKind::Equal);
        if equal_opt.is_some() {
            init.extend(px.eat(TokenKind::Percent));
            match px.next() {
                TokenKind::Eof
                | TokenKind::Eos
                | TokenKind::LeftParen
                | TokenKind::RightParen
                | TokenKind::Comma => {}
                _ => init.push(px.bump()),
            };
        }

        let comma_opt = px.eat(TokenKind::Comma);
        let comma_seen = comma_opt.is_some();

        params.push(PMacroParam {
            percent_opt,
            number_opt,
            equal_opt,
            init: init.split_off(0),
            comma_opt,
        });
        if !comma_seen {
            break;
        }
    }

    params
}

fn eat_arbitrary_tokens(px: &mut Px) -> Vec<PToken> {
    let mut tokens = vec![];
    while !px.next().is_end_of_preproc() {
        tokens.push(px.bump());
    }
    tokens
}

fn parse_define_stmt(hash: PToken, px: &mut Px) -> PDefineStmt {
    assert_eq!(px.next_token().body_text(), "define");
    let keyword = px.bump();

    let privacy_opt = parse_privacy(px);
    let ctype_opt = eat_ident("ctype", px);

    let name_opt = px.eat(TokenKind::Ident);
    let has_params = {
        // マクロ名と `(` の間にスペースがないときだけパラメータリストとみなす。
        px.next() == TokenKind::LeftParen
            && name_opt.as_ref().map_or(false, |name| {
                name.body.loc.end() == px.next_token().body.loc.start()
            })
    };
    let (left_paren_opt, params, right_paren_opt) = if has_params {
        let left_paren_opt = px.eat(TokenKind::LeftParen);
        let params = parse_macro_params(px);
        let right_paren_opt = px.eat(TokenKind::RightParen);
        (left_paren_opt, params, right_paren_opt)
    } else {
        (None, vec![], None)
    };

    let tokens = eat_arbitrary_tokens(px);

    PDefineStmt {
        hash,
        keyword,
        privacy_opt,
        ctype_opt,
        name_opt,
        left_paren_opt,
        params,
        right_paren_opt,
        tokens,
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
                let param_ty_opt = parse_param_ty(px);
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

    let privacy_opt = parse_privacy(px);
    let name_opt = px.eat(TokenKind::Ident);

    let onexit_opt = eat_ident("onexit", px);
    let params = parse_deffunc_params(px);
    parse_end_of_preproc(px);

    let mut stmts = vec![];
    loop {
        match px.next() {
            TokenKind::Eof => break,
            TokenKind::Eos | TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Colon => {
                px.skip();
            }
            TokenKind::Hash if px.nth_token(1).is_deffunc_terminator() => break,
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

fn parse_uselib_stmt(hash: PToken, px: &mut Px) -> PUseLibStmt {
    assert_eq!(px.next_token().body_text(), "uselib");

    let keyword = px.bump();
    let file_path_opt = px.eat(TokenKind::Str);
    parse_end_of_preproc(px);

    PUseLibStmt {
        hash,
        keyword,
        file_path_opt,
    }
}

fn parse_lib_func_stmt(hash: PToken, px: &mut Px) -> PLibFuncStmt {
    let keyword = px.bump();

    let privacy_opt = parse_privacy(px);
    let name_opt = px.eat(TokenKind::Ident);
    let onexit_opt = eat_ident("onexit", px);

    let func_name_opt = match px.next() {
        TokenKind::Ident | TokenKind::Str => Some(px.bump()),
        _ => None,
    };
    let type_id_opt = px.eat(TokenKind::Number);
    let params = parse_deffunc_params(px);
    parse_end_of_preproc(px);

    PLibFuncStmt {
        hash,
        keyword,
        privacy_opt,
        name_opt,
        onexit_opt,
        func_name_opt,
        type_id_opt,
        params,
    }
}

fn parse_regcmd_stmt(hash: PToken, px: &mut Px) -> PRegCmdStmt {
    assert_eq!(px.next_token().body_text(), "regcmd");

    let keyword = px.bump();
    let args = parse_args(px);
    parse_end_of_preproc(px);

    PRegCmdStmt {
        hash,
        keyword,
        args,
    }
}

fn parse_cmd_stmt(hash: PToken, px: &mut Px) -> PCmdStmt {
    assert_eq!(px.next_token().body_text(), "cmd");

    let keyword = px.bump();
    let privacy_opt = parse_privacy(px);
    let name_opt = px.eat(TokenKind::Ident);
    let command_id_opt = px.eat(TokenKind::Number);
    parse_end_of_preproc(px);

    PCmdStmt {
        hash,
        keyword,
        privacy_opt,
        name_opt,
        command_id_opt,
    }
}

fn parse_module_stmt(hash: PToken, px: &mut Px) -> PModuleStmt {
    assert_eq!(px.next_token().body_text(), "module");

    let keyword = px.bump();

    let name_opt = match px.next() {
        TokenKind::Ident | TokenKind::Str => Some(px.bump()),
        _ => None,
    };

    let fields = parse_deffunc_params(px);
    parse_end_of_preproc(px);

    let mut stmts = vec![];
    let global_opt = loop {
        match px.next() {
            TokenKind::Eof => break None,
            TokenKind::Eos | TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Colon => {
                px.skip();
            }
            TokenKind::Hash if px.nth_token(1).body_text() == "module" => break None,
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
        fields,
        stmts,
        global_opt,
    }
}

fn parse_global_stmt(hash: PToken, px: &mut Px) -> PGlobalStmt {
    assert_eq!(px.next_token().body_text(), "global");

    let keyword = px.bump();
    parse_end_of_preproc(px);

    PGlobalStmt { hash, keyword }
}

fn parse_include_stmt(hash: PToken, is_optional: bool, px: &mut Px) -> PIncludeStmt {
    let keyword = px.bump();
    let file_path_opt = px.eat(TokenKind::Str);
    parse_end_of_preproc(px);

    PIncludeStmt {
        hash,
        keyword,
        file_path_opt,
        is_optional,
    }
}

pub(crate) fn parse_preproc_stmt(px: &mut Px) -> Option<PStmt> {
    let hash = px.eat(TokenKind::Hash)?;

    let stmt = match px.next_token().body_text() {
        "const" => PStmt::Const(parse_const_stmt(hash, px)),
        "enum" => PStmt::Enum(parse_enum_stmt(hash, px)),
        "define" => PStmt::Define(parse_define_stmt(hash, px)),
        "uselib" => PStmt::UseLib(parse_uselib_stmt(hash, px)),
        "func" | "cfunc" => PStmt::LibFunc(parse_lib_func_stmt(hash, px)),
        "regcmd" => PStmt::RegCmd(parse_regcmd_stmt(hash, px)),
        "cmd" => PStmt::Cmd(parse_cmd_stmt(hash, px)),
        "module" => PStmt::Module(parse_module_stmt(hash, px)),
        "global" => PStmt::Global(parse_global_stmt(hash, px)),
        "include" => PStmt::Include(parse_include_stmt(hash, false, px)),
        "addition" => PStmt::Include(parse_include_stmt(hash, true, px)),
        keyword if DEFFUNC_LIKE_KEYWORDS.contains(&keyword) => {
            PStmt::DefFunc(parse_deffunc_like_stmt(hash, px))
        }
        _ => {
            let tokens = eat_arbitrary_tokens(px);
            PStmt::UnknownPreProc(PUnknownPreProcStmt { hash, tokens })
        }
    };
    Some(stmt)
}
