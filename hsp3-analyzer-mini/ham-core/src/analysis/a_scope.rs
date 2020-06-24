use super::ALoc;
use crate::{
    parse::PDefFuncKind,
    utils::{id::Id, rc_str::RcStr},
};

#[derive(Copy, Clone, Debug)]
pub(crate) struct ALocalScope {
    pub(crate) module_opt: Option<AModule>,
    pub(crate) deffunc_opt: Option<ADefFunc>,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum AScope {
    Global,
    Local(ALocalScope),
}

pub(crate) type ADefFunc = Id<ADefFuncData>;

#[derive(Debug)]
pub(crate) struct ADefFuncData {
    pub(crate) kind: PDefFuncKind,
    pub(crate) name_opt: Option<RcStr>,
    pub(crate) keyword_loc: ALoc,
    pub(crate) content_loc: ALoc,
}

pub(crate) type AModule = Id<AModuleData>;

#[derive(Debug, Default)]
pub(crate) struct AModuleData {
    pub(crate) name_opt: Option<RcStr>,
    pub(crate) keyword_loc: ALoc,
    pub(crate) content_loc: ALoc,
}
