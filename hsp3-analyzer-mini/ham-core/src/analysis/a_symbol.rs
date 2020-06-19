use super::{ALoc, AScope};
use crate::utils::{id::Id, rc_str::RcStr};

#[allow(unused)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum ASymbolKind {
    None,
    Command,
    CommandOrFunc,
    CommandOrFuncOrVar,
    Const,
    Directory,
    Enum,
    Field,
    File,
    Func,
    Label,
    Module,
    Param,
    PreProc,
    Type,
}

impl Default for ASymbolKind {
    fn default() -> Self {
        ASymbolKind::None
    }
}

#[allow(unused)]
pub(crate) type ASymbol = Id<ASymbolData>;

#[derive(Debug)]
pub(crate) struct ASymbolData {
    pub(crate) kind: ASymbolKind,
    pub(crate) name: RcStr,
    pub(crate) def_sites: Vec<ALoc>,
    pub(crate) use_sites: Vec<ALoc>,
    pub(crate) comments: Vec<RcStr>,
    pub(crate) scope: AScope,
}

#[allow(unused)]
pub(crate) struct ASymbolDetails {
    pub(crate) desc: Option<RcStr>,
    pub(crate) docs: Vec<String>,
}
