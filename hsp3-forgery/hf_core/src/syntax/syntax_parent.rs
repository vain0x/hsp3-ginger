use super::*;
use std::rc::Rc;

pub(crate) enum SyntaxParent {
    Root {
        root: Rc<SyntaxRoot>,
    },
    NonRoot {
        node: Rc<SyntaxNode>,
        child_index: usize,
    },
}
