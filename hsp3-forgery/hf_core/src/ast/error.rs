use super::*;

#[derive(Clone, Debug)]
pub(crate) struct SyntaxError {
    pub(crate) msg: String,
    pub(crate) location: Location,
}

impl SyntaxError {
    pub(crate) fn new(msg: String, location: Location) -> Self {
        Self { msg, location }
    }
}
