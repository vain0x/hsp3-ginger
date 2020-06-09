#[macro_use]
extern crate log;

mod assists;
mod canonical_uri;
mod help_source;
mod id;
mod lang_service;
mod lsp_server;
mod rc_str;
mod sem;
mod syntax;

pub use crate::lsp_server::lsp_main::start_lsp_server;
