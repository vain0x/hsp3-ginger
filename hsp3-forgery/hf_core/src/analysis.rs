pub(crate) mod completion;
pub(crate) mod diagnostic;
pub(crate) mod get_global_symbols;
pub(crate) mod get_signature_help;
pub(crate) mod get_syntax_errors;
pub(crate) mod goto_definition;
pub(crate) mod name_resolution;
pub(crate) mod types;

pub(crate) use diagnostic::*;
pub(crate) use types::*;

use crate::ast::*;
use crate::source::{Position, Range};
use crate::syntax::*;
use crate::token::{Location, Token};
