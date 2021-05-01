// 構文木を辿ってプロプロセッサ命令に関する情報を集める。

use super::{a_scope::*, a_symbol::*};
use crate::{parse::*, token::TokenKind};
use std::mem::replace;

#[derive(Default)]
struct Ctx {
    symbols: Vec<ASymbolData>,
    scope: ALocalScope,
    modules: Vec<AModuleData>,
    deffunc_len: usize,
}

impl Ctx {
    fn deffunc_scope(&self) -> AScope {
        AScope::Local(self.scope)
    }

    fn module_scope(&self) -> AScope {
        AScope::Local(ALocalScope {
            deffunc_opt: None,
            ..self.scope
        })
    }

    fn privacy_scope_or_local(&self, privacy_opt: &Option<(PPrivacy, PToken)>) -> AScope {
        match privacy_opt {
            Some((PPrivacy::Global, _)) => AScope::Global,
            _ => self.module_scope(),
        }
    }

    fn privacy_scope_or_global(&self, privacy_opt: &Option<(PPrivacy, PToken)>) -> AScope {
        match privacy_opt {
            Some((PPrivacy::Local, _)) => self.module_scope(),
            _ => AScope::Global,
        }
    }

    fn add_symbol(&mut self, kind: ASymbolKind, leader: &PToken, name: &PToken, scope: AScope) {
        add_symbol(kind, leader, name, scope, &mut self.symbols);
    }
}

fn add_symbol(
    kind: ASymbolKind,
    // 構文ノードの先頭のトークン
    leader: &PToken,
    name: &PToken,
    scope: AScope,
    symbols: &mut Vec<ASymbolData>,
) {
    symbols.push(ASymbolData {
        kind,
        name: name.body.text.clone(),
        def_sites: vec![name.body.loc.clone()],
        use_sites: vec![],
        leader: leader.clone(),
        scope,
    });
}

