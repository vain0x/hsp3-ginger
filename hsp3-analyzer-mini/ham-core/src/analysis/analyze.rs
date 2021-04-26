use super::{
    a_scope::{ADefFunc, ADefFuncData, ALocalScope, AModule, AModuleData},
    a_symbol::{ASymbolData, AWsSymbol},
    integrate::APublicEnv,
    ADoc, ALoc, APos, AScope, ASymbol, ASymbolKind,
};
use crate::{
    analysis::comment::str_is_ornament_comment,
    parse::*,
    token::{TokenData, TokenKind},
    utils::rc_str::RcStr,
};
use std::{
    collections::HashMap,
    mem::{replace, take},
};

#[derive(Copy, Clone, Debug)]
enum ADefCandidateKind {
    VarOrArray,
    ArrayOrFunc,
}

#[derive(Debug)]
struct ADefCandidateData {
    kind: ADefCandidateKind,
    name: RcStr,
    loc: ALoc,
    scope: ALocalScope,
}

#[derive(Copy, Clone, Debug)]
enum AUseCandidateKind {
    Label,
    Command,
    VarOrArray,
    ArrayOrFunc,
}

#[derive(Debug)]
struct AUseCandidateData {
    kind: AUseCandidateKind,
    name: RcStr,
    loc: ALoc,
    scope: ALocalScope,
}

/// Analysis context.
#[derive(Default)]
struct Ax {
    eof_loc: ALoc,
    symbols: Vec<ASymbolData>,
    def_candidates: Vec<ADefCandidateData>,
    use_candidates: Vec<AUseCandidateData>,
    deffuncs: Vec<ADefFuncData>,
    deffunc_opt: Option<ADefFunc>,
    modules: Vec<AModuleData>,
    module_opt: Option<AModule>,
}

impl Ax {
    fn new() -> Self {
        Self::default()
    }

    fn current_local_scope(&self) -> ALocalScope {
        ALocalScope {
            deffunc_opt: self.deffunc_opt,
            module_opt: self.module_opt,
        }
    }

    fn current_scope(&self) -> AScope {
        AScope::Local(self.current_local_scope())
    }

    fn add_symbol(
        &mut self,
        kind: ASymbolKind,
        token: &PToken,
        privacy: PPrivacy,
        definer: &PToken,
    ) -> ASymbol {
        let scope = match privacy {
            PPrivacy::Global => AScope::Global,
            PPrivacy::Local => self.current_scope(),
        };
        add_symbol(kind, token, definer, scope, &mut self.symbols)
    }
}

fn add_symbol(
    kind: ASymbolKind,
    token: &PToken,
    definer: &PToken,
    scope: AScope,
    symbols: &mut Vec<ASymbolData>,
) -> ASymbol {
    let comments = definer
        .leading
        .iter()
        .filter_map(|t| {
            if t.kind == TokenKind::Comment && !str_is_ornament_comment(&t.text) {
                Some(t.text.clone())
            } else {
                None
            }
        })
        .collect();

    let symbol_id = symbols.len();
    symbols.push(ASymbolData {
        kind,
        name: token.body.text.clone(),
        def_sites: vec![token.body.loc.clone()],
        use_sites: vec![],
        comments,
        scope,
    });
    ASymbol::new(symbol_id)
}

fn get_privacy_or_local(privacy_opt: &Option<(PPrivacy, PToken)>) -> PPrivacy {
    match privacy_opt {
        Some((privacy, _)) => *privacy,
        None => PPrivacy::Local,
    }
}

fn on_symbol_def(name: &PToken, kind: ADefCandidateKind, ax: &mut Ax) {
    ax.def_candidates.push(ADefCandidateData {
        kind,
        name: name.body.text.clone(),
        loc: name.body.loc.clone(),
        scope: ax.current_local_scope(),
    });
}

fn on_symbol_use(name: &PToken, kind: AUseCandidateKind, ax: &mut Ax) {
    ax.use_candidates.push(AUseCandidateData {
        kind,
        name: name.body.text.clone(),
        loc: name.body.loc.clone(),
        scope: ax.current_local_scope(),
    });
}

fn on_compound_def(compound: &PCompound, ax: &mut Ax) {
    match compound {
        PCompound::Name(name) => on_symbol_def(name, ADefCandidateKind::VarOrArray, ax),
        PCompound::Paren(PNameParen { name, args, .. }) => {
            on_symbol_def(name, ADefCandidateKind::ArrayOrFunc, ax);

            for arg in args {
                on_expr_opt(arg.expr_opt.as_ref(), ax);
            }
        }
        PCompound::Dots(PNameDot { name, args }) => {
            on_symbol_def(name, ADefCandidateKind::ArrayOrFunc, ax);

            for arg in args {
                on_expr_opt(arg.expr_opt.as_ref(), ax);
            }
        }
    }
}

