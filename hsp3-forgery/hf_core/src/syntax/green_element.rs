use super::*;

pub(crate) enum GreenElement {
    Token(TokenData),
    Node(GreenNode),
}
