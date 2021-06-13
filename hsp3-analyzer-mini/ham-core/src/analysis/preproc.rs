// 構文木を辿ってプロプロセッサ命令に関する情報を集める。

use super::*;
use crate::parse::*;

pub(crate) struct IncludeGuard {
    /// `#if` と `#define` の範囲
    pub(crate) loc: Loc,
}

pub(crate) fn find_include_guard(root: &PRoot) -> Option<IncludeGuard> {
    let s1 = root.stmts.get(0)?;
    let s2 = root.stmts.get(1)?;

    let (pp_if_stmt, define_stmt) = match (s1, s2) {
        (PStmt::UnknownPreProc(i), PStmt::Define(d)) => (i, d),
        _ => return None,
    };

    let name = define_stmt.name_opt.as_ref()?.body_text();
    if !pp_if_stmt
        .tokens
        .iter()
        .any(|t| t.body_text().contains(name))
    {
        return None;
    }

    // endif は略

    let doc = define_stmt.hash.body.loc.doc;
    let range = s1.compute_range().join(s2.compute_range());

    Some(IncludeGuard {
        loc: Loc::new(doc, range),
    })
}

pub(crate) struct ASignatureData {
    pub(crate) name: RcStr,
    pub(crate) params: Vec<(Option<PParamTy>, Option<RcStr>, Option<String>)>,
}

#[derive(Default)]
struct Ctx {
    doc: DocId,
    symbols: Vec<SymbolRc>,
    include_guard: Option<IncludeGuard>,
    includes: Vec<(RcStr, Loc)>,
    scope: LocalScope,
    module_map: ModuleMap,
    deffunc_map: DefFuncMap,
    module_len: usize,
    deffunc_len: usize,
}

impl Ctx {
    fn privacy_scope_or_local(&self, privacy_opt: &Option<(PPrivacy, PToken)>) -> ImportMode {
        match privacy_opt {
            Some((PPrivacy::Global, _)) => ImportMode::Global,
            _ => ImportMode::Local,
        }
    }

    fn privacy_scope_or_global(&self, privacy_opt: &Option<(PPrivacy, PToken)>) -> ImportMode {
        match privacy_opt {
            Some((PPrivacy::Local, _)) => ImportMode::Local,
            _ => ImportMode::Global,
        }
    }

    fn add_symbol(
        &mut self,
        kind: HspSymbolKind,
        leader: &PToken,
        name: &PToken,
        def: ImportMode,
    ) -> SymbolRc {
        add_symbol(
            kind,
            leader,
            name,
            def,
            &self.scope,
            &self.module_map,
            &mut self.symbols,
        )
    }
}

fn add_symbol(
    kind: HspSymbolKind,
    // 構文ノードの先頭のトークン
    leader: &PToken,
    name: &PToken,
    def: ImportMode,
    local: &LocalScope,
    module_map: &ModuleMap,
    symbols: &mut Vec<SymbolRc>,
) -> SymbolRc {
    let NameScopeNsTriple {
        basename,
        scope_opt,
        ns_opt,
    } = resolve_name_scope_ns_for_def(&name.body.text, def, local, module_map);

    let symbol = SymbolRc::from(ASymbolData {
        doc: leader.body.loc.doc,
        kind,
        name: basename,
        leader_opt: Some(leader.clone()),
        scope_opt,
        ns_opt,

        details_opt: None,
        preproc_def_site_opt: Some(name.body.loc),
        signature_opt: Default::default(),
        linked_symbol_opt: Default::default(),
    });
    symbols.push(symbol.clone());
    symbol
}

fn on_block(block: &PBlock, ctx: &mut Ctx) {
    for stmt in &block.outer_stmts {
        on_stmt(stmt, ctx);
    }
    for stmt in &block.inner_stmts {
        on_stmt(stmt, ctx);
    }
}

