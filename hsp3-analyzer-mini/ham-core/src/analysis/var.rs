// 変数の定義・使用箇所の列挙。

use super::*;
use crate::parse::*;

struct Ctx<'a> {
    module_map: &'a ModuleMap,
    public_env: &'a mut PublicEnv,
    ns_env: &'a mut NsEnv,

    // 他のドキュメントのシンボルの定義・使用箇所を記録するもの。
    public_def_sites: &'a mut Vec<(SymbolRc, Loc)>,
    public_use_sites: &'a mut Vec<(SymbolRc, Loc)>,

    doc: DocId,

    /// ドキュメント内のシンボル
    new_symbols: Vec<SymbolRc>,

    /// ドキュメント内の環境
    local_env: HashMap<LocalScope, SymbolEnv>,

    deffunc_len: usize,
    module_len: usize,
    scope: LocalScope,
}

const DEF_SITE: bool = true;
const USE_SITE: bool = false;

fn add_symbol(kind: HspSymbolKind, name: &PToken, def_site: bool, ctx: &mut Ctx) {
    let NameScopeNsTriple {
        basename,
        scope_opt,
        ns_opt,
    } = resolve_name_scope_ns_for_def(
        &name.body.text,
        ImportMode::Local,
        &ctx.scope,
        ctx.module_map,
    );

    let symbol = DefInfo::Name {
        kind,
        name: name.clone(),
        basename: basename.clone(),
        scope_opt: scope_opt.clone(),
        ns_opt: ns_opt.clone(),
    }
    .into_symbol();
    ctx.new_symbols.push(symbol.clone());

    if def_site {
        ctx.public_def_sites.push((symbol.clone(), name.body.loc));
    } else {
        ctx.public_use_sites.push((symbol.clone(), name.body.loc));
    }

    import_symbol_to_env(
        &symbol,
        basename,
        scope_opt,
        ns_opt,
        &mut ctx.public_env,
        &mut ctx.ns_env,
        &mut ctx.local_env,
    );
}

fn on_symbol_def(name: &PToken, ctx: &mut Ctx) {
    match resolve_implicit_symbol(
        &name.body.text,
        &ctx.scope,
        &ctx.public_env,
        &ctx.ns_env,
        &ctx.local_env,
        ctx.module_map,
    ) {
        Some(symbol) => {
            ctx.public_def_sites.push((symbol, name.body.loc));
        }
        None => add_symbol(HspSymbolKind::StaticVar, name, DEF_SITE, ctx),
    }
}

fn on_symbol_use(name: &PToken, is_var: bool, ctx: &mut Ctx) {
    match resolve_implicit_symbol(
        &name.body.text,
        &ctx.scope,
        &ctx.public_env,
        &ctx.ns_env,
        &ctx.local_env,
        &ctx.module_map,
    ) {
        Some(symbol) => {
            ctx.public_use_sites.push((symbol, name.body.loc));
        }
        None => {
            let kind = if is_var {
                HspSymbolKind::StaticVar
            } else {
                HspSymbolKind::Unresolved
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
                add_symbol(HspSymbolKind::Label, name, DEF_SITE, ctx);
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
        PStmt::If(stmt) => {
            on_expr_opt(stmt.cond_opt.as_ref(), ctx);

            for stmt in &stmt.body.outer_stmts {
                on_stmt(stmt, ctx);
            }
            for stmt in &stmt.body.inner_stmts {
                on_stmt(stmt, ctx);
            }
            for stmt in &stmt.alt.outer_stmts {
                on_stmt(stmt, ctx);
            }
            for stmt in &stmt.alt.inner_stmts {
                on_stmt(stmt, ctx);
            }
        }
        PStmt::DefFunc(PDefFuncStmt { stmts, .. }) => {
            let deffunc = DefFuncKey::new(ctx.doc, ctx.deffunc_len);
            ctx.deffunc_len += 1;

            let parent_deffunc = replace(&mut ctx.scope.deffunc_opt, Some(deffunc));

            for stmt in stmts {
                on_stmt(stmt, ctx);
            }

            ctx.scope.deffunc_opt = parent_deffunc;
        }
        PStmt::Module(PModuleStmt { stmts, .. }) => {
            let module = ModuleKey::new(ctx.doc, ctx.module_len);
            ctx.module_len += 1;

            let parent_scope = replace(
                &mut ctx.scope,
                LocalScope {
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

pub(crate) fn analyze_var_def(
    doc: DocId,
    root: &PRoot,
    preproc_symbols: &[SymbolRc],
    module_map: &ModuleMap,
    public_env: &mut PublicEnv,
    ns_env: &mut NsEnv,
    new_symbols: &mut Vec<SymbolRc>,
    def_sites: &mut Vec<(SymbolRc, Loc)>,
    use_sites: &mut Vec<(SymbolRc, Loc)>,
) {
    let mut local_env = HashMap::new();
    extend_local_env_from_symbols(preproc_symbols, &mut local_env);

    let mut ctx = Ctx {
        public_env,
        ns_env,
        module_map,
        public_def_sites: def_sites,
        public_use_sites: use_sites,
        doc,
        new_symbols: vec![],
        local_env,
        deffunc_len: 0,
        module_len: 0,
        scope: LocalScope::default(),
    };

    for stmt in &root.stmts {
        on_stmt(stmt, &mut ctx);
    }

    new_symbols.extend(ctx.new_symbols);
}
