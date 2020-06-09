#[macro_use]
extern crate log;

mod canonical_uri;
mod docs;
mod help_source;
mod id;
mod lsp;
mod rc_str;
mod sem;
mod syntax;

pub use crate::lsp::lsp_main::start_lsp_server;
