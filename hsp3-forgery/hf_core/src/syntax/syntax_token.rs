use super::*;
use std::fmt;

/// 具象構文木のリーフノード。
/// 位置情報と、親ノードへの参照を持つ。
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

    pub(crate) fn location(&self) -> &Location {
        &self.location
    }

    pub(crate) fn range(&self) -> Range {
        self.location().range()
    }

    pub(crate) fn green(&self) -> &TokenData {
        match &self.parent {
            SyntaxParent::Root { .. } => unreachable!("SyntaxParent::Root bug"),
            SyntaxParent::NonRoot { node, child_index } => {
                match &node.green().children().get(*child_index) {
                    Some(GreenElement::Token(token)) => token,
                    Some(GreenElement::Node(..)) | None => {
                        unreachable!("SyntaxParent::NonRoot bug")
                    }
                }
            }
        }
    }
}

impl fmt::Debug for SyntaxToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}({}) {:?}",
            self.kind(),
            self.location().range(),
            self.text()
        )
    }
}
