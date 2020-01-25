use super::*;

#[derive(Clone)]
pub(crate) enum KNode {
    Entry,
    Abort,
    Prim {
        prim: KPrim,
        args: Vec<KTerm>,
        results: Vec<String>,
        nexts: Vec<KNode>,
    },
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
    Assign { left: KName, next: KNode },
    ReturnWithArg,
}

impl KHole {
    pub(crate) fn apply(self, term: KTerm) -> KNode {
        match self {
            KHole::Entry => KNode::Entry,
            KHole::Assign { left, next } => KNode::Prim {
                prim: KPrim::Assign,
                args: vec![KTerm::Name(left), term],
                results: vec![],
                nexts: vec![next],
            },
            KHole::ReturnWithArg => KNode::Return(KArgs { terms: vec![term] }),
        }
    }
}
