use super::ALoc;
use crate::{
    parse::PDefFuncKind,
    utils::{id::Id, rc_str::RcStr},
};

#[derive(Clone, Debug)]
pub(crate) struct AScope {
    pub(crate) module_opt: Option<AModule>,
    pub(crate) deffunc_opt: Option<ADefFunc>,
    pub(crate) is_global: bool,
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
