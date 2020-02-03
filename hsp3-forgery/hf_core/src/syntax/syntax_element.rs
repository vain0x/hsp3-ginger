use super::*;
use std::rc::Rc;

pub(crate) enum SyntaxElement {
    Token(SyntaxToken),
    Node(SyntaxNode),
}
