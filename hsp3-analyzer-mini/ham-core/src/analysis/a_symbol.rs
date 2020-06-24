use super::{ALoc, AScope};
use crate::utils::{id::Id, rc_str::RcStr};

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum ASymbolKind {
    Unresolved,
    /// `#deffunc` etc.
    Command,
    /// `#func`
    CommandOrFunc,
    /// `#cmd`
    CommandOrFuncOrVar,
    Const,
    Directory,
    Enum,
    Field,
    File,
    /// `#defcfunc` etc.
    Func,
    Label,
    Module,
    Param,
    PreProc,
    StaticVar,
    Type,
}

impl Default for ASymbolKind {
    fn default() -> Self {
        ASymbolKind::Unresolved
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
