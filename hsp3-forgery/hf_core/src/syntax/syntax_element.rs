use super::*;
use std::fmt;

/// 具象構文木の要素。
pub(crate) enum SyntaxElement {
    Token(SyntaxToken),
    Node(SyntaxNode),
}

impl SyntaxElement {
    pub(crate) fn cast_token(self) -> Option<SyntaxToken> {
        match self {
            SyntaxElement::Token(token) => Some(token),
            SyntaxElement::Node(_) => None,
        }
    }

    pub(crate) fn cast_node(self) -> Option<SyntaxNode> {
        match self {
            SyntaxElement::Token(_) => None,
            SyntaxElement::Node(node) => Some(node),
        }
    }

    pub(crate) fn range(&self) -> Range {
        match self {
            SyntaxElement::Token(token) => token.range(),
            SyntaxElement::Node(node) => node.range(),
        }
    }
}

impl From<SyntaxToken> for SyntaxElement {
    fn from(token: SyntaxToken) -> SyntaxElement {
        SyntaxElement::Token(token)
    }
}

impl From<SyntaxNode> for SyntaxElement {
    fn from(node: SyntaxNode) -> SyntaxElement {
        SyntaxElement::Node(node)
    }
}

impl fmt::Debug for SyntaxElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SyntaxElement::Token(x) => x.fmt(f),
            SyntaxElement::Node(x) => x.fmt(f),
        }
    }
}
