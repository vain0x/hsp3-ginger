use super::comment::*;
use super::*;
use crate::parse::PParamTy;

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) enum HspSymbolKind {
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

impl HspSymbolKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            HspSymbolKind::Unresolved => "未解決",
            HspSymbolKind::Unknown => "不明",
            HspSymbolKind::Const => "定数",
            HspSymbolKind::Enum => "列挙子",
            HspSymbolKind::Macro { ctype: false } => "マクロ",
            HspSymbolKind::Macro { ctype: true } => "関数形式マクロ",
            HspSymbolKind::DefFunc => "命令",
            HspSymbolKind::DefCFunc => "関数",
            HspSymbolKind::ModFunc => "命令(モジュール変数)",
            HspSymbolKind::ModCFunc => "関数(モジュール変数)",
            HspSymbolKind::Param(None) => "パラメータ",
            HspSymbolKind::Param(Some(param)) => param.to_str(),
            HspSymbolKind::LibFunc => "ライブラリ関数",
            HspSymbolKind::PluginCmd => "プラグインコマンド",
            HspSymbolKind::Module => "モジュール",
            HspSymbolKind::Field => "モジュール変数",
            HspSymbolKind::Label => "ラベル",
            HspSymbolKind::StaticVar => "変数",
            HspSymbolKind::ComInterface => "COMインターフェイス",
            HspSymbolKind::ComFunc => "COMメソッド",
        }
    }
}

impl Default for HspSymbolKind {
    fn default() -> Self {
        HspSymbolKind::Unresolved
    }
}

#[derive(Clone)]
pub(crate) struct SymbolRc(Rc<ASymbolData>);

impl SymbolRc {
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

impl AsRef<ASymbolData> for SymbolRc {
    fn as_ref(&self) -> &ASymbolData {
        self.0.as_ref()
    }
}

impl Deref for SymbolRc {
    type Target = ASymbolData;

    fn deref(&self) -> &ASymbolData {
        self.0.deref()
    }
}

impl PartialEq for SymbolRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SymbolRc {}

impl Hash for SymbolRc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0.as_ref() as *const _ as usize).hash(state)
    }
}

impl Debug for SymbolRc {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0.name, f)
    }
}

pub(crate) struct ASymbolData {
    #[allow(unused)]
    pub(crate) doc: DocId,

    pub(crate) kind: HspSymbolKind,
    pub(crate) name: RcStr,
    pub(crate) leader_opt: Option<PToken>,
    pub(crate) scope_opt: Option<Scope>,
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

// -----------------------------------------------
// deffunc
// -----------------------------------------------

pub(crate) type DefFuncMap = HashMap<DefFuncKey, DefFuncData>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DefFuncKey {
    pub(crate) doc: DocId,
    pub(crate) index: usize,
}

impl DefFuncKey {
    pub(crate) fn new(doc: DocId, index: usize) -> Self {
        Self { doc, index }
    }
}

pub(crate) struct DefFuncData {
    pub(crate) content_loc: Loc,
}

// -----------------------------------------------
// module
// -----------------------------------------------

pub(crate) type ModuleMap = HashMap<ModuleKey, ModuleRc>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ModuleKey {
    pub(crate) doc: DocId,
    pub(crate) index: usize,
}

impl ModuleKey {
    pub(crate) fn new(doc: DocId, index: usize) -> ModuleKey {
        Self { doc, index }
    }
}

#[derive(Clone)]
pub(crate) struct ModuleRc(Rc<ModuleData>);

impl ModuleRc {
    pub(crate) fn new(data: ModuleData) -> Self {
        Self(Rc::new(data))
    }
}

impl Deref for ModuleRc {
    type Target = ModuleData;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub(crate) struct ModuleData {
    pub(crate) name_opt: Option<RcStr>,
    pub(crate) content_loc: Loc,
}

/// 文字列リテラルを識別子とみなす。
fn str_as_module_name_ident(s: &RcStr) -> Option<RcStr> {
    // "..." の形で、引用符の間に1文字以上必要。
    if s.len() <= 2 || !s.starts_with('"') || !s.ends_with('"') {
        return None;
    }

    // 数字で始まらないこと。
    if s.chars().next().unwrap().is_ascii_digit() {
        return None;
    }

    // モジュール名として許可されない文字を含まないこと。(`@` も不許可。)
    let ok = s[1..s.len() - 1]
        .chars()
        .all(|c| "_`".contains(c) || (!c.is_ascii_punctuation() && !c.is_control()));
    if !ok {
        return None;
    }

    Some(s.slice(1, s.len() - 1))
}

pub(crate) fn module_name_as_ident(token: &TokenData) -> Option<RcStr> {
    match token.kind {
        TokenKind::Ident => Some(token.text.clone()),
        TokenKind::Str => str_as_module_name_ident(&token.text),
        _ => None,
    }
}
