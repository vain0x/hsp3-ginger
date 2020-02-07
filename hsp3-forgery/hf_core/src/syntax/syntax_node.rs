use super::*;
use std::fmt;
use std::rc::Rc;

/// 具象構文木のノード。
/// 位置情報と、親ノードへの参照を持つ。
#[derive(Clone)]
pub(crate) struct SyntaxNode {
    pub(crate) kind: NodeKind,
    pub(crate) parent: SyntaxParent,
    pub(crate) range: Range,
}

impl SyntaxNode {
    pub(crate) fn from_root(root: Rc<SyntaxRoot>) -> SyntaxNode {
        let range = root.range();

        SyntaxNode {
            kind: NodeKind::Root,
            parent: SyntaxParent::Root { root },
            range,
        }
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

    pub(crate) fn child_elements(&self) -> impl DoubleEndedIterator<Item = SyntaxElement> {
        let it = Rc::new(self.clone());
        let mut start = self.range.start;

        // この move は self の所有権をクロージャに渡す。
        (0..self.green().children().len()).map(move |child_index| {
            // self → SyntaxRoot → GreenNode (Root) → self.green() のように、
            // 間接的にイミュータブルな参照を握っているので child_index が無効になることはない。
            // そのため unwrap は失敗しないはず。
            match it.green().children().get(child_index).unwrap() {
                GreenElement::Node(node) => {
                    let end = start + node.position();
                    let range = Range::new(start, end);
                    start = end;

                    SyntaxElement::Node(SyntaxNode {
                        kind: node.kind(),
                        parent: SyntaxParent::NonRoot {
                            node: Rc::clone(&it),
                            child_index,
                        },
                        range,
                    })
                }
                GreenElement::Token(token) => {
                    let end = start + token.position();
                    let range = Range::new(start, end);
                    start = end;

                    SyntaxElement::Token(SyntaxToken {
                        kind: token.token(),
                        parent: SyntaxParent::NonRoot {
                            node: Rc::clone(&it),
                            child_index,
                        },
                        location: Location::new(token.location.source.clone(), range),
                    })
                }
            }
        })
    }

    pub(crate) fn child_nodes(&self) -> impl Iterator<Item = SyntaxNode> {
        self.child_elements().filter_map(|element| match element {
            SyntaxElement::Node(node) => Some(node),
            SyntaxElement::Token(..) => None,
        })
    }

    pub(crate) fn child_tokens(&self) -> impl Iterator<Item = SyntaxToken> {
        self.child_elements().filter_map(|element| match element {
            SyntaxElement::Token(token) => Some(token),
            SyntaxElement::Node(..) => None,
        })
    }

    pub(crate) fn descendant_elements(&self) -> impl Iterator<Item = SyntaxElement> {
        iter::DescendantElementsIter::new(self.clone().into())
    }
}

impl fmt::Debug for SyntaxNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}({:?})", self.kind(), self.range())?;
        self.child_elements().collect::<Vec<_>>().fmt(f)
    }
}

mod iter {
    use super::*;

    pub(super) struct DescendantElementsIter {
        stack: Vec<SyntaxElement>,
    }

    impl DescendantElementsIter {
        pub(super) fn new(element: SyntaxElement) -> Self {
            DescendantElementsIter {
                stack: vec![element],
            }
        }
    }

    impl Iterator for DescendantElementsIter {
        type Item = SyntaxElement;

        fn next(&mut self) -> Option<SyntaxElement> {
            let element = self.stack.pop()?;
            match &element {
                SyntaxElement::Token(_) => {}
                SyntaxElement::Node(node) => {
                    self.stack.extend(node.child_elements().rev());
                }
            }
            Some(element)
        }
    }
}
