#[macro_use]
extern crate log;

pub mod c_api;

mod assists;
mod help_source;
mod lang;
mod lang_service;
mod lsp_server;
mod tests;

pub use crate::lsp_server::lsp_main::start_lsp_server;

mod analysis {
    pub(crate) mod a_scope;
    pub(crate) mod a_symbol;
    pub(crate) mod analyze;
    pub(crate) mod comment;
    pub(crate) mod integrate;
    pub(crate) mod preproc;
    pub(crate) mod syntax_linter;
    pub(crate) mod var;

    pub(crate) use self::{
        a_scope::AScope,
        a_symbol::{ASymbol, ASymbolDetails, ASymbolKind},
    };
}

mod parse {
    //! 構文木・構文解析

    #![allow(dead_code)]

    pub(crate) mod p_const_ty;
    pub(crate) mod p_jump_modifier;
    pub(crate) mod p_op_kind;
    pub(crate) mod p_param_ty;
    pub(crate) mod p_privacy;
    pub(crate) mod p_token;
    pub(crate) mod p_tree;
    pub(crate) mod parse_context;
    pub(crate) mod parse_expr;
    pub(crate) mod parse_preproc;
    pub(crate) mod parse_stmt;

    mod parse_tests;

    pub(crate) use p_const_ty::PConstTy;
    pub(crate) use p_jump_modifier::PJumpModifier;
    pub(crate) use p_param_ty::PParamTy;
    pub(crate) use p_privacy::PPrivacy;
    pub(crate) use p_token::PToken;
    pub(crate) use p_tree::*;

    pub(crate) use parse_stmt::parse_root;
}

mod source {
    //! ソースファイルの位置情報など

    mod loc;
    mod pos;
    mod range;

    pub(crate) use loc::*;
    pub(crate) use pos::*;
    pub(crate) use range::*;

    pub(crate) type ADoc = usize;
}

mod token {
    //! 字句・字句解析

    pub(crate) mod token_data;
    pub(crate) mod token_kind;
    pub(crate) mod tokenize_context;
    pub(crate) mod tokenize_rules;
    pub(crate) mod tokenize_tests;

    pub(crate) use token_data::TokenData;
    pub(crate) use token_kind::TokenKind;
    pub(crate) use tokenize_rules::tokenize;
}

mod utils {
    pub(crate) mod canonical_uri;
    pub(crate) mod id;
    pub(crate) mod rc_item;
    pub(crate) mod rc_slice;
    pub(crate) mod rc_str;
    pub(crate) mod read_file;
}
