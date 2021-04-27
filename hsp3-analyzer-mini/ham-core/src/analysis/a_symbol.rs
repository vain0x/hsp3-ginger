use super::{ADoc, ALoc, AScope};
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
    #[allow(unused)]
    Directory,
    #[allow(unused)]
    Enum,
    Field,
    #[allow(unused)]
    File,
    /// `#defcfunc` etc.
    #[allow(unused)]
    Func,
    Label,
    Module,
    Param,
    #[allow(unused)]
    PreProc,
    StaticVar,
    #[allow(unused)]
    Type,
}

impl ASymbolKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ASymbolKind::Unresolved => "定義がみつかりません",
            ASymbolKind::Command => "命令",
            ASymbolKind::CommandOrFunc => "関数",
            ASymbolKind::CommandOrFuncOrVar => "コマンド",
            ASymbolKind::Const => "定数",
            ASymbolKind::Directory => "ディレクトリ",
            ASymbolKind::Enum => "列挙子",
            ASymbolKind::Field => "フィールド",
            ASymbolKind::File => "ファイル",
            ASymbolKind::Func => "関数",
            ASymbolKind::Label => "ラベル",
            ASymbolKind::Module => "モジュール",
            ASymbolKind::Param => "パラメータ",
            ASymbolKind::PreProc => "プリプロセッサ命令",
            ASymbolKind::StaticVar => "変数",
            ASymbolKind::Type => "型",
        }
    }
}

impl Default for ASymbolKind {
    fn default() -> Self {
        ASymbolKind::Unresolved
    }
}

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

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AWsSymbol {
    pub(crate) doc: ADoc,
    pub(crate) symbol: ASymbol,
}
