use super::AScope;
use crate::{
    parse::{PParamTy, PToken},
    source::{DocId, Loc},
    utils::{id::Id, rc_str::RcStr},
};

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum ASymbolKind {
    /// 定義箇所がみつからない。
    Unresolved,

    Label,
    StaticVar,

    Const,
    Enum,

    /// `#define`
    Macro {
        ctype: bool,
    },

    DefFunc,
    DefCFunc,
    ModFunc,
    ModCFunc,
    Param(Option<PParamTy>),

    Module,

    /// モジュール変数
    Field,

    /// `#func`
    LibFunc,

    /// `#cmd`
    PluginCmd,

    ComInterface,
    ComFunc,
}

impl ASymbolKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ASymbolKind::Unresolved => "不明",
            ASymbolKind::Const => "定数",
            ASymbolKind::Enum => "列挙子",
            ASymbolKind::Macro { ctype: false } => "マクロ",
            ASymbolKind::Macro { ctype: true } => "関数形式マクロ",
            ASymbolKind::DefFunc => "命令",
            ASymbolKind::DefCFunc => "関数",
            ASymbolKind::ModFunc => "命令(モジュール変数)",
            ASymbolKind::ModCFunc => "関数(モジュール変数)",
            ASymbolKind::Param(None) => "パラメータ",
            ASymbolKind::Param(Some(param)) => param.to_str(),
            ASymbolKind::LibFunc => "ライブラリ関数",
            ASymbolKind::PluginCmd => "プラグインコマンド",
            ASymbolKind::Module => "モジュール",
            ASymbolKind::Field => "モジュール変数",
            ASymbolKind::Label => "ラベル",
            ASymbolKind::StaticVar => "変数",
            ASymbolKind::ComInterface => "COMインターフェイス",
            ASymbolKind::ComFunc => "COMメソッド",
        }
    }
}

impl Default for ASymbolKind {
    fn default() -> Self {
        ASymbolKind::Unresolved
    }
}

pub(crate) type ASymbol = Id<ASymbolData>;

#[derive(Clone, Debug)]
pub(crate) struct ASymbolData {
    pub(crate) kind: ASymbolKind,
    pub(crate) name: RcStr,
    pub(crate) def_sites: Vec<Loc>,
    pub(crate) use_sites: Vec<Loc>,
    pub(crate) leader: PToken,
    pub(crate) scope: AScope,
}

#[allow(unused)]
pub(crate) struct ASymbolDetails {
    pub(crate) desc: Option<RcStr>,
    pub(crate) docs: Vec<String>,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AWsSymbol {
    pub(crate) doc: DocId,
    pub(crate) symbol: ASymbol,
}
