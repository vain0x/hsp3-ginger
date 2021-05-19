// 名前解決に関係するもの。スコープや名前空間など。

use super::*;

/// 名前の修飾子。
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Qual {
    /// 非修飾。`xxx`
    Unqualified,

    /// トップレベルの名前空間の修飾付き。`xxx@`
    Toplevel,

    /// モジュールの名前空間の修飾付き。`xxx@m_hoge`
    Module(RcStr),
}

/// 名前: 識別子をbasenameと修飾子に分解したもの。
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct NamePath {
    /// `@` の前の部分
    pub(crate) base: RcStr,
    /// `@` 以降の部分
    pub(crate) qual: Qual,
}

impl NamePath {
    pub(crate) fn new(name: &RcStr) -> Self {
        match name.rfind('@') {
            Some(i) if i + 1 == name.len() => NamePath {
                base: name.slice(0, i),
                qual: Qual::Toplevel,
            },
            Some(i) => NamePath {
                base: name.slice(0, i),
                qual: Qual::Module(name.slice(i + 1, name.len())),
            },
            None => NamePath {
                base: name.clone(),
                qual: Qual::Unqualified,
            },
        }
    }
}

/// 環境。名前からシンボルへのマップ。
#[derive(Clone, Debug, Default)]
pub(crate) struct SymbolEnv {
    map: HashMap<RcStr, SymbolRc>,
}

impl SymbolEnv {
    pub(crate) fn get(&self, name: &str) -> Option<SymbolRc> {
        self.map.get(name).cloned()
    }

    pub(crate) fn insert(&mut self, name: RcStr, symbol: SymbolRc) {
        self.map.insert(name, symbol);
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }
}

/// 名前空間
pub(crate) type NsEnv = HashMap<RcStr, SymbolEnv>;

#[derive(Default)]
pub(crate) struct PublicEnv {
    /// 標準命令などのシンボルが属す環境。(この環境はソースファイルの変更時に無効化しないので、globalと分けている。)
    pub(crate) builtin: Rc<SymbolEnv>,

    /// あらゆる場所で使えるシンボルが属す環境。(標準命令や `#define global` で定義されたマクロなど)
    pub(crate) global: SymbolEnv,
}

impl PublicEnv {
    pub(crate) fn resolve(&self, name: &str) -> Option<SymbolRc> {
        self.global.get(name).or_else(|| self.builtin.get(name))
    }

    pub(crate) fn clear(&mut self) {
        self.global.clear();
    }
}

/// globalではないスコープ
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct LocalScope {
    pub(crate) module_opt: Option<ModuleKey>,

    /// `#deffunc` 系命令の下の部分。(このスコープに属して定義されるのはパラメータだけ。)
    pub(crate) deffunc_opt: Option<DefFuncKey>,
}

impl LocalScope {
    pub(crate) fn is_public(&self) -> bool {
        self.module_opt.is_none() && self.deffunc_opt.is_none()
    }

    pub(crate) fn is_outside_module(&self) -> bool {
        self.module_opt.is_none()
    }

    /// スコープselfで定義されたシンボルが、スコープotherにおいてみえるか？
    pub(crate) fn is_visible_to(&self, other: &LocalScope) -> bool {
        // 異なるモジュールに定義されたものはみえない。
        // deffuncの中で定義されたものは、その中でしかみえないが、外で定義されたものは中からもみえる。
        self.module_opt == other.module_opt
            && (self.deffunc_opt.is_none() || self.deffunc_opt == other.deffunc_opt)
    }
}

/// スコープ。シンボルの有効範囲
#[derive(Clone, Debug)]
pub(crate) enum Scope {
    Global,
    Local(LocalScope),
}

impl Scope {
    /// globalかトップレベル？
    pub(crate) fn is_public(&self) -> bool {
        match self {
            Scope::Global => true,
            Scope::Local(local) => local.is_public(),
        }
    }

    /// スコープselfで定義されたシンボルが、スコープotherにおいてみえるか？
    pub(crate) fn is_visible_to(&self, other: &LocalScope) -> bool {
        match self {
            Scope::Local(scope) => scope.is_visible_to(other),
            _ => false,
        }
    }
}

/// シンボルをスコープに追加するときのモード
#[derive(Clone, Copy)]
pub(crate) enum ImportMode {
    Global,
    Local,
    Param,
}

/// 名前、スコープ、名前空間。
pub(crate) struct NameScopeNsTriple {
    pub(crate) basename: RcStr,
    pub(crate) scope_opt: Option<Scope>,
    pub(crate) ns_opt: Option<RcStr>,
}

fn module_name(m: ModuleKey, module_name_map: &ModuleNameMap) -> Option<RcStr> {
    module_name_map.get(&m).cloned()
}