fn on_stmt(stmt: &PStmt, ctx: &mut Ctx) {
    match stmt {
        PStmt::Label(PLabel { star, name_opt }) => {
            if let Some(name) = name_opt {
                ctx.add_symbol(HspSymbolKind::Label, star, name, ImportMode::Local);
            }
        }
        PStmt::Assign(_) | PStmt::Command(_) | PStmt::Invoke(_) => {}
        PStmt::If(stmt) => {
            on_block(&stmt.body, ctx);
            on_block(&stmt.alt, ctx);
        }
        PStmt::Const(PConstStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let scope = ctx.privacy_scope_or_local(privacy_opt);
                ctx.add_symbol(HspSymbolKind::Const, hash, name, scope);
            }
        }
        PStmt::Define(PDefineStmt {
            hash,
            privacy_opt,
            ctype_opt,
            name_opt,
            ..
        }) => {
            if ctx.include_guard.as_ref().map_or(false, |g| {
                g.loc.is_touched(hash.body.loc.doc, hash.body_pos16())
            }) {
                return;
            }

            if let Some(name) = name_opt {
                let scope = ctx.privacy_scope_or_local(privacy_opt);
                let ctype = ctype_opt.is_some();
                ctx.add_symbol(HspSymbolKind::Macro { ctype }, hash, name, scope);
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
                ctx.add_symbol(HspSymbolKind::Enum, hash, name, scope);
            }
        }
        PStmt::DefFunc(stmt) => {
            let PDefFuncStmt {
                hash,
                kind,
                privacy_opt,
                name_opt,
                onexit_opt,
                params,
                stmts,
                behind,
                ..
            } = stmt;

            let deffunc = DefFuncKey::new(ctx.doc, ctx.deffunc_len);
            ctx.deffunc_len += 1;
            ctx.deffunc_map.insert(
                deffunc,
                DefFuncData {
                    content_loc: hash.body.loc.unite(behind),
                },
            );

            let mut symbol_opt = None;

            let kind = to_symbol_kind(*kind);

            if let Some(name) = name_opt {
                if onexit_opt.is_none() {
                    let scope = ctx.privacy_scope_or_global(privacy_opt);
                    symbol_opt = Some(ctx.add_symbol(kind, hash, name, scope));
                }
            }

            if let Some(symbol) = symbol_opt {
                if let Some(data) = new_signature_data_for_deffunc(stmt) {
                    *symbol.signature_opt.borrow_mut() = Some(Rc::new(data));
                }
            }

            let parent_deffunc = replace(&mut ctx.scope.deffunc_opt, Some(deffunc));

            for param in params {
                if let Some(name) = &param.name_opt {
                    let param_ty = param.param_ty_opt.as_ref().map(|&(t, _)| t);
                    ctx.add_symbol(
                        HspSymbolKind::Param(param_ty),
                        hash,
                        name,
                        ImportMode::Param,
                    );
                }
            }

            for stmt in stmts {
                on_stmt(stmt, ctx);
            }

            ctx.scope.deffunc_opt = parent_deffunc;
        }
        PStmt::UseLib(_) => {}
        PStmt::LibFunc(stmt) => {
            let PLibFuncStmt {
                hash,
                privacy_opt,
                name_opt,
                onexit_opt,
                ..
            } = stmt;
            let mut symbol_opt = None;

            if let Some(name) = name_opt {
                if onexit_opt.is_none() {
                    let scope = ctx.privacy_scope_or_local(privacy_opt);
                    symbol_opt = Some(ctx.add_symbol(HspSymbolKind::LibFunc, hash, name, scope));
                }
            }

            if let Some(symbol) = symbol_opt {
                if let Some(signature_data) = new_signature_data_for_lib_func(stmt) {
                    *symbol.signature_opt.borrow_mut() = Some(Rc::new(signature_data));
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
                ctx.add_symbol(HspSymbolKind::ComInterface, hash, name, scope);
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
                ctx.add_symbol(HspSymbolKind::ComFunc, hash, name, scope);
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
                ctx.add_symbol(HspSymbolKind::PluginCmd, hash, name, scope);
            }
        }
        PStmt::Module(PModuleStmt {
            hash,
            name_opt,
            fields,
            stmts,
            behind,
            ..
        }) => {
            let module = ModuleKey::new(ctx.doc, ctx.module_len);
            ctx.module_len += 1;

            let module_rc = {
                let name_opt = name_opt
                    .as_ref()
                    .and_then(|t| module_name_as_ident(&t.body));
                let content_loc = hash.body.loc.unite(&behind);
                ModuleRc::new(ModuleData {
                    name_opt,
                    content_loc,
                })
            };
            ctx.module_map.insert(module, module_rc);

            let parent_scope = replace(
                &mut ctx.scope,
                LocalScope {
                    module_opt: Some(module),
                    deffunc_opt: None,
                },
            );

            if let Some(name) = name_opt {
                ctx.add_symbol(HspSymbolKind::Module, hash, name, ImportMode::Global);
            }

            for field in fields.iter().filter_map(|param| param.name_opt.as_ref()) {
                ctx.add_symbol(HspSymbolKind::Field, field, field, ImportMode::Local);
            }

            for stmt in stmts {
                on_stmt(stmt, ctx);
            }

            ctx.scope = parent_scope;
        }
        PStmt::Global(_) => {}
        PStmt::Include(stmt) => {
            if let Some(file_path) = &stmt.file_path_opt {
                if file_path.body.kind == TokenKind::Str {
                    let mut text = file_path.body.text.clone();

                    // クオートを外す。
                    let l = if text.starts_with("\"") { 1 } else { 0 };
                    let r = text.len() - (if text.ends_with("\"") { 1 } else { 0 });
                    text = text.slice(l, r);

                    // 標準化する。
                    text = text.replace("\\\\", "/").to_ascii_lowercase().into();

                    let loc = stmt.hash.body.loc.unite(&file_path.behind());
                    ctx.includes.push((text, loc));
                }
            }
        }
        PStmt::UnknownPreProc(_) => {}
    }
}

