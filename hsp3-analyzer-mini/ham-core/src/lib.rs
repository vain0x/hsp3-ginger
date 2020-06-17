#[macro_use]
extern crate log;

pub mod c_api;

mod assists;
mod help_source;
mod lang;
mod lang_service;
mod lsp_server;
mod sem;

pub use crate::lsp_server::lsp_main::start_lsp_server;

mod analysis {
    pub(crate) mod a_doc;
    pub(crate) mod a_loc;
    pub(crate) mod a_pos;
    pub(crate) mod a_range;
    pub(crate) mod a_scope;
    pub(crate) mod a_symbol;

    mod analysis_tests;

    #[allow(unused)]
    pub(crate) use self::{
        a_doc::ADoc,
        a_loc::ALoc,
        a_pos::APos,
        a_range::ARange,
        a_scope::AScope,
        a_symbol::{ASymbol, ASymbolDetails, ASymbolKind},
    };
}

mod parse {
    //! 構文木・構文解析

    pub(crate) mod p_token;

    pub(crate) use p_token::PToken;
}

mod token {
    //! 字句・字句解析

    #![allow(unused)]

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
    pub(crate) mod rc_str;
}
