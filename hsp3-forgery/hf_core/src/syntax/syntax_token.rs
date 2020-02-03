use super::*;

pub(crate) struct SyntaxToken {
    pub(crate) kind: Token,
    pub(crate) parent: SyntaxParent,
    pub(crate) location: Location,
}

impl SyntaxToken {
    pub(crate) fn kind(&self) -> Token {
        self.kind
    }

    pub(crate) fn text(&self) -> &str {
        self.green().text()
    }

    pub(crate) fn green(&self) -> &TokenData {
        match &self.parent {
            SyntaxParent::Root { .. } => unreachable!("SyntaxParent::Root bug"),
            SyntaxParent::NonRoot { node, child_index } => {
                match &node.green().children.get(*child_index) {
                    Some(GreenElement::Token(token)) => token,
                    Some(GreenElement::Node(..)) | None => {
                        unreachable!("SyntaxParent::NonRoot bug")
                    }
                }
            }
        }
    }
}
