use super::*;
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
                match &node.green().children.get(*child_index) {
                    Some(GreenElement::Node(node)) => node,
                    Some(GreenElement::Token(..)) | None => {
                        unreachable!("SyntaxParent::NonRoot bug")
                    }
                }
            }
        }
    }

    pub(crate) fn child_elements(&self) -> impl Iterator<Item = &GreenElement> {
        self.green().children().iter()
    }

    pub(crate) fn child_nodes(self: Rc<Self>) -> impl Iterator<Item = Rc<SyntaxNode>> {
        // この move は self の所有権をクロージャに渡す。
        (0..self.green().children().len()).filter_map(move |child_index| {
            // self → SyntaxRoot → GreenNode (Root) → self.green() のように、
            // 間接的にイミュータブルな参照を握っているので child_index が無効になることはない。
            // そのため unwrap は失敗しないはず。
            match self.green().children().get(child_index).unwrap() {
                GreenElement::Node(node) => Some(Rc::new(SyntaxNode {
                    kind: node.kind(),
                    parent: SyntaxParent::NonRoot {
                        node: Rc::clone(&self),
                        child_index,
                    },
                    // FIXME: オフセットを計算する。
                    offset: 0,
                })),
                GreenElement::Token(..) => None,
            }
        })
    }

    pub(crate) fn child_tokens(self: Rc<Self>) -> impl Iterator<Item = Rc<SyntaxToken>> {
        // 上に同じ。
        (0..self.green().children().len()).filter_map(move |child_index| {
            match self.green().children().get(child_index).unwrap() {
                GreenElement::Token(token) => Some(Rc::new(SyntaxToken {
                    kind: token.token(),
                    parent: SyntaxParent::NonRoot {
                        node: Rc::clone(&self),
                        child_index,
                    },
                    location: token.location.clone(),
                })),
                GreenElement::Node(..) => None,
            }
        })
    }
}
