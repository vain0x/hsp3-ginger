// 名前解決に関係するもの。スコープや名前空間など。

use super::{a_scope::*, a_symbol::*};
use crate::{source::*, utils::rc_str::RcStr};
use std::collections::HashMap;

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
#[derive(Debug, Default)]
pub(crate) struct SymbolEnv {
    map: HashMap<RcStr, AWsSymbol>,
}

impl SymbolEnv {
    pub(crate) fn get(&self, name: &str) -> Option<AWsSymbol> {
        self.map.get(name).cloned()
    }

    pub(crate) fn insert(&mut self, name: RcStr, symbol: AWsSymbol) {
        self.map.insert(name, symbol);
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }
}

#[derive(Default)]
pub(crate) struct APublicEnv {
    /// 標準命令などのシンボルが属す環境。(この環境はソースファイルの変更時に無効化しないので、globalと分けている。)
    pub(crate) builtin: SymbolEnv,

    /// あらゆる場所で使えるシンボルが属す環境。(標準命令や `#define global` で定義されたマクロなど)
    pub(crate) global: SymbolEnv,
}

impl APublicEnv {
    pub(crate) fn resolve(&self, name: &str) -> Option<AWsSymbol> {
        self.global.get(name).or_else(|| self.builtin.get(name))
    }

    pub(crate) fn clear(&mut self) {
        self.global.clear();
    }
}

/// globalではないスコープ
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct ALocalScope {
    pub(crate) module_opt: Option<AModule>,

    /// `#deffunc` 系命令の下の部分。(このスコープに属して定義されるのはパラメータだけ。)
    pub(crate) deffunc_opt: Option<ADefFunc>,
}

impl ALocalScope {
    pub(crate) fn is_public(&self) -> bool {
        self.module_opt.is_none() && self.deffunc_opt.is_none()
    }

    pub(crate) fn is_outside_module(&self) -> bool {
        self.module_opt.is_none()
    }

    /// スコープselfで定義されたシンボルが、スコープotherにおいてみえるか？
    pub(crate) fn is_visible_to(&self, other: &ALocalScope) -> bool {
        // 異なるモジュールに定義されたものはみえない。
        // deffuncの中で定義されたものは、その中でしかみえないが、外で定義されたものは中からもみえる。
        self.module_opt == other.module_opt
            && (self.deffunc_opt.is_none() || self.deffunc_opt == other.deffunc_opt)
    }
}

/// スコープ。シンボルの有効範囲
#[derive(Clone, Debug)]
pub(crate) enum AScope {
    Global,
    Local(ALocalScope),
}

impl AScope {
    /// スコープselfで定義されたシンボルが、スコープotherにおいてみえるか？
    pub(crate) fn is_visible_to(&self, other: &ALocalScope) -> bool {
        match self {
            AScope::Local(scope) => scope.is_visible_to(other),
            _ => false,
        }
    }
}

/// シンボルをスコープに追加するときのモード
#[derive(Clone, Copy)]
pub(crate) enum ADefScope {
    Global,
    Local,
    Param,
}

/// 名前、スコープ、名前空間。
pub(crate) struct NameScopeNsTriple {
    pub(crate) basename: RcStr,
    pub(crate) scope_opt: Option<AScope>,
    pub(crate) ns_opt: Option<RcStr>,
}

