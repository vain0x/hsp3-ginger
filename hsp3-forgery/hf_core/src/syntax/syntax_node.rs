use super::*;
use std::fmt;
use std::rc::Rc;

pub(crate) struct SyntaxNode {
    pub(crate) kind: NodeKind,
    pub(crate) parent: SyntaxParent,
    pub(crate) offset: usize,
}

impl SyntaxNode {
    pub(crate) fn from_root(root: Rc<SyntaxRoot>) -> Rc<SyntaxNode> {
        Rc::new(SyntaxNode {
            kind: NodeKind::Root,
            parent: SyntaxParent::Root { root },
            offset: 0,
        })
    }

    pub(crate) fn kind(&self) -> NodeKind {
        self.kind
    }

    pub(crate) fn green(&self) -> &GreenNode {
        match &self.parent {
            SyntaxParent::Root { root } => root.green(),
            SyntaxParent::NonRoot { node, child_index } => {
                match &node.green().children().get(*child_index) {
                    Some(GreenElement::Node(node)) => node,
                    Some(GreenElement::Token(..)) | None => {
                        unreachable!("SyntaxParent::NonRoot bug")
                    }
                }
            }
        }
    }

    pub(crate) fn child_elements(self: Rc<Self>) -> impl Iterator<Item = SyntaxElement> {
        // この move は self の所有権をクロージャに渡す。
        (0..self.green().children().len()).filter_map(move |child_index| {
            // self → SyntaxRoot → GreenNode (Root) → self.green() のように、
            // 間接的にイミュータブルな参照を握っているので child_index が無効になることはない。
            // そのため unwrap は失敗しないはず。
            match self.green().children().get(child_index).unwrap() {
                GreenElement::Node(node) => Some(
                    (SyntaxElement::Node(SyntaxNode {
                        kind: node.kind(),
                        parent: SyntaxParent::NonRoot {
                            node: Rc::clone(&self),
                            child_index,
                        },
                        // FIXME: オフセットを計算する。
                        offset: 0,
                    })),
                ),
                GreenElement::Token(token) => Some(
                    (SyntaxElement::Token(SyntaxToken {
                        kind: token.token(),
                        parent: SyntaxParent::NonRoot {
                            node: Rc::clone(&self),
                            child_index,
                        },
                        location: token.location.clone(),
                    })),
                ),
            }
        })
    }

    pub(crate) fn child_nodes(self: Rc<Self>) -> impl Iterator<Item = Rc<SyntaxNode>> {
        self.child_elements().filter_map(|element| match element {
            SyntaxElement::Node(node) => Some(Rc::new(node)),
            SyntaxElement::Token(..) => None,
        })
    }

    pub(crate) fn child_tokens(self: Rc<Self>) -> impl Iterator<Item = Rc<SyntaxToken>> {
        self.child_elements().filter_map(|element| match element {
            SyntaxElement::Token(token) => Some(Rc::new(token)),
            SyntaxElement::Node(..) => None,
        })
    }
}
