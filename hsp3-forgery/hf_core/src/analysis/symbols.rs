use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SymbolKind {
    StaticVar,
    Label,
    Deffunc,
    Param,
    Module,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Symbol(usize);

#[derive(Default)]
pub(crate) struct Symbols {
    symbols: Vec<Symbol>,
    kinds: HashMap<Symbol, SymbolKind>,
    declared: HashSet<Symbol>,
    unqualified_names: HashMap<Symbol, String>,
    def_sites: HashMap<Symbol, Vec<SyntaxNode>>,
    closing_sites: HashMap<Symbol, Vec<SyntaxNode>>,
    params: HashMap<Symbol, Vec<Symbol>>,
    param_nodes: HashMap<Symbol, AParam>,
}

impl Symbols {
    fn fresh_symbol(&mut self, kind: SymbolKind) -> Symbol {
        let symbol = Symbol(self.symbols.len());
        self.kinds.insert(symbol.clone(), kind);
        self.declared.insert(symbol.clone());
        self.symbols.push(symbol.clone());
        symbol
    }

    pub(crate) fn kind(&self, symbol: &Symbol) -> SymbolKind {
        *self.kinds.get(symbol).unwrap()
    }

    pub(crate) fn unqualified_name(&self, symbol: &Symbol) -> Option<&str> {
        self.unqualified_names.get(symbol).map(|s| s.as_str())
    }

    pub(crate) fn def_sites(&self, symbol: &Symbol) -> impl Iterator<Item = &SyntaxNode> {
        self.def_sites.get(symbol).into_iter().flatten()
    }

    pub(crate) fn params<'a>(&'a self, symbol: &Symbol) -> impl Iterator<Item = Symbol> + 'a {
        self.params.get(symbol).into_iter().flatten().cloned()
    }

    pub(crate) fn param_node(&self, symbol: &Symbol) -> &AParam {
        assert_eq!(self.kind(symbol), SymbolKind::Param);

        self.param_nodes.get(symbol).unwrap()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    fn do_define(&mut self, symbol: &Symbol) {
        self.declared.remove(symbol);
    }

    fn add_unqualified_name(&mut self, symbol: &Symbol, name: String) {
        self.unqualified_names.insert(symbol.clone(), name);
    }

    fn add_def_site(&mut self, symbol: &Symbol, def_site: SyntaxNode) {
        self.def_sites
            .entry(symbol.clone())
            .or_default()
            .push(def_site);
    }

    fn add_closing_site(&mut self, symbol: &Symbol, closing_site: SyntaxNode) {
        self.closing_sites
            .entry(symbol.clone())
            .or_default()
            .push(closing_site);
    }

    pub(crate) fn add_static_var(&mut self, name: &AName) -> Symbol {
        self.fresh_symbol(SymbolKind::StaticVar)
    }

    fn add_param(&mut self, deffunc: Symbol, param: Symbol) {
        assert_eq!(self.kind(&deffunc), SymbolKind::Deffunc);
        assert_eq!(self.kind(&param), SymbolKind::Param);

        self.params.entry(deffunc).or_default().push(param);
    }

    pub(crate) fn define_fresh_param(&mut self, param: AParam, enclosing_deffunc: Option<Symbol>) {
        let symbol = self.fresh_symbol(SymbolKind::Param);

        if let Some(name) = param.name() {
            self.add_unqualified_name(&symbol, name.unqualified_name());
            self.add_def_site(&symbol, name.syntax().clone());
        }

        self.param_nodes.insert(symbol.clone(), param);

        if let Some(deffunc) = enclosing_deffunc {
            self.add_param(deffunc, symbol);
        }
    }

    pub(crate) fn fresh_deffunc(&mut self, deffunc_stmt: ADeffuncPp) -> Symbol {
        let symbol = self.fresh_symbol(SymbolKind::Deffunc);

        if let Some(name) = deffunc_stmt.name() {
            self.add_unqualified_name(&symbol, name.unqualified_name());
            self.add_def_site(&symbol, name.syntax().clone());
        }

        symbol
    }

    pub(crate) fn define_deffunc(&mut self, symbol: &Symbol, closing_site_opt: Option<SyntaxNode>) {
        self.do_define(symbol);

        if let Some(closing_site) = closing_site_opt {
            self.add_closing_site(symbol, closing_site);
        }
    }

    pub(crate) fn fresh_module(&mut self, module_stmt: AModulePp) -> Symbol {
        let symbol = self.fresh_symbol(SymbolKind::Module);

        if let Some(module_name) = module_stmt.name() {
            self.add_unqualified_name(&symbol, module_name.to_string());
            self.add_def_site(&symbol, module_name.syntax().clone());
        }

        symbol
    }

    pub(crate) fn define_module(&mut self, symbol: &Symbol, closing_site_opt: Option<SyntaxNode>) {
        assert_eq!(self.kind(symbol), SymbolKind::Module);

        self.do_define(symbol);

        if let Some(closing_site) = closing_site_opt {
            self.add_closing_site(symbol, closing_site);
        }
    }
}
