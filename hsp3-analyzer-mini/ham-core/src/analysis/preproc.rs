// 構文木を辿ってプロプロセッサ命令に関する情報を集める。

use super::{a_scope::*, a_symbol::*};
use crate::{
    analysis::var::resolve_symbol_scope,
    parse::*,
    source::{DocId, Loc},
    utils::rc_str::RcStr,
};
use std::{collections::HashMap, mem::replace, rc::Rc};

pub(crate) struct ASignatureData {
    pub(crate) name: RcStr,
    pub(crate) params: Vec<(PParamTy, Option<RcStr>)>,
    #[allow(unused)]
    pub(crate) command: bool,
    #[allow(unused)]
    pub(crate) func: bool,
    #[allow(unused)]
    pub(crate) def_site: Loc,
}

#[derive(Default)]
struct Ctx {
    doc: DocId,
    symbols: Vec<ASymbolData>,
    scope: ALocalScope,
    modules: HashMap<AModule, AModuleData>,
    signatures: HashMap<ASymbol, Rc<ASignatureData>>,
    module_len: usize,
    deffunc_len: usize,
}

impl Ctx {
    fn privacy_scope_or_local(&self, privacy_opt: &Option<(PPrivacy, PToken)>) -> ADefScope {
        match privacy_opt {
            Some((PPrivacy::Global, _)) => ADefScope::Global,
            _ => ADefScope::Local,
        }
    }

    fn privacy_scope_or_global(&self, privacy_opt: &Option<(PPrivacy, PToken)>) -> ADefScope {
        match privacy_opt {
            Some((PPrivacy::Local, _)) => ADefScope::Local,
            _ => ADefScope::Global,
        }
    }

    fn add_symbol(&mut self, kind: ASymbolKind, leader: &PToken, name: &PToken, def: ADefScope) {
        add_symbol(kind, leader, name, def, &self.scope, &mut self.symbols);
    }
}

fn add_symbol(
    kind: ASymbolKind,
    // 構文ノードの先頭のトークン
    leader: &PToken,
    name: &PToken,
    def: ADefScope,
    local: &ALocalScope,
    symbols: &mut Vec<ASymbolData>,
) {
    let (basename, scope_opt, ns_opt) = resolve_symbol_scope(&name.body.text, def, local);

    symbols.push(ASymbolData {
        kind,
        name: basename,
        def_sites: vec![name.body.loc],
        use_sites: vec![],
        leader: leader.clone(),
        scope_opt,
        ns_opt,
    });
}

fn on_stmt(stmt: &PStmt, ctx: &mut Ctx) {
    match stmt {
        PStmt::Label(PLabel { star, name_opt }) => {
            if let Some(name) = name_opt {
                ctx.add_symbol(ASymbolKind::Label, star, name, ADefScope::Local);
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
        PStmt::DefFunc(stmt) => {
            let PDefFuncStmt {
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
            } = stmt;

            ctx.deffunc_len += 1;
            let deffunc = ADefFunc::new(ctx.deffunc_len);
            let mut symbol_opt = None;
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
                    symbol_opt = Some(ASymbol::new(ctx.symbols.len()));
                    ctx.add_symbol(kind, hash, name, scope);
                }
            }

            if let Some(symbol) = symbol_opt {
                if let Some(data) = new_signature_data(stmt) {
                    ctx.signatures.insert(symbol, Rc::new(data));
                }
            }

            let parent_deffunc = replace(&mut ctx.scope.deffunc_opt, Some(deffunc));

            for param in params {
                if let Some(name) = &param.name_opt {
                    let param_ty = param.param_ty_opt.as_ref().map(|&(t, _)| t);
                    ctx.add_symbol(ASymbolKind::Param(param_ty), hash, name, ADefScope::Param);
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
            let module = AModule::new(ctx.doc, &mut ctx.module_len, name_opt);

            ctx.modules.insert(
                module.clone(),
                AModuleData {
                    keyword_loc: keyword.body.loc.clone(),
                    content_loc: hash.body.loc.unite(&behind),
                },
            );

            let parent_scope = replace(
                &mut ctx.scope,
                ALocalScope {
                    module_opt: Some(module),
                    deffunc_opt: None,
                },
            );

            if let Some(name) = name_opt {
                ctx.add_symbol(ASymbolKind::Module, hash, name, ADefScope::Global);
            }

            for field in fields.iter().filter_map(|param| param.name_opt.as_ref()) {
                ctx.add_symbol(ASymbolKind::Field, field, field, ADefScope::Local);
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

fn new_signature_data(stmt: &PDefFuncStmt) -> Option<ASignatureData> {
    let (command, func, take_modvar) = match stmt.kind {
        PDefFuncKind::DefFunc => (true, false, false),
        PDefFuncKind::DefCFunc => (false, true, false),
        PDefFuncKind::ModFunc => (true, false, true),
        PDefFuncKind::ModCFunc => (false, true, false),
        PDefFuncKind::ModInit | PDefFuncKind::ModTerm => return None,
    };

    let name = stmt.name_opt.as_ref()?.body.text.clone();

    let mut params = vec![];

    if take_modvar {
        params.push((PParamTy::Modvar, Some("thismod".into())));
    }

    for param in &stmt.params {
        let ty = match param.param_ty_opt {
            Some((ty, _)) if ty.take_arg() => ty,
            _ => continue,
        };
        let name_opt = param.name_opt.as_ref().map(|name| name.body.text.clone());

        params.push((ty, name_opt));
    }

    Some(ASignatureData {
        name,
        params,
        command,
        func,
        def_site: stmt.hash.body.loc,
    })
}

pub(crate) struct PreprocAnalysisResult {
    pub(crate) symbols: Vec<ASymbolData>,
    pub(crate) modules: HashMap<AModule, AModuleData>,
    pub(crate) signatures: HashMap<ASymbol, Rc<ASignatureData>>,
}

pub(crate) fn analyze_preproc(doc: DocId, root: &PRoot) -> PreprocAnalysisResult {
    let mut ctx = Ctx::default();
    ctx.doc = doc;

    for stmt in &root.stmts {
        on_stmt(stmt, &mut ctx);
    }

    let Ctx {
        symbols,
        modules,
        signatures,
        ..
    } = ctx;

    PreprocAnalysisResult {
        symbols,
        modules,
        signatures,
    }
}
