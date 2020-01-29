use super::*;
use crate::ast::*;

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub(crate) struct KFn {
    pub(crate) name: String,
    pub(crate) body: KNode,
}

#[derive(Clone, Debug)]
pub(crate) struct KModule {
    pub(crate) name: String,
    pub(crate) fns: Vec<KFn>,
}

#[derive(Clone, Debug)]
pub(crate) struct KRoot {
    pub(crate) modules: Vec<KModule>,
}
