use super::*;

#[derive(Clone)]
pub(crate) enum KNode {
    Entry,
    Return(KArgs),
}

#[derive(Clone)]
pub(crate) struct KFn {
    pub name: String,
    pub body: KNode,
}

#[derive(Clone)]
pub(crate) struct KModule {
    pub name: String,
    pub fns: Vec<KFn>,
}

#[derive(Clone)]
pub(crate) struct KRoot {
    pub modules: Vec<KModule>,
}

pub(crate) enum KHole {
    Entry,
    ReturnWithArg,
}

impl KHole {
    pub(crate) fn apply(self, term: KTerm) -> KNode {
        match self {
            KHole::Entry => KNode::Entry,
            KHole::ReturnWithArg => KNode::Return(KArgs { terms: vec![term] }),
        }
    }
}
