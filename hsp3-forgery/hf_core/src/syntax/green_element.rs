use super::*;
use std::fmt;

pub(crate) enum GreenElement {
    Token(TokenData),
    Node(GreenNode),
}

impl fmt::Debug for GreenElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GreenElement::Token(token) => token.fmt(f),
            GreenElement::Node(node) => node.fmt(f),
        }
    }
}