fn on_compound_use(compound: &PCompound, ax: &mut Ax) {
    match compound {
        PCompound::Name(name) => on_symbol_use(name, AUseCandidateKind::VarOrArray, ax),
        PCompound::Paren(PNameParen { name, args, .. }) => {
            on_symbol_use(name, AUseCandidateKind::ArrayOrFunc, ax);

            for arg in args {
                on_expr_opt(arg.expr_opt.as_ref(), ax);
            }
        }
        PCompound::Dots(PNameDot { name, args }) => {
            on_symbol_use(name, AUseCandidateKind::ArrayOrFunc, ax);

            for arg in args {
                on_expr_opt(arg.expr_opt.as_ref(), ax);
            }
        }
    }
}

fn on_expr(expr: &PExpr, ax: &mut Ax) {
    match expr {
        PExpr::Literal(_) => {}
        PExpr::Label(PLabel { star: _, name_opt }) => {
            if let Some(name) = name_opt {
                on_symbol_use(name, AUseCandidateKind::Label, ax);
            }
        }
        PExpr::Compound(compound) => on_compound_use(compound, ax),
        PExpr::Paren(PParenExpr { body_opt, .. }) => on_expr_opt(body_opt.as_deref(), ax),
        PExpr::Prefix(PPrefixExpr { prefix: _, arg_opt }) => on_expr_opt(arg_opt.as_deref(), ax),
        PExpr::Infix(PInfixExpr {
            infix: _,
            left,
            right_opt,
        }) => {
            on_expr(left, ax);
            on_expr_opt(right_opt.as_deref(), ax);
        }
    }
}

fn on_expr_opt(expr_opt: Option<&PExpr>, ax: &mut Ax) {
    if let Some(expr) = expr_opt {
        on_expr(expr, ax);
    }
}

fn on_args(args: &[PArg], ax: &mut Ax) {
    for arg in args {
        on_expr_opt(arg.expr_opt.as_ref(), ax);
    }
}

fn on_stmt(stmt: &PStmt, ax: &mut Ax) {
    match stmt {
        PStmt::Label(PLabel { star, name_opt }) => {
            if let Some(name) = name_opt {
                ax.add_symbol(ASymbolKind::Label, name, PPrivacy::Local, star);
            }
        }
        PStmt::Assign(PAssignStmt {
            left,
            op_opt: _,
            args,
        }) => {
            // FIXME: def/use は演算子の種類による
            on_compound_def(left, ax);
            on_args(args, ax);
        }
        PStmt::Command(PCommandStmt {
            command,
            jump_modifier_opt: _,
            args,
        }) => {
            on_symbol_use(&command, AUseCandidateKind::Command, ax);
            on_args(&args, ax);
        }
        PStmt::Invoke(PInvokeStmt {
            left,
            arrow_opt: _,
            method_opt,
            args,
        }) => {
            on_compound_use(left, ax);
            on_expr_opt(method_opt.as_ref(), ax);
            on_args(&args, ax);
        }
        PStmt::Const(PConstStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let privacy = get_privacy_or_local(privacy_opt);
                ax.add_symbol(ASymbolKind::Const, name, privacy, hash);
            }
        }
        PStmt::Define(PDefineStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            // FIXME: ctype などをみて kind を決定する。

            if let Some(name) = name_opt {
                let privacy = get_privacy_or_local(privacy_opt);
                ax.add_symbol(ASymbolKind::Const, name, privacy, hash);
            }
        }
        PStmt::Enum(PEnumStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let privacy = get_privacy_or_local(privacy_opt);
                ax.add_symbol(ASymbolKind::Const, name, privacy, hash);
            }
        }
        PStmt::DefFunc(PDefFuncStmt {
            hash,
            keyword,
            kind,
            privacy_opt,
            name_opt,
            onexit_opt,
            params,
            stmts,
            behind,
            ..
        }) => {
            let deffunc = ADefFunc::new(ax.deffuncs.len());
            ax.deffuncs.push(ADefFuncData {
                kind: *kind,
                name_opt: None,
                keyword_loc: keyword.body.loc.clone(),
                content_loc: hash.body.loc.unite(behind),
            });

            if let Some(name) = name_opt {
                ax.deffuncs[deffunc.get()].name_opt = Some(name.body.text.clone());

                if onexit_opt.is_none() {
                    let privacy = match privacy_opt {
                        Some((privacy, _)) => *privacy,
                        None => PPrivacy::Global,
                    };
                    ax.add_symbol(ASymbolKind::CommandOrFunc, name, privacy, hash);
                }
            }

            let parent_deffunc = replace(&mut ax.deffunc_opt, Some(deffunc));

            for param in params {
                if let Some(name) = &param.name_opt {
                    ax.add_symbol(ASymbolKind::Param, name, PPrivacy::Local, hash);
                }
            }

            for stmt in stmts {
                on_stmt(stmt, ax);
            }

            ax.deffunc_opt = parent_deffunc;
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
                    let privacy = get_privacy_or_local(privacy_opt);
                    ax.add_symbol(ASymbolKind::CommandOrFunc, name, privacy, hash);
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
                let privacy = get_privacy_or_local(privacy_opt);
                ax.add_symbol(ASymbolKind::Const, name, privacy, hash);
            }
        }
        PStmt::ComFunc(PComFuncStmt {
            hash,
            privacy_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let privacy = match privacy_opt {
                    Some((privacy, _)) => *privacy,
                    None => PPrivacy::Global,
                };
                ax.add_symbol(ASymbolKind::Command, name, privacy, hash);
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
                let privacy = get_privacy_or_local(privacy_opt);
                ax.add_symbol(ASymbolKind::CommandOrFuncOrVar, name, privacy, hash);
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
            let module = AModule::from(ax.modules.len());
            ax.modules.push(AModuleData {
                name_opt: None,
                keyword_loc: keyword.body.loc.clone(),
                content_loc: hash.body.loc.unite(&behind),
            });

            let parent_deffunc_opt = take(&mut ax.deffunc_opt);
            let parent_module_opt = replace(&mut ax.module_opt, Some(module));

            if let Some(name) = name_opt {
                ax.modules[module.get()].name_opt = Some(name.body.text.clone());

                match name.kind() {
                    TokenKind::Ident => {
                        ax.add_symbol(ASymbolKind::Module, name, PPrivacy::Global, hash);
                    }
                    TokenKind::Str => {
                        // FIXME: 識別子として有効な文字列ならシンボルとして登録できる。
                    }
                    _ => {}
                }
            }

            for field in fields.iter().filter_map(|param| param.name_opt.as_ref()) {
                ax.add_symbol(ASymbolKind::Field, field, PPrivacy::Local, field);
            }

            for stmt in stmts {
                on_stmt(stmt, ax);
            }

            ax.deffunc_opt = parent_deffunc_opt;
            ax.module_opt = parent_module_opt;
        }
        PStmt::Global(_) => {}
        PStmt::Include(_) => {}
        PStmt::UnknownPreProc(_) => {}
    }
}

