#[macro_use]
extern crate log;

pub mod c_api;

mod assists;
mod help_source;
mod lang;
mod lang_service;
mod lsp_server;
mod sem;
mod syntax;

pub use crate::lsp_server::lsp_main::start_lsp_server;

pub(crate) mod utils {
    pub(crate) mod canonical_uri;
    pub(crate) mod id;
    pub(crate) mod rc_str;
}
