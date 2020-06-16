use crate::utils::id::Id;

#[allow(unused)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum ASymbolKind {
    None,
    Command,
    Const,
    Directory,
    Enum,
    File,
    Func,
    Label,
    Module,
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

pub(crate) struct ASymbolData;

#[allow(unused)]
pub(crate) struct ASymbolDetails {
    pub(crate) desc: String,
    pub(crate) docs: Vec<String>,
}