#[derive(Debug, Default)]
pub(crate) struct AAnalysis {
    symbols: Vec<ASymbolData>,
    def_candidates: Vec<ADefCandidateData>,
    use_candidates: Vec<AUseCandidateData>,
    deffuncs: Vec<ADefFuncData>,
    modules: Vec<AModuleData>,

    /// 定義箇所候補を解決する前のシンボルの個数。
    base_symbol_len: usize,
}

pub(crate) fn analyze(root: &PRoot) -> AAnalysis {
    let mut ax = Ax::new();
    ax.eof_loc = root.eof.behind();

    for stmt in &root.stmts {
        on_stmt(stmt, &mut ax);
    }

    let base_symbol_len = ax.symbols.len();

    AAnalysis {
        symbols: ax.symbols,
        base_symbol_len,
        def_candidates: ax.def_candidates,
        use_candidates: ax.use_candidates,
        deffuncs: ax.deffuncs,
        modules: ax.modules,
    }
}

fn do_extend_public_env(
    doc: ADoc,
    symbols: &[ASymbolData],
    global_env: &mut HashMap<RcStr, AWsSymbol>,
    toplevel_env: &mut HashMap<RcStr, AWsSymbol>,
) {
    for (i, symbol_data) in symbols.iter().enumerate() {
        let env = match symbol_data.scope {
            AScope::Global => &mut *global_env,
            AScope::Local(scope) if scope.is_outside_module() => &mut *toplevel_env,
            AScope::Local(_) => continue,
        };

        let symbol = ASymbol::new(i);
        env.insert(symbol_data.name.clone(), AWsSymbol { doc, symbol });
    }
}

fn do_collect_explicit_def_sites(
    doc: ADoc,
    symbols: &[ASymbolData],
    def_sites: &mut Vec<(AWsSymbol, ALoc)>,
) {
    for (i, symbol) in symbols.iter().enumerate() {
        let symbol_id = ASymbol::new(i);
        let ws_symbol = AWsSymbol {
            doc,
            symbol: symbol_id,
        };
        def_sites.extend(symbol.def_sites.iter().map(|&loc| (ws_symbol, loc)));
    }
}

