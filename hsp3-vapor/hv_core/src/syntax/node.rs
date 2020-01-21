use super::*;
use std::fmt::{self, Debug};

///　構文ノードの種類
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Node {
    Name,
    NumberLiteral,
    Group,
    Call,
    Argument,
    ExprStmt,
    MatchStmt,
    MatchArm,
    EnumDecl,
    /// K or K(...)
    CtorDecl,
    /// <コンストラクタ> "(" <タプルフィールド>,* ")"
    TupleDecl,
    TupleFieldDecl,
    Root,
    NotSpecified,
}

/// 構文ノードのデータ
pub(crate) struct NodeData {
    node: Node,
    children: Vec<Element>,
}

impl NodeData {
    pub(crate) fn new() -> Self {
        NodeData {
            node: Node::NotSpecified,
            children: vec![],
        }
    }

    pub(crate) fn new_before(child: NodeData) -> Self {
        let mut parent = NodeData::new();
        parent.push_node(child);
        parent
    }

    pub(crate) fn node(&self) -> Node {
        self.node
    }

    pub(crate) fn set_node(mut self, node: Node) -> Self {
        assert_eq!(self.node, Node::NotSpecified);
        assert_ne!(node, Node::NotSpecified);

        self.node = node;
        self
    }

    pub(crate) fn children(&self) -> &[Element] {
        &self.children
    }

    pub(crate) fn push_token(&mut self, token: FatToken) {
        let (leading, token, trailing) = token.into_slim();

        for trivia in leading {
            self.children.push(trivia.into());
        }

        self.children.push(token.into());

        for trivia in trailing {
            self.children.push(trivia.into());
        }
    }

    pub(crate) fn push_error(&mut self, error: ParseError) {
        self.children.push(error.into())
    }

    pub(crate) fn push_node(&mut self, node: NodeData) {
        assert_ne!(node.node(), Node::NotSpecified);

        self.children.push(node.into())
    }
}

// AST のダンプ時に邪魔にならないようにする。
impl Debug for NodeData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NodeData(..)")
    }
}
