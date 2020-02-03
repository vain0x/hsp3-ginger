use super::*;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeKind {
    Other,
    LabelLiteral,
    StrLiteral,
    DoubleLiteral,
    IntLiteral,
    Ident,
    SystemVar,
    Param,
    Arg,
    CallExpr,
    BinaryExpr,
    AssignStmt,
    CommandStmt,
    LabelStmt,
    DeffuncPp,
    ModulePp,
    GlobalPp,
    UnknownPp,
    Root,
}

pub(crate) struct GreenNode {
    kind: NodeKind,
    children: Vec<GreenElement>,
    position: Position,
}

impl GreenNode {
    pub(crate) fn new() -> Self {
        GreenNode {
            kind: NodeKind::Other,
            children: vec![],
            position: Position::default(),
        }
    }

    pub(crate) fn kind(&self) -> NodeKind {
        self.kind
    }

    pub(crate) fn children(&self) -> &[GreenElement] {
        &self.children
    }

    pub(crate) fn position(&self) -> Position {
        self.position
    }

    pub(crate) fn is_frozen(&self) -> bool {
        self.kind() != NodeKind::Other
    }

    pub(crate) fn set_kind(&mut self, kind: NodeKind) {
        assert!(!self.is_frozen());

        self.kind = kind;
        self.position = self
            .children()
            .iter()
            .map(GreenElement::position)
            .sum::<Position>();
    }

    pub(crate) fn push_token(&mut self, token: TokenData) {
        assert!(!self.is_frozen());
        self.children.push(GreenElement::Token(token))
    }

    pub(crate) fn push_node(&mut self, node: GreenNode) {
        assert!(!self.is_frozen());
        self.children.push(GreenElement::Node(node))
    }

    pub(crate) fn drain_last_node_from(&mut self, other: &mut GreenNode) {
        assert!(!self.is_frozen());

        // 最後のノードの位置を計算する。
        let mut i = other.children().len();
        while i >= 1 {
            i -= 1;
            match other.children()[i] {
                GreenElement::Node(..) => break,
                GreenElement::Token(..) => continue,
            }
        }

        self.children.extend(other.children.drain(i..));
    }
}

impl fmt::Debug for GreenNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.kind)?;
        self.children().fmt(f)
    }
}
