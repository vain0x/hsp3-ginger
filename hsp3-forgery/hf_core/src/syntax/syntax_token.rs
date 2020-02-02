use super::*;

pub(crate) struct SyntaxToken {
    pub(crate) kind: Token,
    pub(crate) parent: SyntaxParent,
    pub(crate) location: Location,
}
