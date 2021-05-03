// 変数の定義・使用箇所の列挙。

use super::{
    a_scope::{ADefFunc, ADefScope, ALocalScope, AModule},
    a_symbol::{ASymbolData, AWsSymbol},
    integrate::{AEnv, APublicEnv, Name, Qual},
    AScope, ASymbol, ASymbolKind,
};
use crate::{parse::*, source::*, utils::rc_str::RcStr};
use std::{collections::HashMap, mem::replace};

pub(crate) struct APublicState {
    pub(crate) env: APublicEnv,
    pub(crate) ns_env: HashMap<RcStr, AEnv>,

    // 他のドキュメントのシンボルの定義・使用箇所を記録するもの。
    pub(crate) def_sites: Vec<(AWsSymbol, Loc)>,
    pub(crate) use_sites: Vec<(AWsSymbol, Loc)>,
}

struct Ctx<'a> {
    public: &'a mut APublicState,

    doc: DocId,

    /// ドキュメント内のシンボル
    symbols: Vec<ASymbolData>,

    /// ドキュメント内の環境
    env: HashMap<ALocalScope, AEnv>,

    deffunc_len: usize,
    module_len: usize,
    scope: ALocalScope,
}

/// (basename, scope, namespace)
pub(crate) fn resolve_symbol_scope(
    name: &RcStr,
    def: ADefScope,
    local: &ALocalScope,
) -> (RcStr, Option<AScope>, Option<RcStr>) {
    let Name { base, qual } = Name::new(name);

    // 識別子が非修飾のときはスコープに属す。
    // 例外的に、`@` で修飾された識別子はglobalスコープかtoplevelスコープに属す。
    let scope_opt = match (&qual, def, &local.module_opt) {
        (Qual::Unqualified, ADefScope::Param, _) => Some(AScope::Local(local.clone())),
        (Qual::Unqualified, ADefScope::Global(level), _)
        | (Qual::Toplevel, ADefScope::Global(level), _) => Some(AScope::Global(level)),
        (Qual::Unqualified, ADefScope::Local(level), _) => {
            let scope = AScope::Local(ALocalScope {
                module_opt: local.module_opt.clone(),
                deffunc_opt: None,
                define_level: level,
            });
            Some(scope)
        }
        (Qual::Toplevel, ADefScope::Local(_), _) => Some(AScope::Local(ALocalScope::default())),
        _ => None,
    };

    let ns_opt: Option<RcStr> = match (qual, def, &local.module_opt) {
        (_, ADefScope::Param, _) => None,
        (Qual::Module(ns), _, _) => Some(ns),
        (Qual::Toplevel, _, _)
        | (Qual::Unqualified, ADefScope::Global(_), _)
        | (Qual::Unqualified, ADefScope::Local(_), None) => Some("".into()),
        (Qual::Unqualified, ADefScope::Local(_), Some(m)) => m.name_opt.clone(),
    };

    (base, scope_opt, ns_opt)
}

