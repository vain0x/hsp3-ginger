use super::*;
use std::fmt;

/// 永続構文木のノード。
pub(crate) enum GreenElement {
    Token(TokenData),
    Node(GreenNode),
}

impl From<TokenData> for GreenElement {
    fn from(token: TokenData) -> Self {
        GreenElement::Token(token)
    }
}

impl From<GreenNode> for GreenElement {
    fn from(node: GreenNode) -> Self {
        GreenElement::Node(node)
    }
}

impl GreenElement {
    pub(crate) fn position(&self) -> Position {
        match self {
            GreenElement::Token(token) => token.position(),
            GreenElement::Node(node) => node.position(),
        }
    }
}

impl fmt::Debug for GreenElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GreenElement::Token(token) => token.fmt(f),
            GreenElement::Node(node) => node.fmt(f),
        }
    }
}
