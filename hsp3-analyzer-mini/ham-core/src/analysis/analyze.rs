use super::{
    a_scope::{ADefFunc, ADefFuncData, ALocalScope, AModule, AModuleData},
    a_symbol::{ASymbolData, AWsSymbol},
    comment::calculate_details,
    integrate::{AEnv, APublicEnv},
    ADoc, ALoc, APos, AScope, ASymbol, ASymbolDetails, ASymbolKind,
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

    fn current_deffunc_scope(&self) -> ALocalScope {
        ALocalScope {
            deffunc_opt: self.deffunc_opt,
            module_opt: self.module_opt,
        }
    }

    fn current_module_scope(&self) -> ALocalScope {
        ALocalScope {
            deffunc_opt: None,
            module_opt: self.module_opt,
        }
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
            PPrivacy::Local => AScope::Local(self.current_module_scope()),
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
        scope: ax.current_deffunc_scope(),
    });
}

fn on_symbol_use(name: &PToken, kind: AUseCandidateKind, ax: &mut Ax) {
    ax.use_candidates.push(AUseCandidateData {
        kind,
        name: name.body.text.clone(),
        loc: name.body.loc.clone(),
        scope: ax.current_deffunc_scope(),
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
            ctype_opt,
            name_opt,
            ..
        }) => {
            if let Some(name) = name_opt {
                let privacy = get_privacy_or_local(privacy_opt);
                let ctype = ctype_opt.is_some();
                ax.add_symbol(ASymbolKind::Macro { ctype }, name, privacy, hash);
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
                ax.add_symbol(ASymbolKind::Enum, name, privacy, hash);
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

            let kind = match *kind {
                PDefFuncKind::DefFunc => ASymbolKind::DefFunc,
                PDefFuncKind::DefCFunc => ASymbolKind::DefCFunc,
                PDefFuncKind::ModInit | PDefFuncKind::ModTerm | PDefFuncKind::ModFunc => {
                    ASymbolKind::ModFunc
                }
                PDefFuncKind::ModCFunc => ASymbolKind::ModCFunc,
            };

            if let Some(name) = name_opt {
                ax.deffuncs[deffunc.get()].name_opt = Some(name.body.text.clone());

                if onexit_opt.is_none() {
                    let privacy = match privacy_opt {
                        Some((privacy, _)) => *privacy,
                        None => PPrivacy::Global,
                    };
                    ax.add_symbol(kind, name, privacy, hash);
                }
            }

            let parent_deffunc = replace(&mut ax.deffunc_opt, Some(deffunc));

            for param in params {
                if let Some(name) = &param.name_opt {
                    add_symbol(
                        ASymbolKind::Param(param.param_ty_opt.as_ref().map(|&(t, _)| t)),
                        name,
                        hash,
                        AScope::Local(ax.current_deffunc_scope()),
                        &mut ax.symbols,
                    );
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
                    ax.add_symbol(ASymbolKind::LibFunc, name, privacy, hash);
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
                ax.add_symbol(ASymbolKind::ComInterface, name, privacy, hash);
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
                ax.add_symbol(ASymbolKind::ComFunc, name, privacy, hash);
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
                ax.add_symbol(ASymbolKind::PluginCmd, name, privacy, hash);
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

    /// モジュールやdeffunc内部の環境
    local_env: HashMap<ALocalScope, AEnv>,
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
        local_env: HashMap::new(),
    }
}

fn do_extend_public_env(doc: ADoc, symbols: &[ASymbolData], public_env: &mut APublicEnv) {
    for (i, symbol_data) in symbols.iter().enumerate() {
        let env = match symbol_data.scope {
            AScope::Global => &mut public_env.global,
            AScope::Local(scope) if scope.is_outside_module() && !symbol_data.kind.is_param() => {
                &mut public_env.toplevel
            }
            AScope::Local(_) => continue,
        };

        let symbol = ASymbol::new(i);
        env.insert(symbol_data.name.clone(), AWsSymbol { doc, symbol });
    }
}

fn do_collect_explicit_def_sites(
    doc: ADoc,
    symbols: &[ASymbolData],
    local_env: &mut HashMap<ALocalScope, AEnv>,
    def_sites: &mut Vec<(AWsSymbol, ALoc)>,
) {
    for (i, symbol) in symbols.iter().enumerate() {
        let ws_symbol = AWsSymbol {
            doc,
            symbol: ASymbol::new(i),
        };

        match symbol.scope {
            AScope::Local(scope) if !scope.is_outside_module() || symbol.kind.is_param() => {
                local_env
                    .entry(scope)
                    .or_default()
                    .insert(symbol.name.clone(), ws_symbol);
            }
            AScope::Local(_) | AScope::Global => {
                // すでにpublic_envに入ってる。
            }
        }

        def_sites.extend(symbol.def_sites.iter().map(|&loc| (ws_symbol, loc)));
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

fn do_resolve_symbol_def_candidates(
    doc: ADoc,
    def_candidates: &[ADefCandidateData],
    public_env: &mut APublicEnv,
    local_env: &mut HashMap<ALocalScope, AEnv>,
    symbols: &mut Vec<ASymbolData>,
    def_sites: &mut Vec<(AWsSymbol, ALoc)>,
    use_sites: &mut Vec<(AWsSymbol, ALoc)>,
) {
    for candidate in def_candidates {
        match resolve_candidate(&candidate.name, candidate.scope, public_env, local_env) {
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
        let defined_scope = ALocalScope {
            deffunc_opt: None,
            ..candidate.scope
        };
        let symbol = add_symbol(
            ASymbolKind::StaticVar,
            &token,
            &token,
            AScope::Local(defined_scope),
            symbols,
        );
        let ws_symbol = AWsSymbol { doc, symbol };

        if !candidate.scope.is_outside_module() {
            local_env
                .entry(defined_scope)
                .or_default()
                .insert(candidate.name.clone(), ws_symbol);
        } else {
            public_env
                .toplevel
                .insert(candidate.name.clone(), ws_symbol);
        }

        def_sites.push((ws_symbol, candidate.loc));
    }
}

fn do_resolve_symbol_use_candidates(
    doc: ADoc,
    use_candidates: &[AUseCandidateData],
    public_env: &APublicEnv,
    local_env: &mut HashMap<ALocalScope, AEnv>,
    symbols: &mut Vec<ASymbolData>,
    use_sites: &mut Vec<(AWsSymbol, ALoc)>,
) {
    // eprintln!("use_candidates={:?}", use_candidates);

    for candidate in use_candidates {
        match resolve_candidate(&candidate.name, candidate.scope, public_env, local_env) {
            None => {
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
                let defined_scope = ALocalScope {
                    deffunc_opt: None,
                    ..candidate.scope
                };
                let symbol = add_symbol(
                    ASymbolKind::Unresolved,
                    &token,
                    &token,
                    AScope::Local(defined_scope),
                    symbols,
                );
                let ws_symbol = AWsSymbol { doc, symbol };
                local_env
                    .entry(defined_scope)
                    .or_default()
                    .insert(candidate.name.clone(), ws_symbol);
                use_sites.push((ws_symbol, candidate.loc));
            }
            Some(ws_symbol) => {
                use_sites.push((ws_symbol, candidate.loc));
            }
        }
    }
}

impl AAnalysis {
    #[allow(unused)]
    pub(crate) fn symbol_name(&self, symbol: ASymbol) -> Option<&str> {
        let symbol = self.symbols.get(symbol.get())?;
        Some(&symbol.name)
    }

    pub(crate) fn get_symbol_details(
        &self,
        symbol: ASymbol,
    ) -> Option<(RcStr, &'static str, ASymbolDetails)> {
        let symbol_data = self.symbols.get(symbol.get())?;

        Some((
            symbol_data.name.clone(),
            symbol_data.kind.as_str(),
            calculate_details(&symbol_data.comments),
        ))
    }

    pub(crate) fn invalidate_previous_workspace_analysis(&mut self) {
        self.symbols.drain(self.base_symbol_len..);
        self.local_env.clear();
    }

    pub(crate) fn extend_public_env(&self, doc: ADoc, public_env: &mut APublicEnv) {
        do_extend_public_env(doc, &self.symbols, public_env);
    }

    pub(crate) fn collect_explicit_def_sites(
        &mut self,
        doc: ADoc,
        def_sites: &mut Vec<(AWsSymbol, ALoc)>,
    ) {
        do_collect_explicit_def_sites(doc, &self.symbols, &mut self.local_env, def_sites);
    }

    pub(crate) fn resolve_symbol_def_candidates(
        &mut self,
        doc: ADoc,
        public_env: &mut APublicEnv,
        def_sites: &mut Vec<(AWsSymbol, ALoc)>,
        use_sites: &mut Vec<(AWsSymbol, ALoc)>,
    ) {
        do_resolve_symbol_def_candidates(
            doc,
            &self.def_candidates,
            public_env,
            &mut self.local_env,
            &mut self.symbols,
            def_sites,
            use_sites,
        );
    }

    pub(crate) fn resolve_symbol_use_candidates(
        &mut self,
        doc: ADoc,
        public_env: &APublicEnv,
        use_sites: &mut Vec<(AWsSymbol, ALoc)>,
    ) {
        do_resolve_symbol_use_candidates(
            doc,
            &self.use_candidates,
            public_env,
            &mut self.local_env,
            &mut self.symbols,
            use_sites,
        );
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
