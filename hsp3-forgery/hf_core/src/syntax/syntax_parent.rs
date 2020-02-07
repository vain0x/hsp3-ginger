use super::*;
use std::rc::Rc;

/// 具象構文木における親ノードの参照。
#[derive(Clone)]
pub(crate) enum SyntaxParent {
    /// 親ノードを持たないノードには、構文木ルートへの参照を持たせておく。
    /// これにより、どのノードからも構文木ルートを辿れるようにしている。
    Root { root: Rc<SyntaxRoot> },
    NonRoot {
        /// 親ノード。
        node: Rc<SyntaxNode>,

        /// これを持っているノードは、親ノード (`node`) の何番目の子要素か？
        child_index: usize,
    },
}