/// (basename, scope, namespace)
pub(crate) fn resolve_symbol_scope_for_search(
    name: &RcStr,
    local: &ALocalScope,
) -> (RcStr, Option<AScope>, Option<RcStr>) {
    let Name { base, qual } = Name::new(name);

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

/// 暗黙のシンボルの出現を解決する。
fn resolve_candidate(
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

        // 他のレベルからも探す。

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

const DEF_SITE: bool = true;
const USE_SITE: bool = false;

fn add_symbol(kind: ASymbolKind, name: &PToken, def_site: bool, ctx: &mut Ctx) {
    let (basename, scope_opt, ns_opt) =
        resolve_symbol_scope(&name.body.text, ADefScope::Local(0), &ctx.scope);

    // 新しいシンボルを登録する。
    let symbol = ASymbol::new(ctx.symbols.len());

    let mut symbol_data = ASymbolData {
        kind,
        name: basename.clone(),
        def_sites: vec![],
        use_sites: vec![],
        leader: name.clone(),
        scope_opt: scope_opt.clone(),
        ns_opt: ns_opt.clone(),
    };

    if def_site {
        symbol_data.def_sites.push(name.body.loc);
    } else {
        symbol_data.use_sites.push(name.body.loc);
    }

    ctx.symbols.push(symbol_data);

    // 環境に追加する。
    let ws_symbol = AWsSymbol {
        doc: ctx.doc,
        symbol,
    };
    let env_opt = match scope_opt {
        Some(AScope::Global(_)) => Some(&mut ctx.public.env.global),
        Some(AScope::Local(scope)) => Some(ctx.env.entry(scope).or_default()),
        None => None,
    };
    if let Some(env) = env_opt {
        env.insert(basename.clone(), ws_symbol);
    }

    if let Some(ns) = ns_opt {
        ctx.public
            .ns_env
            .entry(ns)
            .or_default()
            .insert(basename, ws_symbol);
    }
}

fn on_symbol_def(name: &PToken, ctx: &mut Ctx) {
    match resolve_candidate(
        &name.body.text,
        &ctx.scope,
        &ctx.public.env,
        &ctx.public.ns_env,
        &ctx.env,
    ) {
        Some(ws_symbol) if ws_symbol.doc != ctx.doc => {
            ctx.public.def_sites.push((ws_symbol, name.body.loc));
        }
        Some(ws_symbol) => {
            assert_eq!(ws_symbol.doc, ctx.doc);
            ctx.symbols[ws_symbol.symbol.get()]
                .def_sites
                .push(name.body.loc);
        }
        None => add_symbol(ASymbolKind::StaticVar, name, DEF_SITE, ctx),
    }
}

fn on_symbol_use(name: &PToken, is_var: bool, ctx: &mut Ctx) {
    match resolve_candidate(
        &name.body.text,
        &ctx.scope,
        &ctx.public.env,
        &ctx.public.ns_env,
        &ctx.env,
    ) {
        Some(ws_symbol) if ws_symbol.doc != ctx.doc => {
            ctx.public.use_sites.push((ws_symbol, name.body.loc));
        }
        Some(ws_symbol) => {
            assert_eq!(ws_symbol.doc, ctx.doc);
            ctx.symbols[ws_symbol.symbol.get()]
                .use_sites
                .push(name.body.loc);
        }
        None => {
            let kind = if is_var {
                ASymbolKind::StaticVar
            } else {
                ASymbolKind::Unresolved
            };
            add_symbol(kind, name, USE_SITE, ctx);
        }
    }
}

fn on_compound_def(compound: &PCompound, ctx: &mut Ctx) {
    match compound {
        PCompound::Name(name) => on_symbol_def(name, ctx),
        PCompound::Paren(PNameParen { name, args, .. }) => {
            on_symbol_def(name, ctx);

            for arg in args {
                on_expr_opt(arg.expr_opt.as_ref(), ctx);
            }
        }
        PCompound::Dots(PNameDot { name, args }) => {
            on_symbol_def(name, ctx);

            for arg in args {
                on_expr_opt(arg.expr_opt.as_ref(), ctx);
            }
        }
    }
}

fn on_compound_use(compound: &PCompound, ctx: &mut Ctx) {
    match compound {
        PCompound::Name(name) => on_symbol_use(name, true, ctx),
        PCompound::Paren(PNameParen { name, args, .. }) => {
            on_symbol_use(name, true, ctx);

            for arg in args {
                on_expr_opt(arg.expr_opt.as_ref(), ctx);
            }
        }
        PCompound::Dots(PNameDot { name, args }) => {
            on_symbol_use(name, true, ctx);

            for arg in args {
                on_expr_opt(arg.expr_opt.as_ref(), ctx);
            }
        }
    }
}

fn on_expr(expr: &PExpr, ctx: &mut Ctx) {
    match expr {
        PExpr::Literal(_) => {}
        PExpr::Label(PLabel { star: _, name_opt }) => {
            if let Some(name) = name_opt {
                on_symbol_use(name, false, ctx);
            }
        }
        PExpr::Compound(compound) => on_compound_use(compound, ctx),
        PExpr::Paren(PParenExpr { body_opt, .. }) => on_expr_opt(body_opt.as_deref(), ctx),
        PExpr::Prefix(PPrefixExpr { prefix: _, arg_opt }) => on_expr_opt(arg_opt.as_deref(), ctx),
        PExpr::Infix(PInfixExpr {
            infix: _,
            left,
            right_opt,
        }) => {
            on_expr(left, ctx);
            on_expr_opt(right_opt.as_deref(), ctx);
        }
    }
}

fn on_expr_opt(expr_opt: Option<&PExpr>, ctx: &mut Ctx) {
    if let Some(expr) = expr_opt {
        on_expr(expr, ctx);
    }
}

fn on_args(args: &[PArg], ctx: &mut Ctx) {
    for arg in args {
        on_expr_opt(arg.expr_opt.as_ref(), ctx);
    }
}

fn on_stmt(stmt: &PStmt, ctx: &mut Ctx) {
    match stmt {
        PStmt::Label(PLabel { name_opt, .. }) => {
            if let Some(name) = name_opt {
                add_symbol(ASymbolKind::Label, name, DEF_SITE, ctx);
            }
        }
        PStmt::Assign(PAssignStmt {
            left,
            op_opt: _,
            args,
        }) => {
            // FIXME: def/use は演算子の種類による
            on_compound_def(left, ctx);
            on_args(args, ctx);
        }
        PStmt::Command(PCommandStmt { command, args, .. }) => {
            on_symbol_use(command, false, ctx);

            static COMMANDS: &[&str] = &[
                "ldim", "sdim", "ddim", "dim", "dimtype", "newlab", "newmod", "dup", "dupptr",
                "mref",
            ];

            let mut i = 0;

            if COMMANDS.contains(&command.body_text()) {
                if let Some(PArg {
                    expr_opt: Some(PExpr::Compound(compound)),
                    ..
                }) = args.get(0)
                {
                    i += 1;
                    on_compound_def(compound, ctx);
                }
            }

            on_args(&args[i..], ctx);
        }
        PStmt::Invoke(PInvokeStmt {
            left,
            method_opt,
            args,
            ..
        }) => {
            on_compound_use(left, ctx);
            on_expr_opt(method_opt.as_ref(), ctx);
            on_args(&args, ctx);
        }
        PStmt::DefFunc(PDefFuncStmt { stmts, .. }) => {
            ctx.deffunc_len += 1;
            let deffunc = ADefFunc::new(ctx.deffunc_len);

            let parent_deffunc = replace(&mut ctx.scope.deffunc_opt, Some(deffunc));

            for stmt in stmts {
                on_stmt(stmt, ctx);
            }

            ctx.scope.deffunc_opt = parent_deffunc;
        }
        PStmt::Module(PModuleStmt {
            name_opt, stmts, ..
        }) => {
            let module = AModule::new(ctx.doc, &mut ctx.module_len, name_opt);

            let define_level = ctx.scope.define_level;
            let parent_scope = replace(
                &mut ctx.scope,
                ALocalScope {
                    deffunc_opt: None,
                    module_opt: Some(module),
                    define_level,
                },
            );

            for stmt in stmts {
                on_stmt(stmt, ctx);
            }

            ctx.scope = parent_scope;
        }

        PStmt::Const(_) | PStmt::Define(_) | PStmt::Enum(_) | PStmt::Cmd(_) => {
            ctx.scope.define_level += 1;
        }

        PStmt::UseLib(_)
        | PStmt::LibFunc(_)
        | PStmt::UseCom(_)
        | PStmt::ComFunc(_)
        | PStmt::RegCmd(_)
        | PStmt::Global(_)
        | PStmt::Include(_)
        | PStmt::UnknownPreProc(_) => {}
    }
}

#[derive(Default)]
pub(crate) struct AAnalysis {
    pub(crate) symbols: Vec<ASymbolData>,

    /// 解析前にあったシンボルの個数。
    #[allow(unused)]
    preproc_symbol_len: usize,
}

pub(crate) fn analyze_var_def(
    doc: DocId,
    root: &PRoot,
    symbols: Vec<ASymbolData>,
    public: &mut APublicState,
) -> AAnalysis {
    let preproc_symbol_len = symbols.len();

    let mut local_env: HashMap<ALocalScope, AEnv> = HashMap::new();

    for (i, symbol) in symbols.iter().enumerate() {
        let ws_symbol = AWsSymbol {
            doc,
            symbol: ASymbol::new(i),
        };

        match &symbol.scope_opt {
            Some(AScope::Local(scope)) if !scope.is_public() => {
                local_env
                    .entry(scope.clone())
                    .or_default()
                    .insert(symbol.name.clone(), ws_symbol);
            }
            _ => {}
        }
    }

    let mut ctx = Ctx {
        public,
        doc,
        symbols,
        env: local_env,
        deffunc_len: 0,
        module_len: 0,
        scope: ALocalScope::default(),
    };

    for stmt in &root.stmts {
        on_stmt(stmt, &mut ctx);
    }

    AAnalysis {
        symbols: ctx.symbols,
        preproc_symbol_len,
    }
}
