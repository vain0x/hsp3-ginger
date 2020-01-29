use super::*;
use crate::syntax::*;

#[derive(Clone, Debug)]
pub(crate) struct SyntaxError {
    pub(crate) msg: String,
    pub(crate) location: SourceLocation,
}

impl SyntaxError {
    pub(crate) fn new(msg: String, location: SourceLocation) -> Self {
        Self { msg, location }
    }
}
