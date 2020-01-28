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
    pub name: String,
    pub body: KNode,
}

#[derive(Clone, Debug)]
pub(crate) struct KModule {
    pub name: String,
    pub fns: Vec<KFn>,
}

#[derive(Clone, Debug)]
pub(crate) struct KRoot {
    pub modules: Vec<KModule>,
}
