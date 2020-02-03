use super::*;
use std::fmt;

pub(crate) enum SyntaxElement {
    Token(SyntaxToken),
    Node(SyntaxNode),
}

impl fmt::Debug for SyntaxElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SyntaxElement::Token(x) => x.fmt(f),
            SyntaxElement::Node(x) => x.fmt(f),
        }
    }
}
