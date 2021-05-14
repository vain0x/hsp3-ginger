use super::{
    comment::{calculate_details, collect_comments},
    preproc::ASignatureData,
    AScope,
};
use crate::{
    parse::{PParamTy, PToken},
    source::{DocId, Loc},
    utils::rc_str::RcStr,
};
use std::{
    cell::RefCell,
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
    rc::Rc,
};

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum ASymbolKind {
    /// 定義箇所がみつからない。
    Unresolved,
    /// 不明 (hsphelpに書いてあるシンボル)
    Unknown,

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
            ASymbolKind::Unresolved => "未解決",
            ASymbolKind::Unknown => "不明",
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

#[derive(Clone)]
pub(crate) struct ASymbol(Rc<ASymbolData>);

impl ASymbol {
    pub(crate) fn from(data: ASymbolData) -> Self {
        Self(Rc::new(data))
    }

    pub(crate) fn name(&self) -> RcStr {
        self.0.name.clone()
    }

    pub(crate) fn signature_opt(&self) -> Option<Rc<ASignatureData>> {
        self.0.signature_opt.borrow().clone()
    }

    pub(crate) fn compute_details(&self) -> ASymbolDetails {
        if let Some(details) = self.details_opt.as_ref() {
            return details.clone();
        }

        match &self.leader_opt {
            Some(leader) => calculate_details(&collect_comments(leader)),
            None => ASymbolDetails::default(),
        }
    }
}

impl AsRef<ASymbolData> for ASymbol {
    fn as_ref(&self) -> &ASymbolData {
        self.0.as_ref()
    }
}

impl Deref for ASymbol {
    type Target = ASymbolData;

    fn deref(&self) -> &ASymbolData {
        self.0.deref()
    }
}

impl PartialEq for ASymbol {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Hash for ASymbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0.as_ref() as *const _ as usize).hash(state)
    }
}

impl Debug for ASymbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0.name, f)
    }
}

pub(crate) struct ASymbolData {
    pub(crate) kind: ASymbolKind,
    pub(crate) name: RcStr,
    pub(crate) leader_opt: Option<PToken>,
    pub(crate) scope_opt: Option<AScope>,
    pub(crate) ns_opt: Option<RcStr>,

    pub(crate) details_opt: Option<ASymbolDetails>,
    pub(crate) preproc_def_site_opt: Option<Loc>,

    // 追加の情報:
    pub(crate) signature_opt: RefCell<Option<Rc<ASignatureData>>>,
}

#[derive(Clone, Default)]
pub(crate) struct ASymbolDetails {
    pub(crate) desc: Option<RcStr>,
    pub(crate) docs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AWsSymbol {
    pub(crate) doc: DocId,
    pub(crate) symbol: ASymbol,
}