/// 定義箇所の名前に関連付けられるスコープと名前空間を決定する。
pub(crate) fn resolve_name_scope_ns_for_def(
    basename: &RcStr,
    def: ADefScope,
    local: &ALocalScope,
) -> NameScopeNsTriple {
    let NamePath { base, qual } = NamePath::new(basename);

    // 識別子が非修飾のときはスコープに属す。
    // 例外的に、`@` で修飾された識別子はglobalスコープかtoplevelスコープに属す。
    let scope_opt = match (&qual, def, &local.module_opt) {
        (Qual::Unqualified, ADefScope::Param, _) => Some(AScope::Local(local.clone())),
        (Qual::Unqualified, ADefScope::Global, _) | (Qual::Toplevel, ADefScope::Global, _) => {
            Some(AScope::Global)
        }
        (Qual::Unqualified, ADefScope::Local, _) => {
            let scope = AScope::Local(ALocalScope {
                deffunc_opt: None,
                ..local.clone()
            });
            Some(scope)
        }
        (Qual::Toplevel, ADefScope::Local, _) => Some(AScope::Local(ALocalScope::default())),
        _ => None,
    };

    let ns_opt: Option<RcStr> = match (qual, def, &local.module_opt) {
        (_, ADefScope::Param, _) => None,
        (Qual::Module(ns), _, _) => Some(ns),
        (Qual::Toplevel, _, _)
        | (Qual::Unqualified, ADefScope::Global, _)
        | (Qual::Unqualified, ADefScope::Local, None) => Some("".into()),
        (Qual::Unqualified, ADefScope::Local, Some(m)) => m.name_opt.clone(),
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
    local: &ALocalScope,
) -> NameScopeNsTriple {
    let NamePath { base, qual } = NamePath::new(basename);

    let scope_opt = match &qual {
        Qual::Unqualified => Some(AScope::Local(local.clone())),
        Qual::Toplevel => Some(AScope::Local(ALocalScope::default())),
        Qual::Module(_) => None,
    };

    let ns_opt: Option<RcStr> = match (qual, &local.module_opt) {
        (Qual::Module(ns), _) => Some(ns),
        (Qual::Toplevel, _) | (Qual::Unqualified, None) => Some("".into()),
        (Qual::Unqualified, Some(m)) => m.name_opt.clone(),
    };

    NameScopeNsTriple {
        basename: base,
        scope_opt,
        ns_opt,
    }
}

pub(crate) fn resolve_implicit_symbol(
    name: &RcStr,
    local: &ALocalScope,
    public_env: &APublicEnv,
    ns_env: &HashMap<RcStr, SymbolEnv>,
    local_env: &HashMap<ALocalScope, SymbolEnv>,
) -> Option<AWsSymbol> {
    let NameScopeNsTriple {
        basename,
        scope_opt,
        ns_opt,
    } = resolve_name_scope_ns_for_use(name, local);

    if let Some(AScope::Local(scope)) = &scope_opt {
        // ローカル環境で探す
        if let it @ Some(_) = local_env.get(&scope).and_then(|env| env.get(&basename)) {
            return it;
        }

        // deffuncの外からも探す。
        if scope.deffunc_opt.is_some() {
            let scope = ALocalScope {
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
    ws_symbol: AWsSymbol,
    basename: RcStr,
    scope_opt: Option<AScope>,
    ns_opt: Option<RcStr>,
    public_env: &mut APublicEnv,
    local_env: &mut HashMap<ALocalScope, SymbolEnv>,
    ns_env: &mut HashMap<RcStr, SymbolEnv>,
) {
    let env_opt = match scope_opt {
        Some(AScope::Global) => Some(&mut public_env.global),
        Some(AScope::Local(scope)) => Some(local_env.entry(scope).or_default()),
        None => None,
    };

    if let Some(env) = env_opt {
        env.insert(basename.clone(), ws_symbol);
    }

    if let Some(ns) = ns_opt {
        ns_env.entry(ns).or_default().insert(basename, ws_symbol);
    }
}

pub(crate) fn extend_public_env_from_symbols(
    doc: DocId,
    symbols: &[ASymbolData],
    public_env: &mut APublicEnv,
    ns_env: &mut HashMap<RcStr, SymbolEnv>,
) {
    for (i, symbol_data) in symbols.iter().enumerate() {
        let symbol = ASymbol::new(i);
        let ws_symbol = AWsSymbol { doc, symbol };

        if let Some(AScope::Global) = &symbol_data.scope_opt {
            public_env
                .global
                .insert(symbol_data.name.clone(), ws_symbol);
        }

        if let Some(ns) = &symbol_data.ns_opt {
            ns_env
                .entry(ns.clone())
                .or_default()
                .insert(symbol_data.name.clone(), ws_symbol);
        }
    }
}