fn to_symbol_kind(kind: PDefFuncKind) -> HspSymbolKind {
    match kind {
        PDefFuncKind::DefFunc => HspSymbolKind::DefFunc,
        PDefFuncKind::DefCFunc => HspSymbolKind::DefCFunc,
        PDefFuncKind::ModInit | PDefFuncKind::ModTerm | PDefFuncKind::ModFunc => {
            HspSymbolKind::ModFunc
        }
        PDefFuncKind::ModCFunc => HspSymbolKind::ModCFunc,
    }
}

fn new_signature_data_for_lib_func(stmt: &PLibFuncStmt) -> Option<ASignatureData> {
    let name = stmt.name_opt.as_ref()?.body.text.clone();

    let params = stmt
        .params
        .iter()
        .filter_map(|param| {
            let ty_opt = match param.param_ty_opt {
                Some((ty, _)) if !ty.take_arg() => return None,
                Some((ty, _)) => Some(ty),
                _ => None,
            };
            let name_opt = param.name_opt.as_ref().map(|name| name.body.text.clone());
            Some((ty_opt, name_opt, None))
        })
        .collect::<Vec<_>>();

    Some(ASignatureData { name, params })
}

fn new_signature_data_for_deffunc(stmt: &PDefFuncStmt) -> Option<ASignatureData> {
    let take_modvar = match stmt.kind {
        PDefFuncKind::DefFunc | PDefFuncKind::DefCFunc => false,
        PDefFuncKind::ModFunc | PDefFuncKind::ModCFunc => true,
        PDefFuncKind::ModInit | PDefFuncKind::ModTerm => return None,
    };

    let name = stmt.name_opt.as_ref()?.body.text.clone();

    let mut params = vec![];

    if take_modvar {
        params.push((Some(PParamTy::Modvar), Some("thismod".into()), None));
    }

    for param in &stmt.params {
        let ty_opt = match param.param_ty_opt {
            Some((ty, _)) if !ty.take_arg() => continue,
            Some((ty, _)) => Some(ty),
            _ => None,
        };
        let name_opt = param.name_opt.as_ref().map(|name| name.body.text.clone());

        params.push((ty_opt, name_opt, None));
    }

    Some(ASignatureData { name, params })
}

pub(crate) struct PreprocAnalysisResult {
    pub(crate) symbols: Vec<SymbolRc>,
    pub(crate) include_guard: Option<IncludeGuard>,
    pub(crate) includes: Vec<(RcStr, Loc)>,
    pub(crate) module_map: ModuleMap,
    pub(crate) deffunc_map: HashMap<DefFuncKey, DefFuncData>,
}

pub(crate) fn analyze_preproc(doc: DocId, root: &PRoot) -> PreprocAnalysisResult {
    let mut ctx = Ctx::default();
    ctx.doc = doc;
    ctx.include_guard = find_include_guard(root);

    for stmt in &root.stmts {
        on_stmt(stmt, &mut ctx);
    }

    let Ctx {
        symbols,
        include_guard,
        includes,
        module_map,
        deffunc_map,
        ..
    } = ctx;

    PreprocAnalysisResult {
        symbols,
        include_guard,
        includes,
        module_map,
        deffunc_map,
    }
}
