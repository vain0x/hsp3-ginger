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
    def_sites: HashMap<Symbol, Vec<SyntaxNode>>,
    closing_sites: HashMap<Symbol, Vec<SyntaxNode>>,
}

impl Symbols {
    fn fresh_symbol(&mut self, kind: SymbolKind) -> Symbol {
        let symbol = Symbol(self.symbols.len());
        self.kinds.insert(symbol.clone(), kind);
        self.declared.insert(symbol.clone());
        self.symbols.push(symbol.clone());
        symbol
    }

    fn kind(&self, symbol: &Symbol) -> SymbolKind {
        *self.kinds.get(symbol).unwrap()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    fn do_define(&mut self, symbol: &Symbol) {
        self.declared.remove(symbol);
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

    pub(crate) fn fresh_deffunc(&mut self, deffunc_stmt: ADeffuncPp) -> Symbol {
        let symbol = self.fresh_symbol(SymbolKind::Deffunc);
        self.add_def_site(&symbol, deffunc_stmt.syntax().clone());
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
        self.add_def_site(&symbol, module_stmt.syntax().clone());
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
