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
pub(crate) struct SymbolRc(Rc<SymbolData>);

impl SymbolRc {
    pub(crate) fn from(data: SymbolData) -> Self {
        Self(Rc::new(data))
    }

    pub(crate) fn name(&self) -> RcStr {
        self.0.name.clone()
    }

    pub(crate) fn signature_opt(&self) -> Option<Rc<SignatureData>> {
        self.0.signature_opt.borrow().clone()
    }

    pub(crate) fn compute_details(&self) -> SymbolDetails {
        if let Some(details) = self.details_opt.as_ref() {
            return details.clone();
        }

        let item_opt = self.linked_symbol_opt.borrow();
        if let Some(item) = item_opt.as_ref() {
            return SymbolDetails {
                desc: item.detail.clone().map(RcStr::from),
                docs: item
                    .documentation
                    .as_ref()
                    .into_iter()
                    .flat_map(|text| match text {
                        lsp_types::Documentation::String(it) => Some(it.clone()),
                        _ => None,
                    })
                    .collect(),
            };
        }

        match &self.leader_opt {
            Some(leader) => calculate_details(&collect_comments(leader)),
            None => SymbolDetails::default(),
        }
    }
}

impl AsRef<SymbolData> for SymbolRc {
    fn as_ref(&self) -> &SymbolData {
        self.0.as_ref()
    }
}

impl Deref for SymbolRc {
    type Target = SymbolData;

    fn deref(&self) -> &SymbolData {
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

/// シンボルを定義するもの
pub(crate) enum DefInfo {
    HspHelp {
        name: RcStr,
        details: SymbolDetails,
        signature_opt: Option<Rc<SignatureData>>,
    },
    Preproc {
        kind: HspSymbolKind,
        basename: RcStr,
        scope_opt: Option<Scope>,
        ns_opt: Option<RcStr>,
        leader: PToken,
        loc: Loc,
    },
    Name {
        name: PToken,
        kind: HspSymbolKind,
        basename: RcStr,
        scope_opt: Option<Scope>,
        ns_opt: Option<RcStr>,
    },
}

impl DefInfo {
    pub(crate) fn into_symbol(self) -> SymbolRc {
        let symbol_data = match self {
            DefInfo::HspHelp {
                name,
                details,
                signature_opt,
            } => SymbolData {
                kind: HspSymbolKind::Unknown,
                name,
                scope_opt: None,
                ns_opt: None,
                leader_opt: None,
                details_opt: Some(details),

                preproc_def_site_opt: None,
                signature_opt: RefCell::new(signature_opt),
                linked_symbol_opt: Default::default(),
            },
            DefInfo::Preproc {
                leader,
                kind,
                basename,
                scope_opt,
                ns_opt,
                loc,
            } => SymbolData {
                kind,
                name: basename,
                scope_opt,
                ns_opt,
                leader_opt: Some(leader),

                details_opt: None,
                preproc_def_site_opt: Some(loc),
                signature_opt: Default::default(),
                linked_symbol_opt: Default::default(),
            },
            DefInfo::Name {
                name,
                kind,
                basename,
                scope_opt,
                ns_opt,
            } => SymbolData {
                kind,
                name: basename,
                scope_opt,
                ns_opt,
                leader_opt: Some(name),

                details_opt: None,
                preproc_def_site_opt: None,
                signature_opt: Default::default(),
                linked_symbol_opt: Default::default(),
            },
        };
        SymbolRc::from(symbol_data)
    }
}

pub(crate) struct SymbolData {
    pub(crate) kind: HspSymbolKind,
    pub(crate) name: RcStr,
    pub(crate) scope_opt: Option<Scope>,
    pub(crate) ns_opt: Option<RcStr>,
    leader_opt: Option<PToken>,

    details_opt: Option<SymbolDetails>,
    pub(crate) preproc_def_site_opt: Option<Loc>,

    // 追加の情報:
    pub(crate) signature_opt: RefCell<Option<Rc<SignatureData>>>,
    pub(crate) linked_symbol_opt: RefCell<Option<lsp_types::CompletionItem>>,
}

#[derive(Clone, Default)]
pub(crate) struct SymbolDetails {
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
