use super::*;

pub(crate) struct SyntaxNode {
    pub(crate) kind: NodeKind,
    pub(crate) parent: SyntaxParent,
    pub(crate) range: Range,
}