fn do_resolve_symbol_def_candidates(
    doc: ADoc,
    def_candidates: &[ADefCandidateData],
    public_env: &APublicEnv,
    symbols: &mut Vec<ASymbolData>,
    def_sites: &mut Vec<(AWsSymbol, ALoc)>,
    use_sites: &mut Vec<(AWsSymbol, ALoc)>,
) {
    for candidate in def_candidates {
        match public_env.resolve(&candidate.name, candidate.scope.is_outside_module()) {
            None => {}
            Some(ws_symbol) => {
                use_sites.push((ws_symbol, candidate.loc));
                continue;
            }
        }

        // FIXME: name, definer のトークンへの参照がほしい
        let token = PToken {
            leading: [].into(),
            body: TokenData {
                kind: TokenKind::Ident,
                text: candidate.name.clone(),
                loc: candidate.loc.clone(),
            }
            .into(),
            trailing: [].into(),
        };

        // FIXME: モジュール内のシンボルならモジュールの環境にインポートする
        // 登録済みのシンボルなら同じシンボル ID をつける
        let symbol = add_symbol(
            ASymbolKind::StaticVar,
            &token,
            &token,
            AScope::Local(candidate.scope),
            symbols,
        );

        let ws_symbol = AWsSymbol { doc, symbol };
        def_sites.push((ws_symbol, candidate.loc));
    }
}

fn do_resolve_symbol_use_candidates(
    use_candidates: &[AUseCandidateData],
    public_env: &APublicEnv,
    use_sites: &mut Vec<(AWsSymbol, ALoc)>,
) {
    // eprintln!("use_candidates={:?}", use_candidates);

    for candidate in use_candidates {
        // FIXME: globalの前にモジュールの環境から探す

        let ws_symbol =
            match public_env.resolve(&candidate.name, candidate.scope.is_outside_module()) {
                None => {
                    // 未解決シンボルとして定義する
                    continue;
                }
                Some(it) => it,
            };

        use_sites.push((ws_symbol, candidate.loc));
    }
}

impl AAnalysis {
    pub(crate) fn symbol_name(&self, symbol: ASymbol) -> Option<&str> {
        let symbol = self.symbols.get(symbol.get())?;
        Some(&symbol.name)
    }

    pub(crate) fn invalidate_previous_workspace_analysis(&mut self) {
        self.symbols.drain(self.base_symbol_len..);
    }

    pub(crate) fn extend_public_env(
        &self,
        doc: ADoc,
        global_env: &mut HashMap<RcStr, AWsSymbol>,
        toplevel_env: &mut HashMap<RcStr, AWsSymbol>,
    ) {
        do_extend_public_env(doc, &self.symbols, global_env, toplevel_env);
    }

    pub(crate) fn collect_explicit_def_sites(
        &mut self,
        doc: ADoc,
        def_sites: &mut Vec<(AWsSymbol, ALoc)>,
    ) {
        do_collect_explicit_def_sites(doc, &self.symbols, def_sites);
    }

    pub(crate) fn resolve_symbol_def_candidates(
        &mut self,
        doc: ADoc,
        public_env: &APublicEnv,
        def_sites: &mut Vec<(AWsSymbol, ALoc)>,
        use_sites: &mut Vec<(AWsSymbol, ALoc)>,
    ) {
        do_resolve_symbol_def_candidates(
            doc,
            &self.def_candidates,
            public_env,
            &mut self.symbols,
            def_sites,
            use_sites,
        );
    }

    pub(crate) fn resolve_symbol_use_candidates(
        &mut self,
        public_env: &APublicEnv,
        use_sites: &mut Vec<(AWsSymbol, ALoc)>,
    ) {
        do_resolve_symbol_use_candidates(&self.use_candidates, public_env, use_sites);
    }

    pub(crate) fn resolve_scope_at(&self, pos: APos) -> ALocalScope {
        let module_opt: Option<AModule> = self
            .modules
            .iter()
            .position(|m| m.content_loc.range.is_touched(pos))
            .map(AModule::new);

        let deffunc_opt: Option<ADefFunc> = self
            .deffuncs
            .iter()
            .position(|d| d.content_loc.range.is_touched(pos))
            .map(ADefFunc::new);

        ALocalScope {
            module_opt,
            deffunc_opt,
        }
    }

    pub(crate) fn collect_local_completion_items<'a>(
        &'a self,
        current_scope: ALocalScope,
        completion_items: &mut Vec<ACompletionItem<'a>>,
    ) {
        for s in &self.symbols {
            match s.scope {
                AScope::Local(scope) if scope.is_visible_to(current_scope) => {
                    completion_items.push(ACompletionItem::Symbol(s));
                }
                AScope::Global | AScope::Local(_) => continue,
            }
        }
    }

    pub(crate) fn collect_global_completion_items<'a>(
        &'a self,
        completion_items: &mut Vec<ACompletionItem<'a>>,
    ) {
        for s in &self.symbols {
            match s.scope {
                AScope::Global => {
                    completion_items.push(ACompletionItem::Symbol(s));
                }
                AScope::Local(_) => continue,
            }
        }
    }
}

pub(crate) enum ACompletionItem<'a> {
    Symbol(&'a ASymbolData),
}