/// 定義箇所の名前に関連付けられるスコープと名前空間を決定する。
pub(crate) fn resolve_name_scope_ns_for_def(
    basename: &RcStr,
    mode: ImportMode,
    local: &LocalScope,
    module_name_map: &ModuleNameMap,
) -> NameScopeNsTriple {
    let NamePath { base, qual } = NamePath::new(basename);

    // 識別子が非修飾のときはスコープに属す。
    // 例外的に、`@` で修飾された識別子はglobalスコープかtoplevelスコープに属す。
    let scope_opt = match (&qual, mode, &local.module_opt) {
        (Qual::Unqualified, ImportMode::Param, _) => Some(Scope::Local(local.clone())),
        (Qual::Unqualified, ImportMode::Global, _) | (Qual::Toplevel, ImportMode::Global, _) => {
            Some(Scope::Global)
        }
        (Qual::Unqualified, ImportMode::Local, _) => {
            let scope = Scope::Local(LocalScope {
                deffunc_opt: None,
                ..local.clone()
            });
            Some(scope)
        }
        (Qual::Toplevel, ImportMode::Local, _) => Some(Scope::Local(LocalScope::default())),
        _ => None,
    };

    let ns_opt: Option<RcStr> = match (qual, mode, &local.module_opt) {
        (_, ImportMode::Param, _) => None,
        (Qual::Module(ns), _, _) => Some(ns),
        (Qual::Toplevel, _, _)
        | (Qual::Unqualified, ImportMode::Global, _)
        | (Qual::Unqualified, ImportMode::Local, None) => Some("".into()),
        (Qual::Unqualified, ImportMode::Local, Some(m)) => module_name(*m, module_name_map),
    };

    NameScopeNsTriple {
        basename: base,
        scope_opt,
        ns_opt,
    }
}

/// 使用箇所の名前に関連付けられるスコープと名前空間を決定する。
pub(crate) fn resolve_name_scope_ns_for_use(
    basename: &RcStr,
    local: &LocalScope,
    module_name_map: &ModuleNameMap,
) -> NameScopeNsTriple {
    let NamePath { base, qual } = NamePath::new(basename);

    let scope_opt = match &qual {
        Qual::Unqualified => Some(Scope::Local(local.clone())),
        Qual::Toplevel => Some(Scope::Local(LocalScope::default())),
        Qual::Module(_) => None,
    };

    let ns_opt: Option<RcStr> = match (qual, &local.module_opt) {
        (Qual::Module(ns), _) => Some(ns),
        (Qual::Toplevel, _) | (Qual::Unqualified, None) => Some("".into()),
        (Qual::Unqualified, Some(m)) => module_name(*m, module_name_map),
    };

    NameScopeNsTriple {
        basename: base,
        scope_opt,
        ns_opt,
    }
}

pub(crate) fn resolve_implicit_symbol(
    name: &RcStr,
    local: &LocalScope,
    public_env: &PublicEnv,
    ns_env: &HashMap<RcStr, SymbolEnv>,
    local_env: &HashMap<LocalScope, SymbolEnv>,
    module_name_map: &ModuleNameMap,
) -> Option<SymbolRc> {
    let NameScopeNsTriple {
        basename,
        scope_opt,
        ns_opt,
    } = resolve_name_scope_ns_for_use(name, local, module_name_map);

    if let Some(Scope::Local(scope)) = &scope_opt {
        // ローカル環境で探す
        if let it @ Some(_) = local_env.get(&scope).and_then(|env| env.get(&basename)) {
            return it;
        }

        // deffuncの外からも探す。
        if scope.deffunc_opt.is_some() {
            let scope = LocalScope {
                deffunc_opt: None,
                ..scope.clone()
            };
            if let it @ Some(_) = local_env.get(&scope).and_then(|env| env.get(&basename)) {
                return it;
            }
        }
    }

    if let Some(ns) = &ns_opt {
        if let it @ Some(_) = ns_env.get(ns).and_then(|env| env.get(&basename)) {
            return it;
        }
    }

    if let Some(_) = scope_opt {
        // globalで探す。
        if let it @ Some(_) = public_env.resolve(&basename) {
            return it;
        }
    }

    None
}

pub(crate) fn import_symbol_to_env(
    symbol: &SymbolRc,
    basename: RcStr,
    scope_opt: Option<Scope>,
    ns_opt: Option<RcStr>,
    public_env: &mut PublicEnv,
    ns_env: &mut NsEnv,
    local_env: &mut HashMap<LocalScope, SymbolEnv>,
) {
    let env_opt = match scope_opt {
        Some(Scope::Global) => Some(&mut public_env.global),
        Some(Scope::Local(scope)) => Some(local_env.entry(scope).or_default()),
        None => None,
    };

    if let Some(env) = env_opt {
        env.insert(basename.clone(), symbol.clone());
    }

    if let Some(ns) = ns_opt {
        ns_env
            .entry(ns)
            .or_default()
            .insert(basename, symbol.clone());
    }
}

pub(crate) fn extend_public_env_from_symbols(
    symbols: &[SymbolRc],
    public_env: &mut PublicEnv,
    ns_env: &mut NsEnv,
) {
    for symbol in symbols.iter().cloned() {
        if let Some(Scope::Global) = &symbol.scope_opt {
            public_env.global.insert(symbol.name(), symbol.clone());
        }

        if let Some(ns) = &symbol.ns_opt {
            ns_env
                .entry(ns.clone())
                .or_default()
                .insert(symbol.name(), symbol.clone());
        }
    }
}

pub(crate) fn extend_local_env_from_symbols(
    symbols: &[SymbolRc],
    local_env: &mut HashMap<LocalScope, SymbolEnv>,
) {
    for symbol in symbols.iter().cloned() {
        match &symbol.scope_opt {
            Some(Scope::Local(scope)) if !scope.is_public() => {
                local_env
                    .entry(scope.clone())
                    .or_default()
                    .insert(symbol.name(), symbol.clone());
            }
            _ => {}
        }
    }
}
