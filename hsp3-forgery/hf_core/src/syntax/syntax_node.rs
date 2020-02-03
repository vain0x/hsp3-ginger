use super::*;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct SyntaxNode {
    pub(crate) kind: NodeKind,
    pub(crate) parent: SyntaxParent,
    pub(crate) range: Range,
}

impl SyntaxNode {
    pub(crate) fn from_root(root: Rc<SyntaxRoot>) -> Rc<SyntaxNode> {
        let range = root.range();

        Rc::new(SyntaxNode {
            kind: NodeKind::Root,
            parent: SyntaxParent::Root { root },
            range,
        })
    }

    pub(crate) fn kind(&self) -> NodeKind {
        self.kind
    }

    pub(crate) fn range(&self) -> Range {
        self.range
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
        let mut start = self.range.start;

        // この move は self の所有権をクロージャに渡す。
        (0..self.green().children().len()).filter_map(move |child_index| {
            // self → SyntaxRoot → GreenNode (Root) → self.green() のように、
            // 間接的にイミュータブルな参照を握っているので child_index が無効になることはない。
            // そのため unwrap は失敗しないはず。
            match self.green().children().get(child_index).unwrap() {
                GreenElement::Node(node) => {
                    let end = start + node.position();
                    let range = Range::new(start, end);
                    start = end;

                    Some(SyntaxElement::Node(SyntaxNode {
                        kind: node.kind(),
                        parent: SyntaxParent::NonRoot {
                            node: Rc::clone(&self),
                            child_index,
                        },
                        range,
                    }))
                }
                GreenElement::Token(token) => {
                    let end = start + token.position();
                    let range = Range::new(start, end);
                    start = end;

                    Some(SyntaxElement::Token(SyntaxToken {
                        kind: token.token(),
                        parent: SyntaxParent::NonRoot {
                            node: Rc::clone(&self),
                            child_index,
                        },
                        location: Location::new(token.location.source.clone(), range),
                    }))
                }
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

impl fmt::Debug for SyntaxNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}({:?})", self.kind(), self.range())?;
        Rc::new(self.clone())
            .child_elements()
            .collect::<Vec<_>>()
            .fmt(f)
    }
}
