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

mod utils {
    pub(crate) mod canonical_uri;
    pub(crate) mod id;
    pub(crate) mod rc_str;
}
