use super::*;
use std::rc::Rc;

/// 親ノードの参照。
#[derive(Clone)]
pub(crate) enum SyntaxParent {
    /// 親ノードを持たないノードは、代わりにルートへの参照を持つことにしておく。
    Root { root: Rc<SyntaxRoot> },
    NonRoot {
        node: Rc<SyntaxNode>,

        /// 自身が親ノードの何番目の子要素か？
        child_index: usize,
    },
}
