// 変数の定義・使用箇所の列挙。

use super::{
    a_scope::{ADefFunc, ALocalScope, AModule},
    a_symbol::{ASymbolData, AWsSymbol},
    integrate::{AEnv, APublicEnv},
    AScope, ASymbol, ASymbolKind,
};
use crate::{parse::*, source::*};
use std::{collections::HashMap, mem::replace};

pub(crate) struct APublicState {
    pub(crate) env: APublicEnv,

    // 他のドキュメントのシンボルの定義・使用箇所を記録するもの。
    pub(crate) def_sites: Vec<(AWsSymbol, ALoc)>,
    pub(crate) use_sites: Vec<(AWsSymbol, ALoc)>,
}

struct Ctx<'a> {
    public: &'a mut APublicState,

    doc: ADoc,

    /// ドキュメント内のシンボル
    symbols: Vec<ASymbolData>,

    /// ドキュメント内の環境
    env: HashMap<ALocalScope, AEnv>,

    deffunc_len: usize,
    module_len: usize,
    scope: ALocalScope,
}

impl Ctx<'_> {
    fn module_scope(&self) -> AScope {
        AScope::Local(self.module_local_scope())
    }

    fn module_local_scope(&self) -> ALocalScope {
        ALocalScope {
            deffunc_opt: None,
            ..self.scope
        }
    }
}

/// 暗黙のシンボルの出現を解決する。
fn resolve_candidate(
    name: &str,
    scope: ALocalScope,
    public_env: &APublicEnv,
    local_env: &HashMap<ALocalScope, AEnv>,
) -> Option<AWsSymbol> {
    // ローカル環境で探す
    if let it @ Some(_) = local_env.get(&scope).and_then(|env| env.get(name)) {
        return it;
    }

    // deffuncの外からも探す。
    if scope.deffunc_opt.is_some() {
        let scope = ALocalScope {
            deffunc_opt: None,
            ..scope
        };
        if let it @ Some(_) = local_env.get(&scope).and_then(|env| env.get(name)) {
            return it;
        }
    }

    // globalで探す。
    public_env.resolve(name, scope.is_outside_module())
}

const DEF_SITE: bool = true;
const USE_SITE: bool = false;

fn add_symbol(kind: ASymbolKind, name: &PToken, def_site: bool, ctx: &mut Ctx) {
    // 新しいシンボルを登録する。
    let symbol = ASymbol::new(ctx.symbols.len());

    let mut symbol_data = ASymbolData {
        kind,
        name: name.body.text.clone(),
        def_sites: vec![],
        use_sites: vec![],
        leader: name.clone(),
        scope: ctx.module_scope(),
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
    let name = name.body.text.clone();
    let defined_scope = ctx.module_local_scope();
    let defined_env = if defined_scope.is_outside_module() {
        &mut ctx.public.env.toplevel
    } else {
        ctx.env.entry(defined_scope).or_default()
    };
    defined_env.insert(name, ws_symbol);
}

fn on_symbol_def(name: &PToken, ctx: &mut Ctx) {
    match resolve_candidate(name.body_text(), ctx.scope, &ctx.public.env, &ctx.env) {
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
    match resolve_candidate(name.body_text(), ctx.scope, &ctx.public.env, &ctx.env) {
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
        PStmt::Module(PModuleStmt { stmts, .. }) => {
            ctx.module_len += 1;
            let module = AModule::from(ctx.module_len);

            let parent_scope = replace(
                &mut ctx.scope,
                ALocalScope {
                    deffunc_opt: None,
                    module_opt: Some(module),
                },
            );

            for stmt in stmts {
                on_stmt(stmt, ctx);
            }

            ctx.scope = parent_scope;
        }
        PStmt::Const(_)
        | PStmt::Define(_)
        | PStmt::Enum(_)
        | PStmt::UseLib(_)
        | PStmt::LibFunc(_)
        | PStmt::UseCom(_)
        | PStmt::ComFunc(_)
        | PStmt::RegCmd(_)
        | PStmt::Cmd(_)
        | PStmt::Global(_)
        | PStmt::Include(_)
        | PStmt::UnknownPreProc(_) => {}
    }
}

#[derive(Debug, Default)]
pub(crate) struct AAnalysis {
    pub(crate) symbols: Vec<ASymbolData>,

    /// 解析前にあったシンボルの個数。
    preproc_symbol_len: usize,
}

pub(crate) fn analyze_var_def(
    doc: ADoc,
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

        match symbol.scope {
            AScope::Local(scope) if !scope.is_public() => {
                local_env
                    .entry(scope)
                    .or_default()
                    .insert(symbol.name.clone(), ws_symbol);
            }
            AScope::Local(_) | AScope::Global => {}
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
