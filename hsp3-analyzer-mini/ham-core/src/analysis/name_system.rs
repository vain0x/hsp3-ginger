// 名前解決に関係するもの。スコープや名前空間など。

use super::{a_scope::*, a_symbol::*};
use crate::utils::rc_str::RcStr;
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
pub(crate) struct Name {
    /// `@` の前の部分
    pub(crate) base: RcStr,
    /// `@` 以降の部分
    pub(crate) qual: Qual,
}

impl Name {
    pub(crate) fn new(name: &RcStr) -> Self {
        match name.rfind('@') {
            Some(i) if i + 1 == name.len() => Name {
                base: name.slice(0, i),
                qual: Qual::Toplevel,
            },
            Some(i) => Name {
                base: name.slice(0, i),
                qual: Qual::Module(name.slice(i + 1, name.len())),
            },
            None => Name {
                base: name.clone(),
                qual: Qual::Unqualified,
            },
        }
    }
}

/// 環境。名前からシンボルへのマップ。
#[derive(Debug, Default)]
pub(crate) struct AEnv {
    map: HashMap<RcStr, AWsSymbol>,
}

impl AEnv {
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
    pub(crate) builtin: AEnv,

    /// あらゆる場所で使えるシンボルが属す環境。(標準命令や `#define global` で定義されたマクロなど)
    pub(crate) global: AEnv,
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

/// シンボルをスコープに追加するときのモード
#[derive(Clone, Copy)]
pub(crate) enum ADefScope {
    Global,
    Local,
    Param,
}

/// 定義箇所の名前をシンボルに解決する。
/// (basename, scope, namespace)
pub(crate) fn resolve_symbol_scope(
    basename: &RcStr,
    def: ADefScope,
    local: &ALocalScope,
) -> (RcStr, Option<AScope>, Option<RcStr>) {
    let Name { base, qual } = Name::new(basename);

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

    (base, scope_opt, ns_opt)
}

/// 使用箇所の名前を解決する。
/// (basename, scope, namespace)
pub(crate) fn resolve_symbol_scope_for_search(
    basename: &RcStr,
    local: &ALocalScope,
) -> (RcStr, Option<AScope>, Option<RcStr>) {
    let Name { base, qual } = Name::new(basename);

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

    (base, scope_opt, ns_opt)
}

/// 暗黙のシンボルを解決する。
pub(crate) fn resolve_candidate(
    name: &RcStr,
    local: &ALocalScope,
    public_env: &APublicEnv,
    ns_env: &HashMap<RcStr, AEnv>,
    local_env: &HashMap<ALocalScope, AEnv>,
) -> Option<AWsSymbol> {
    let (basename, scope_opt, ns_opt) = resolve_symbol_scope_for_search(name, local);

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