fn on_stmt(stmt: &PStmt, ctx: &mut Ctx) {
    match stmt {
        PStmt::Label(PLabel { star, name_opt }) => {
            if let Some(name) = name_opt {
                ctx.add_symbol(ASymbolKind::Label, star, name, ctx.module_scope());
            }
        }
        PStmt::Assign(_) | PStmt::Command(_) | PStmt::Invoke(_) => {}
        PStmt::Const(PConstStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let scope = ctx.privacy_scope_or_local(privacy_opt);
                ctx.add_symbol(ASymbolKind::Const, hash, name, scope);
            }
        }
        PStmt::Define(PDefineStmt {
            hash,
            privacy_opt,
            ctype_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let scope = ctx.privacy_scope_or_local(privacy_opt);
                let ctype = ctype_opt.is_some();
                ctx.add_symbol(ASymbolKind::Macro { ctype }, hash, name, scope);
            }
        }
        PStmt::Enum(PEnumStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let scope = ctx.privacy_scope_or_local(privacy_opt);
                ctx.add_symbol(ASymbolKind::Enum, hash, name, scope);
            }
        }
        PStmt::DefFunc(PDefFuncStmt {
            hash,
            keyword: _,
            kind,
            privacy_opt,
            name_opt,
            onexit_opt,
            params,
            stmts,
            behind: _,
            ..
        }) => {
            ctx.deffunc_len += 1;
            let deffunc = ADefFunc::new(ctx.deffunc_len);
            // let deffunc = ADefFunc::new(ax.deffuncs.len());
            // ax.deffuncs.push(ADefFuncData {
            //     kind: *kind,
            //     name_opt: None,
            //     keyword_loc: keyword.body.loc.clone(),
            //     content_loc: hash.body.loc.unite(behind),
            // });

            let kind = match *kind {
                PDefFuncKind::DefFunc => ASymbolKind::DefFunc,
                PDefFuncKind::DefCFunc => ASymbolKind::DefCFunc,
                PDefFuncKind::ModInit | PDefFuncKind::ModTerm | PDefFuncKind::ModFunc => {
                    ASymbolKind::ModFunc
                }
                PDefFuncKind::ModCFunc => ASymbolKind::ModCFunc,
            };

            if let Some(name) = name_opt {
                // ax.deffuncs[deffunc.get()].name_opt = Some(name.body.text.clone());

                if onexit_opt.is_none() {
                    let scope = ctx.privacy_scope_or_global(privacy_opt);
                    ctx.add_symbol(kind, hash, name, scope);
                }
            }

            let parent_deffunc = replace(&mut ctx.scope.deffunc_opt, Some(deffunc));

            for param in params {
                if let Some(name) = &param.name_opt {
                    let param_ty = param.param_ty_opt.as_ref().map(|&(t, _)| t);
                    ctx.add_symbol(
                        ASymbolKind::Param(param_ty),
                        hash,
                        name,
                        ctx.deffunc_scope(),
                    );
                }
            }

            for stmt in stmts {
                on_stmt(stmt, ctx);
            }

            ctx.scope.deffunc_opt = parent_deffunc;
        }
        PStmt::UseLib(_) => {}
        PStmt::LibFunc(PLibFuncStmt {
            hash,
            privacy_opt,
            name_opt,
            onexit_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                if onexit_opt.is_none() {
                    let scope = ctx.privacy_scope_or_local(privacy_opt);
                    ctx.add_symbol(ASymbolKind::LibFunc, hash, name, scope);
                }
            }
        }
        PStmt::UseCom(PUseComStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let scope = ctx.privacy_scope_or_local(privacy_opt);
                ctx.add_symbol(ASymbolKind::ComInterface, hash, name, scope);
            }
        }
        PStmt::ComFunc(PComFuncStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let scope = ctx.privacy_scope_or_global(privacy_opt);
                ctx.add_symbol(ASymbolKind::ComFunc, hash, name, scope);
            }
        }
        PStmt::RegCmd(_) => {}
        PStmt::Cmd(PCmdStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let scope = ctx.privacy_scope_or_local(privacy_opt);
                ctx.add_symbol(ASymbolKind::PluginCmd, hash, name, scope);
            }
        }
        PStmt::Module(PModuleStmt {
            hash,
            keyword,
            name_opt,
            fields,
            stmts,
            behind,
            ..
        }) => {
            let module = AModule::from(ctx.modules.len());
            ctx.modules.push(AModuleData {
                name_opt: None,
                keyword_loc: keyword.body.loc.clone(),
                content_loc: hash.body.loc.unite(&behind),
            });

            let parent_scope = replace(
                &mut ctx.scope,
                ALocalScope {
                    module_opt: Some(module),
                    deffunc_opt: None,
                },
            );

            if let Some(name) = name_opt {
                // ax.modules[module.get()].name_opt = Some(name.body.text.clone());

                match name.kind() {
                    TokenKind::Ident => {
                        ctx.add_symbol(ASymbolKind::Module, hash, name, AScope::Global);
                    }
                    TokenKind::Str => {
                        // FIXME: 識別子として有効な文字列ならシンボルとして登録できる。
                    }
                    _ => {}
                }
            }

            for field in fields.iter().filter_map(|param| param.name_opt.as_ref()) {
                ctx.add_symbol(ASymbolKind::Field, field, field, ctx.module_scope());
            }

            for stmt in stmts {
                on_stmt(stmt, ctx);
            }

            ctx.scope = parent_scope;
        }
        PStmt::Global(_) => {}
        PStmt::Include(_) => {}
        PStmt::UnknownPreProc(_) => {}
    }
}

pub(crate) struct PreprocAnalysisResult {
    pub(crate) symbols: Vec<ASymbolData>,
    pub(crate) modules: Vec<AModuleData>,
}

pub(crate) fn analyze_preproc(root: &PRoot) -> PreprocAnalysisResult {
    let mut ctx = Ctx::default();

    for stmt in &root.stmts {
        on_stmt(stmt, &mut ctx);
    }

    let Ctx {
        symbols, modules, ..
    } = ctx;

    PreprocAnalysisResult { symbols, modules }
}
