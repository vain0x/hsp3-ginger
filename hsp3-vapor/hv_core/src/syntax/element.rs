use super::*;
use std::rc::Rc;

/// 構文要素
#[derive(Debug)]
pub(crate) enum Element {
    Token(TokenData),
    Trivia(Trivia),
    Error(ParseError),
    Node(Rc<NodeData>),
}

impl From<TokenData> for Element {
    fn from(token: TokenData) -> Element {
        Element::Token(token)
    }
}

impl From<ParseError> for Element {
    fn from(error: ParseError) -> Element {
        Element::Error(error)
    }
}

impl From<Trivia> for Element {
    fn from(trivia: Trivia) -> Element {
        Element::Trivia(trivia)
    }
}

impl From<NodeData> for Element {
    fn from(node: NodeData) -> Element {
        Element::Node(Rc::new(node))
    }
}
