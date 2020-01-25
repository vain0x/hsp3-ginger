use super::*;
use crate::syntax::*;

#[derive(Clone, Debug)]
pub(crate) struct SyntaxError {
    pub msg: String,
    pub location: SourceLocation,
}

impl SyntaxError {
    pub(crate) fn new(msg: String, location: SourceLocation) -> Self {
        Self { msg, location }
    }
}
