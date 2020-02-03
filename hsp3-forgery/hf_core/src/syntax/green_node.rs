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
    pub(crate) kind: NodeKind,
    pub(crate) children: Vec<GreenElement>,
}

impl GreenNode {
    pub(crate) fn new(kind: NodeKind) -> Self {
        GreenNode {
            kind,
            children: vec![],
        }
    }

    pub(crate) fn new_dummy() -> Self {
        GreenNode {
            kind: NodeKind::Other,
            children: vec![],
        }
    }

    pub(crate) fn new_root() -> Self {
        GreenNode {
            kind: NodeKind::Root,
            children: vec![],
        }
    }

    pub(crate) fn set_kind(&mut self, kind: NodeKind) {
        assert_eq!(self.kind, NodeKind::Other);

        self.kind = kind;
    }

    pub(crate) fn push_token(&mut self, token: TokenData) {
        self.children.push(GreenElement::Token(token))
    }

    pub(crate) fn push_node(&mut self, node: GreenNode) {
        self.children.push(GreenElement::Node(node))
    }
}

impl fmt::Debug for GreenNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.kind)?;
        self.children.fmt(f)
    }
}
