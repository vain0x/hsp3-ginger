use super::*;
use std::fmt;
use std::hash::{Hash, Hasher};
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

    pub(crate) fn source(&self) -> &TokenSource {
        self.syntax_root().source()
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

    pub(crate) fn parent_node(&self) -> Option<&SyntaxNode> {
        match &self.parent {
            SyntaxParent::NonRoot { node, .. } => Some(node),
            SyntaxParent::Root { .. } => None,
        }
    }

    pub(crate) fn syntax_root(&self) -> &SyntaxRoot {
        match &self.parent {
            SyntaxParent::Root { root } => root,
            SyntaxParent::NonRoot { node, .. } => node.syntax_root(),
        }
    }

    pub(crate) fn child_elements(&self) -> impl Iterator<Item = SyntaxElement> {
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
                        location: Location::new(token.source().clone(), range),
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

    pub(crate) fn descendant_nodes(&self) -> impl Iterator<Item = SyntaxNode> {
        self.descendant_elements()
            .filter_map(|element| match element {
                SyntaxElement::Node(node) => Some(node),
                SyntaxElement::Token(..) => None,
            })
    }

    pub(crate) fn descendant_tokens(&self) -> impl Iterator<Item = SyntaxToken> {
        self.descendant_elements()
            .filter_map(|element| match element {
                SyntaxElement::Token(token) => Some(token),
                SyntaxElement::Node(..) => None,
            })
    }
}

impl fmt::Debug for SyntaxNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}({:?})", self.kind(), self.range())?;
        self.child_elements().collect::<Vec<_>>().fmt(f)
    }
}

impl PartialEq for SyntaxNode {
    fn eq(&self, other: &SyntaxNode) -> bool {
        self.kind == other.kind && self.range == other.range && self.source() == other.source()
    }
}

impl Eq for SyntaxNode {}

impl Hash for SyntaxNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.range.hash(state);
        self.source().hash(state);
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
                    self.stack
                        .extend(node.child_elements().collect::<Vec<_>>().into_iter().rev());
                }
            }
            Some(element)
        }
    }
}
